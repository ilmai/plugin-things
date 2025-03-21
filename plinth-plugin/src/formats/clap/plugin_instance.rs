use std::{collections::BTreeMap, ffi::{c_char, c_void, CStr}, iter::zip, ptr::{null, null_mut}, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

use atomic_refcell::AtomicRefCell;
use clap_sys::{events::clap_input_events, ext::{audio_ports::CLAP_EXT_AUDIO_PORTS, gui::{clap_host_gui, CLAP_EXT_GUI}, latency::CLAP_EXT_LATENCY, note_ports::CLAP_EXT_NOTE_PORTS, params::{clap_host_params, CLAP_EXT_PARAMS}, render::CLAP_EXT_RENDER, state::{clap_host_state, CLAP_EXT_STATE}, tail::{clap_host_tail, CLAP_EXT_TAIL}, timer_support::{clap_host_timer_support, CLAP_EXT_TIMER_SUPPORT}}, host::clap_host, plugin::clap_plugin, process::{clap_process, clap_process_status, CLAP_PROCESS_CONTINUE, CLAP_PROCESS_CONTINUE_IF_NOT_QUIET, CLAP_PROCESS_ERROR, CLAP_PROCESS_TAIL}};
use log::error;
use plinth_core::signals::{ptr_signal::{PtrSignal, PtrSignalMut}, signal::SignalMut};
use portable_atomic::AtomicBool;
use raw_window_handle::RawWindowHandle;

use crate::{Event, ParameterId, ProcessMode, ProcessState, Processor, ProcessorConfig};
use crate::clap::{event::EventIterator, transport::convert_transport};
use crate::parameters::{info::ParameterInfo, has_duplicates, Parameters};

use super::descriptor::Descriptor;
use super::extensions::{audio_ports::AudioPorts, gui::Gui, latency::Latency, note_ports::NotePorts, params::Params, render::Render, state::State, tail::Tail, timer_support::TimerSupport};
use super::parameters::ParameterEventMap;
use super::plugin::ClapPlugin;

pub struct AudioThreadState<P: ClapPlugin> {
    // When active is true, we have a processor
    pub(super) active: AtomicBool,
    pub(super) processor: AtomicRefCell<Option<P::Processor>>,
    pub(super) tail: AtomicUsize,
}

impl<P: ClapPlugin> Default for AudioThreadState<P> {
    fn default() -> Self {
        Self {
            active: false.into(),
            processor: Default::default(),
            tail: 0.into(),
        }
    }
}

#[repr(C)]
pub struct PluginInstance<P: ClapPlugin> {
    raw: clap_plugin,
    pub(super) host: *const clap_host,
    pub(super) parent_window_handle: Option<RawWindowHandle>,

    pub(super) plugin: Option<P>,
    pub(super) editor: Option<P::Editor>,
    pub(super) editor_open: bool,
    pub(super) parameter_info: BTreeMap<ParameterId, ParameterInfo>,

    sample_rate: f64,
    pub(super) timer_id: Option<u32>,
    pub(super) process_mode: ProcessMode,

    pub(super) to_plugin_event_sender: rtrb::Producer<Event>,
    to_plugin_event_receiver: rtrb::Consumer<Event>,
    pub(super) parameter_event_map: Arc<ParameterEventMap>,

    pub(super) audio_thread_state: AudioThreadState<P>,

    // Host extensions
    pub(super) host_ext_gui: *const clap_host_gui,
    pub(super) host_ext_params: *const clap_host_params,
    pub(super) host_ext_state: *const clap_host_state,
    host_ext_tail: *const clap_host_tail,
    pub(super) host_ext_timer_support: *const clap_host_timer_support,
}

impl<P: ClapPlugin> PluginInstance<P> {
    // Extensions
    const EXT_AUDIO_PORTS: AudioPorts<P> = AudioPorts::new();
    const EXT_GUI: Gui<P> = Gui::new();
    const EXT_LATENCY: Latency<P> = Latency::new();
    const EXT_NOTE_PORTS: NotePorts<P> = NotePorts::new();
    const EXT_PARAMS: Params<P> = Params::new();
    const EXT_RENDER: Render<P> = Render::new();
    const EXT_STATE: State<P> = State::new();
    const EXT_TAIL: Tail<P> = Tail::new();
    const EXT_TIMER_SUPPORT: TimerSupport<P> = TimerSupport::new();

    pub fn new(descriptor: &Descriptor, host: *const clap_host) -> Self {
        let plugin = P::default();
        assert!(plugin.with_parameters(|parameters| !has_duplicates(parameters.ids())));

        let (to_plugin_event_sender, to_plugin_event_receiver) = rtrb::RingBuffer::new(P::EVENT_QUEUE_LEN);

        let mut parameter_info = BTreeMap::new();

        // Store parameter info and verify parameters
        let parameter_event_map = plugin.with_parameters(|parameters| {
            assert!(
                parameters.ids().iter()
                    .copied()
                    .filter(|&id| parameters.get(id).unwrap().info().is_bypass())
                    .count() <= 1,
                "You can only define one bypass parameter"
            );

            for &id in parameters.ids() {
                let info = parameters.get(id).unwrap().info();
                parameter_info.insert(id, info.clone());
            }

            Arc::new(ParameterEventMap::new(parameters))
        });

        Self {
            raw: clap_plugin {
                desc: descriptor.as_raw() as _,
                plugin_data: null_mut(),
                init: Some(Self::init),
                destroy: Some(Self::destroy),
                activate: Some(Self::activate),
                deactivate: Some(Self::deactivate),
                start_processing: Some(Self::start_processing),
                stop_processing: Some(Self::stop_processing),
                reset: Some(Self::reset),
                process: Some(Self::process),
                get_extension: Some(Self::get_extension),
                on_main_thread: Some(Self::on_main_thread),
            },
            host,
            parent_window_handle: None,

            plugin: Some(plugin),
            editor: None,
            editor_open: false,
            parameter_info,

            sample_rate: 0.0,
            timer_id: None,
            process_mode: Default::default(),

            to_plugin_event_sender,
            to_plugin_event_receiver,
            parameter_event_map,

            audio_thread_state: Default::default(),

            host_ext_gui: null(),
            host_ext_params: null(),
            host_ext_state: null(),
            host_ext_tail: null(),
            host_ext_timer_support: null(),
        }
    }

    pub(super) fn with_plugin_instance<T>(plugin: *const clap_plugin, mut f: impl FnMut(&mut PluginInstance<P>) -> T) -> T {
        assert!(!plugin.is_null());

        let mut plugin_instance = unsafe { Box::from_raw(plugin as *mut PluginInstance<P>) };
        let result = f(&mut plugin_instance);
        Box::leak(plugin_instance);

        result
    }

    pub(super) fn send_events_to_plugin(&mut self, in_events: *const clap_input_events) {
        let events = EventIterator::new(&self.parameter_info, unsafe { &*in_events });

        for event in events {
            match self.to_plugin_event_sender.push(event) {
                Ok(_) => {},
    
                Err(rtrb::PushError::Full(_)) => {
                    error!("Error sending CLAP event from host to processor, queue is full");
                    break;
                },
            }    
        }
    }

    pub(super) fn process_events_to_plugin(&mut self) {
        while let Ok(event) = self.to_plugin_event_receiver.pop() {
            self.plugin.as_mut().unwrap().process_event(&event);
        }
    }


    unsafe extern "C" fn init(plugin: *const clap_plugin) -> bool {
        log::trace!("plugin::init");

        Self::with_plugin_instance(plugin, |instance| {
            // Grab host extensions
            instance.host_ext_gui = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_GUI.as_ptr()) as _ };
            instance.host_ext_params = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_PARAMS.as_ptr()) as _ };
            instance.host_ext_state = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_STATE.as_ptr()) as _ };
            instance.host_ext_tail = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_TAIL.as_ptr()) as _ };
            instance.host_ext_timer_support = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_TIMER_SUPPORT.as_ptr()) as _ };
        });

        true
    }

    unsafe extern "C" fn destroy(plugin: *const clap_plugin) {
        log::trace!("plugin::destroy");

        Self::with_plugin_instance(plugin, |instance| {
            instance.plugin = None;
        })
    }

    unsafe extern "C" fn activate(
        plugin: *const clap_plugin,
        sample_rate: f64,
        min_frames_count: u32,
        max_frames_count: u32
    ) -> bool
    {
        log::trace!("plugin::activate");

        Self::with_plugin_instance(plugin, |instance| {
            let config = ProcessorConfig {
                sample_rate,
                min_block_size: min_frames_count as _,
                max_block_size: max_frames_count as _,
                process_mode: instance.process_mode,
            };

            instance.sample_rate = sample_rate;

            let mut processor = instance.audio_thread_state.processor.borrow_mut();
            *processor = Some(instance.plugin.as_mut().unwrap().create_processor(&config));

            instance.audio_thread_state.active.store(true, Ordering::Release);
        });

        true
    }

    unsafe extern "C" fn deactivate(plugin: *const clap_plugin) {
        log::trace!("plugin::deactivate");

        Self::with_plugin_instance(plugin, |instance| {
            *instance.audio_thread_state.processor.borrow_mut() = None;
            instance.audio_thread_state.active.store(false, Ordering::Release);
        });
    }

    unsafe extern "C" fn start_processing(_plugin: *const clap_plugin) -> bool {
        log::trace!("plugin::start_processing");

        true
    }

    unsafe extern "C" fn stop_processing(_plugin: *const clap_plugin) {
        log::trace!("plugin::stop_processing");
    }

    unsafe extern "C" fn reset(plugin: *const clap_plugin) {
        log::trace!("plugin::reset");

        Self::with_plugin_instance(plugin, |instance| {
            let mut processor = instance.audio_thread_state.processor.borrow_mut();
            if let Some(processor) = processor.as_mut() {
                processor.reset();
            }
        });
    }

    unsafe extern "C" fn process(plugin: *const clap_plugin, process: *const clap_process) -> clap_process_status {
        log::trace!("plugin::process");

        let process = unsafe { &*process };

        if process.audio_inputs.is_null() || process.audio_outputs.is_null() {
            return CLAP_PROCESS_ERROR;
        }

        // TODO: Support other bus layouts
        if P::HAS_AUX_INPUT {
            assert_eq!(process.audio_inputs_count, 2);
        } else {
            assert_eq!(process.audio_inputs_count, 1);
        }

        assert_eq!(process.audio_outputs_count, 1);

        let input_buffers = unsafe { std::slice::from_raw_parts(process.audio_inputs, process.audio_inputs_count as usize) };
        let output_buffers = unsafe { std::slice::from_raw_parts(process.audio_outputs, process.audio_outputs_count as usize) };

        let input_buffer = input_buffers[0];
        assert_eq!(input_buffer.channel_count, 2);

        let output_buffer = output_buffers[0];
        assert_eq!(output_buffer.channel_count, 2);

        let input = unsafe { PtrSignal::from_pointers(input_buffer.channel_count as usize, process.frames_count as usize, input_buffer.data32 as _) };
        let mut output = unsafe { PtrSignalMut::from_pointers(output_buffer.channel_count as usize, process.frames_count as usize, output_buffer.data32) };

        let aux = if P::HAS_AUX_INPUT {
            let aux_buffer = input_buffers[1];
            assert_eq!(aux_buffer.channel_count, 2);

            Some(unsafe { PtrSignal::from_pointers(aux_buffer.channel_count as usize, process.frames_count as usize, aux_buffer.data32 as _) })
        } else {
            None
        };
        
        // If processing out-of-place, copy input to output
        if zip(input.pointers().iter(), output.pointers().iter())
            .any(|(&input_ptr, &output_ptr)| input_ptr != unsafe { &*output_ptr })
        {
            output.copy_from_signal(&input);
        }

        Self::with_plugin_instance(plugin, |instance| {
            let mut processor_ref = instance.audio_thread_state.processor.borrow_mut();
            let Some(processor) = processor_ref.as_mut() else {
                return CLAP_PROCESS_ERROR;
            };

            // Send events from editor to host
            let editor_events = instance.parameter_event_map.iter_and_send_to_host(&instance.parameter_info, process.out_events);

            // Send a callback request so the main thread can process them
            unsafe { ((*instance.host).request_callback.unwrap())(instance.host); }

            let transport = if process.transport.is_null() {
                None
            } else {
                Some(convert_transport(unsafe { &*process.transport }, instance.sample_rate))
            };

            // Process events coming from the host and events coming from the editor
            let host_events = EventIterator::new(&instance.parameter_info, unsafe { &*process.in_events });
            let events = host_events.chain(editor_events);

            let result = match processor.process(&mut output, aux.as_ref(), transport, events) {
                ProcessState::Error => CLAP_PROCESS_ERROR,
                ProcessState::Normal => CLAP_PROCESS_CONTINUE_IF_NOT_QUIET,
                ProcessState::Tail(tail) => {
                    if tail != instance.audio_thread_state.tail.swap(tail, Ordering::Acquire) {
                        // Inform host if it supports the extension
                        if !instance.host_ext_tail.is_null() {
                            unsafe { ((*instance.host_ext_tail).changed.unwrap())(instance.host) };
                        }
                    }

                    CLAP_PROCESS_TAIL
                },
                ProcessState::KeepAlive => CLAP_PROCESS_CONTINUE,
            };

            drop(processor_ref);

            // Also send events to the main thread
            instance.send_events_to_plugin(process.in_events);

            result
        })
    }

    unsafe extern "C" fn get_extension(_plugin: *const clap_plugin, id: *const c_char) -> *const c_void {
        log::trace!("plugin::get_extension");

        let id = unsafe { CStr::from_ptr(id) };

        if id == CLAP_EXT_AUDIO_PORTS {
            Self::EXT_AUDIO_PORTS.as_raw() as _
        } else if id == CLAP_EXT_GUI {
            Self::EXT_GUI.as_raw() as _
        } else if id == CLAP_EXT_LATENCY {
            Self::EXT_LATENCY.as_raw() as _
        } else if id == CLAP_EXT_NOTE_PORTS {
            Self::EXT_NOTE_PORTS.as_raw() as _
        } else if id == CLAP_EXT_PARAMS {
            Self::EXT_PARAMS.as_raw() as _
        } else if id == CLAP_EXT_RENDER {
            Self::EXT_RENDER.as_raw() as _
        } else if id == CLAP_EXT_STATE {
            Self::EXT_STATE.as_raw() as _
        } else if id == CLAP_EXT_TAIL {
            Self::EXT_TAIL.as_raw() as _
        } else if id == CLAP_EXT_TIMER_SUPPORT {
            Self::EXT_TIMER_SUPPORT.as_raw() as _
        } else {
            null()
        }
    }

    unsafe extern "C" fn on_main_thread(plugin: *const clap_plugin) {
        log::trace!("plugin::on_main_thread");

        Self::with_plugin_instance(plugin, |instance| {
            instance.process_events_to_plugin();
        })        
    }
}
