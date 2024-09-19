use std::{ffi::{c_char, c_void, CStr}, iter::zip, mem::size_of, ptr::{null, null_mut}, sync::mpsc};

use atomic_refcell::AtomicRefCell;
use clap_sys::{events::{clap_event_header, clap_event_param_gesture, clap_event_param_value, clap_output_events, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_IS_LIVE, CLAP_EVENT_PARAM_GESTURE_BEGIN, CLAP_EVENT_PARAM_GESTURE_END, CLAP_EVENT_PARAM_VALUE}, ext::{audio_ports::{clap_plugin_audio_ports, CLAP_EXT_AUDIO_PORTS}, gui::{clap_plugin_gui, CLAP_EXT_GUI}, latency::{clap_plugin_latency, CLAP_EXT_LATENCY}, note_ports::{clap_plugin_note_ports, CLAP_EXT_NOTE_PORTS}, params::{clap_host_params, clap_plugin_params, CLAP_EXT_PARAMS}, render::{clap_plugin_render, CLAP_EXT_RENDER}, state::{clap_host_state, clap_plugin_state, CLAP_EXT_STATE}, tail::{clap_host_tail, clap_plugin_tail, CLAP_EXT_TAIL}, timer_support::{clap_host_timer_support, clap_plugin_timer_support, CLAP_EXT_TIMER_SUPPORT}}, host::clap_host, plugin::clap_plugin, process::{clap_process, clap_process_status, CLAP_PROCESS_CONTINUE, CLAP_PROCESS_CONTINUE_IF_NOT_QUIET, CLAP_PROCESS_ERROR, CLAP_PROCESS_TAIL}};
use log::error;
use plinth_core::signals::{ptr_signal::{PtrSignal, PtrSignalMut}, signal::SignalMut};

use crate::{clap::{event::EventIterator, transport::convert_transport}, parameters::{info::ParameterInfo, parameters::{has_duplicates, Parameters}}, Event, ProcessMode, ProcessState, Processor, ProcessorConfig};

use super::{descriptor::Descriptor, extensions::{audio_ports::AudioPorts, gui::Gui, latency::Latency, note_ports::NotePorts, params::Params, render::Render, state::State, tail::Tail, timer_support::TimerSupport}, parameters::map_parameter_value_to_clap, plugin::ClapPlugin, MAX_EVENTS};

pub struct AudioThreadState<P: ClapPlugin> {
    pub(super) processor: Option<P::Processor>,
    pub(super) tail: usize,
}

impl<P: ClapPlugin> Default for AudioThreadState<P> {
    fn default() -> Self {
        Self {
            processor: None,
            tail: 0,
        }
    }
}

#[repr(C)]
pub struct PluginInstance<P: ClapPlugin> {
    raw: clap_plugin,
    pub(super) host: *const clap_host,

    pub(super) plugin: Option<P>,
    pub(super) editor: Option<P::Editor>,
    pub(super) parameter_info: Vec<ParameterInfo>,

    pub(super) editor_scale: f64,
    sample_rate: f64,
    pub(super) timer_id: Option<u32>,
    pub(super) process_mode: ProcessMode,
    pub(super) to_plugin_event_sender: rtrb::Producer<Event>,
    pub(super) from_editor_event_sender: mpsc::Sender<Event>,
    to_plugin_event_receiver: rtrb::Consumer<Event>,
    pub(super) from_editor_event_receiver: mpsc::Receiver<Event>,
    pub(super) audio_thread_state: AtomicRefCell<AudioThreadState<P>>,

    // Host extensions
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

        let (to_plugin_event_sender, to_plugin_event_receiver) = rtrb::RingBuffer::new(MAX_EVENTS);
        let (from_editor_event_sender, from_editor_event_receiver) = mpsc::channel();

        let mut parameter_info = Vec::new();

        // Store parameter info and verify parameters
        plugin.with_parameters(|parameters| {
            assert!(
                parameters.ids().iter()
                    .copied()
                    .filter(|&id| parameters.get(id).unwrap().info().is_bypass())
                    .count() <= 1,
                "You can only define one bypass parameter"
            );

            for &id in parameters.ids() {
                let info = parameters.get(id).unwrap().info();
                parameter_info.push(info.clone());
            }
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

            plugin: Some(plugin),
            editor: None,
            parameter_info,

            editor_scale: 1.0,
            sample_rate: 0.0,
            timer_id: None,
            process_mode: Default::default(),
            to_plugin_event_sender,
            from_editor_event_sender,
            to_plugin_event_receiver,
            from_editor_event_receiver,

            audio_thread_state: Default::default(),

            host_ext_params: null(),
            host_ext_state: null(),
            host_ext_tail: null(),
            host_ext_timer_support: null(),
        }
    }

    pub(super) fn with_plugin_instance<T>(plugin: *const clap_plugin, mut f: impl FnMut(&mut PluginInstance<P>) -> T) -> T{
        let mut plugin_instance = unsafe { Box::from_raw(plugin as *mut PluginInstance<P>) };
        let result = f(&mut plugin_instance);
        Box::leak(plugin_instance);

        result
    }

    pub(super) fn send_events_to_plugin(&mut self, events: impl Iterator<Item = Event>) {
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

    pub(super) fn send_event_to_host(&self, event: &Event, out_events: *const clap_output_events) {
        let out_events = unsafe { &*out_events };

        match event {
            Event::StartParameterChange { id } => {
                let clap_event = clap_event_param_gesture {
                    header: clap_event_header {
                        size: size_of::<clap_event_param_gesture>() as _,
                        time: 0,
                        space_id: CLAP_CORE_EVENT_SPACE_ID,
                        type_: CLAP_EVENT_PARAM_GESTURE_BEGIN,
                        flags: CLAP_EVENT_IS_LIVE,
                    },
                    param_id: *id,
                };

                unsafe { (out_events.try_push.unwrap())(out_events, &clap_event as *const clap_event_param_gesture as _) };
            },

            Event::EndParameterChange { id } => {
                let clap_event = clap_event_param_gesture {
                    header: clap_event_header {
                        size: size_of::<clap_event_param_gesture>() as _,
                        time: 0,
                        space_id: CLAP_CORE_EVENT_SPACE_ID,
                        type_: CLAP_EVENT_PARAM_GESTURE_END,
                        flags: CLAP_EVENT_IS_LIVE,
                    },
                    param_id: *id,
                };

                unsafe { (out_events.try_push.unwrap())(out_events, &clap_event as *const clap_event_param_gesture as _) };                    
            },

            Event::ParameterValue { id, value, .. } => {
                let value = self.plugin.as_ref().unwrap().with_parameters(|parameters| {
                    let parameter = parameters.get(*id).unwrap();
                    map_parameter_value_to_clap(parameter.info(), *value)
                });

                let clap_event = clap_event_param_value {
                    header: clap_event_header {
                        size: size_of::<clap_event_param_value>() as _,
                        time: 0,
                        space_id: CLAP_CORE_EVENT_SPACE_ID,
                        type_: CLAP_EVENT_PARAM_VALUE,
                        flags: CLAP_EVENT_IS_LIVE,
                    },
                    param_id: *id,
                    cookie: null_mut(),
                    note_id: 0,
                    port_index: 0,
                    channel: 0,
                    key: 0,
                    value,
                };

                unsafe { (out_events.try_push.unwrap())(out_events, &clap_event as *const clap_event_param_value as _) };
            },

            _ => {},
        }
    }

    unsafe extern "C" fn init(plugin: *const clap_plugin) -> bool {
        Self::with_plugin_instance(plugin, |instance| {
            // Grab host extensions
            instance.host_ext_params = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_PARAMS.as_ptr()) as _ };
            instance.host_ext_state = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_STATE.as_ptr()) as _ };
            instance.host_ext_tail = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_TAIL.as_ptr()) as _ };
            instance.host_ext_timer_support = unsafe { ((*instance.host).get_extension.unwrap())(instance.host, CLAP_EXT_TIMER_SUPPORT.as_ptr()) as _ };
        });

        true
    }

    unsafe extern "C" fn destroy(plugin: *const clap_plugin) {
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
        Self::with_plugin_instance(plugin, |instance| {
            let config = ProcessorConfig {
                sample_rate,
                min_block_size: min_frames_count as _,
                max_block_size: max_frames_count as _,
                process_mode: instance.process_mode,
            };

            instance.sample_rate = sample_rate;

            let mut audio_thread_state = instance.audio_thread_state.borrow_mut();
            assert!(audio_thread_state.processor.is_none());
            audio_thread_state.processor = Some(instance.plugin.as_ref().unwrap().create_processor(&config));
        });

        true
    }

    unsafe extern "C" fn deactivate(plugin: *const clap_plugin) {
        Self::with_plugin_instance(plugin, |instance| {
            let mut audio_thread_state = instance.audio_thread_state.borrow_mut();
            assert!(audio_thread_state.processor.is_some());
            audio_thread_state.processor = None;
        });
    }

    unsafe extern "C" fn start_processing(_plugin: *const clap_plugin) -> bool {
        true
    }

    unsafe extern "C" fn stop_processing(_plugin: *const clap_plugin) {
    }

    unsafe extern "C" fn reset(plugin: *const clap_plugin) {
        Self::with_plugin_instance(plugin, |instance| {
            let mut audio_thread_state = instance.audio_thread_state.borrow_mut();
            let processor = audio_thread_state.processor.as_mut().unwrap();
            processor.reset();
        });
    }

    unsafe extern "C" fn process(plugin: *const clap_plugin, process: *const clap_process) -> clap_process_status {
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

        let input_buffers = std::slice::from_raw_parts(process.audio_inputs, process.audio_inputs_count as usize);
        let output_buffers = std::slice::from_raw_parts(process.audio_outputs, process.audio_outputs_count as usize);

        let input_buffer = input_buffers[0];
        assert_eq!(input_buffer.channel_count, 2);

        let output_buffer = output_buffers[0];
        assert_eq!(output_buffer.channel_count, 2);

        let input = PtrSignal::from_pointers(input_buffer.channel_count as usize, process.frames_count as usize, input_buffer.data32);
        let mut output = PtrSignalMut::from_pointers(output_buffer.channel_count as usize, process.frames_count as usize, output_buffer.data32 as _);

        let aux = if P::HAS_AUX_INPUT {
            let aux_buffer = input_buffers[1];
            assert_eq!(aux_buffer.channel_count, 2);

            Some(PtrSignal::from_pointers(aux_buffer.channel_count as usize, process.frames_count as usize, aux_buffer.data32))
        } else {
            None
        };
        
        // If processing out-of-place, copy input to output
        if zip(input.pointers().iter(), output.pointers().iter())
            .any(|(&input_ptr, &output_ptr)| input_ptr != &*output_ptr)
        {
            output.copy_from_signal(&input);
        }

        Self::with_plugin_instance(plugin, |instance| {
            let mut audio_thread_state = instance.audio_thread_state.borrow_mut();
            let processor = audio_thread_state.processor.as_mut().unwrap();           

            // Send events coming from the host to the editor
            let host_events = EventIterator::new(instance.plugin.as_ref().unwrap(), unsafe { &*process.in_events });
            let host_events: heapless::Vec<_, MAX_EVENTS> = host_events.collect();

            let editor_events = instance.from_editor_event_receiver.try_iter();
            let editor_events: heapless::Vec<_, MAX_EVENTS> = editor_events.collect();

            // Send events from editor to host
            for event in editor_events.iter() {
                instance.send_event_to_host(&event, process.out_events);
            }

            // Send a callback request so the main thread can process them
            unsafe { ((*instance.host).request_callback.unwrap())(instance.host); }

            let transport = if process.transport.is_null() {
                None
            } else {
                Some(convert_transport(unsafe { &*process.transport }, instance.sample_rate))
            };

            // Process events coming from the host and events coming from the editor
            let events = host_events.iter()
                .chain(editor_events.iter())
                .cloned();

            let result = match processor.process(&mut output, aux.as_ref(), transport, events) {
                ProcessState::Error => CLAP_PROCESS_ERROR,
                ProcessState::Normal => CLAP_PROCESS_CONTINUE_IF_NOT_QUIET,
                ProcessState::Tail(tail) => {
                    if tail != audio_thread_state.tail {
                        audio_thread_state.tail = tail;

                        // Inform host if it supports the extension
                        if !instance.host_ext_tail.is_null() {
                            ((*instance.host_ext_tail).changed.unwrap())(instance.host);
                        }
                    }

                    CLAP_PROCESS_TAIL
                },
                ProcessState::KeepAlive => CLAP_PROCESS_CONTINUE,
            };

            drop(audio_thread_state);

            // Also send events to the main thread
            instance.send_events_to_plugin(host_events.iter().cloned());

            result
        })
    }

    unsafe extern "C" fn get_extension(_plugin: *const clap_plugin, id: *const c_char) -> *const c_void {
        let id = CStr::from_ptr(id);

        if id == CLAP_EXT_AUDIO_PORTS {
            Self::EXT_AUDIO_PORTS.as_raw() as *const clap_plugin_audio_ports as _
        } else if id == CLAP_EXT_GUI {
            Self::EXT_GUI.as_raw() as *const clap_plugin_gui as _
        } else if id == CLAP_EXT_LATENCY {
            Self::EXT_LATENCY.as_raw() as *const clap_plugin_latency as _
        } else if id == CLAP_EXT_NOTE_PORTS {
            Self::EXT_NOTE_PORTS.as_raw() as *const clap_plugin_note_ports as _
        } else if id == CLAP_EXT_PARAMS {
            Self::EXT_PARAMS.as_raw() as *const clap_plugin_params as _
        } else if id == CLAP_EXT_RENDER {
            Self::EXT_RENDER.as_raw() as *const clap_plugin_render as _
        } else if id == CLAP_EXT_STATE {
            Self::EXT_STATE.as_raw() as *const clap_plugin_state as _
        } else if id == CLAP_EXT_TAIL {
            Self::EXT_TAIL.as_raw() as *const clap_plugin_tail as _
        } else if id == CLAP_EXT_TIMER_SUPPORT {
            Self::EXT_TIMER_SUPPORT.as_raw() as *const clap_plugin_timer_support as _
        } else {
            null()
        }
    }

    unsafe extern "C" fn on_main_thread(plugin: *const clap_plugin) {
        Self::with_plugin_instance(plugin, |instance| {
            instance.process_events_to_plugin();
        })        
    }
}