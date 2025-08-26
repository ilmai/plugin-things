use ::std::sync::atomic::Ordering;
use std::{collections::HashMap, ffi::{c_char, c_void}, rc::Rc, sync::{atomic::AtomicBool, Arc, Mutex}};

use plinth_core::{signals::ptr_signal::{PtrSignal, PtrSignalMut}, util::ptr::{any_null, any_null_mut}};
use portable_atomic::AtomicF64;
use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};

use crate::{formats::PluginFormat, host::HostInfo, Editor, Event, ParameterId, Parameters, ProcessMode, ProcessState, Processor, ProcessorConfig, Transport};
use crate::auv3::{plugin::Auv3Plugin, Auv3Host, EventIterator, PLINTH_AUV3_MAX_STRING_LENGTH};
use crate::parameters::{self, group::ParameterGroupRef, has_duplicates};
use crate::string::copy_str_to_char8;

use super::{parameter_multiplier, parameters::CachedParameter, AURenderEvent, Auv3Reader, Auv3Writer, ParameterGroupInfo};

const MAX_EVENTS: usize = 1024 * 10;

pub struct Auv3Wrapper<P: Auv3Plugin> {
    plugin: Mutex<P>,
    processor: Option<P::Processor>,
    editor: Option<P::Editor>,

    parameter_ids: Vec<ParameterId>,
    parameter_groups: Vec<ParameterGroupRef>,
    cached_parameters: Arc<Mutex<Vec<CachedParameter>>>,
    parameter_index_from_id: HashMap<ParameterId, usize>,

    sample_rate: AtomicF64,
    tail_length_seconds: AtomicF64,

    sending_parameter_change_from_editor: Arc<AtomicBool>,

    events_to_processor_sender: rtrb::Producer<Event>,
    events_to_processor_receiver: rtrb::Consumer<Event>,
}

impl<P: Auv3Plugin> Auv3Wrapper<P> {
    pub fn new() -> Self {
        let (events_to_processor_sender, events_to_processor_receiver) = rtrb::RingBuffer::new(MAX_EVENTS);

        let host_info = HostInfo {
            name: None,
            format: PluginFormat::Auv3,
        };
        let plugin = P::new(host_info);

        let (parameter_groups, cached_parameters) = plugin.with_parameters(|parameters| {
            let parameter_groups = parameters::group::from_parameters(parameters);

            let cached_parameters: Vec<_> = parameters.ids()
                .iter()
                .map(|&id| {
                    let info = parameters.get(id).unwrap().info().clone();

                    CachedParameter {
                        id,
                        info,
                        value: 0.0,
                    }
                })
                .collect();

            (parameter_groups, cached_parameters)
        });

        let parameter_ids: Vec<_> = cached_parameters
            .iter()
            .map(|parameter| parameter.id)
            .collect();

        let parameter_index_from_id: HashMap<_, _> = parameter_ids.iter().enumerate()
            .map(|(index, &id)| (id, index))
            .collect();

        assert!(!has_duplicates(&parameter_ids));

        Self {
            plugin: plugin.into(),
            processor: None,
            editor: None,

            parameter_ids,
            parameter_groups,
            cached_parameters: Arc::new(cached_parameters.into()),
            parameter_index_from_id,

            sample_rate: Default::default(),
            tail_length_seconds: Default::default(),

            sending_parameter_change_from_editor: Default::default(),

            events_to_processor_sender,
            events_to_processor_receiver,
        }
    }

    pub fn with_wrapper<T>(wrapper: *mut c_void, mut f: impl FnMut(&mut Self) -> T) -> T {
        assert!(!wrapper.is_null());
    
        let mut wrapper = unsafe { Box::from_raw(wrapper as *mut Self) };
        let result = f(wrapper.as_mut());
        Box::leak(wrapper);
    
        result
    }

    pub fn activate(&mut self, sample_rate: f64, max_block_size: u64) {
        let processor_config = ProcessorConfig {
            sample_rate,
            min_block_size: 0,
            max_block_size: max_block_size as _,
            process_mode: ProcessMode::Realtime, // TODO
        };

        self.sample_rate.store(sample_rate, Ordering::Release);

        let mut plugin = self.plugin.lock().unwrap();
        self.processor = Some(plugin.create_processor(processor_config));
    }

    pub fn deactivate(&mut self) {
        self.processor = None;
    }

    pub fn tail_length(&self) -> f64 {
        self.tail_length_seconds.load(Ordering::Acquire)
    }

    pub fn parameter_count(&self) -> u64 {
        self.cached_parameters.lock().unwrap().len() as _
    }

    pub fn parameter_info(&self, index: usize, info: &mut super::ParameterInfo) {
        assert!(!info.name.is_null());
        assert!(!info.identifier.is_null());

        let cached_parameters = self.cached_parameters.lock().unwrap();
        let Some(parameter) = cached_parameters.get(index) else {
            return;
        };

        info.address = parameter.id as _;
        info.steps = parameter.info.steps() as _;

        let name_slice = unsafe { std::slice::from_raw_parts_mut(info.name as _, PLINTH_AUV3_MAX_STRING_LENGTH) };
        let identifier_slice = unsafe { std::slice::from_raw_parts_mut(info.identifier as _, PLINTH_AUV3_MAX_STRING_LENGTH) };
        copy_str_to_char8(parameter.info.name(), name_slice);
        copy_str_to_char8(&format!("ID{}", parameter.id), identifier_slice);

        info.parentGroupIndex = self.parameter_groups
            .iter()
            .position(|group| group.path == parameter.info.path())
            .map(|position| position as i64)
            .unwrap_or(-1);
    }

    pub fn parameter_value(&self, address: u64) -> f32 {
        let Some(index) = self.parameter_index_from_id.get(&(address as _)) else {
            return 0.0;
        };
        let cached_parameters = self.cached_parameters.lock().unwrap();

        cached_parameters.get(*index)
            .map(|parameter| parameter.value)
            .unwrap_or_default()
    }

    pub fn set_parameter_value(&mut self, address: u64, value: f32) {
        let Some(index) = self.parameter_index_from_id.get(&(address as _)) else {
            return;
        };
        let mut cached_parameters = self.cached_parameters.lock().unwrap();

        let Some(parameter) = cached_parameters.get_mut(*index) else {
            return;
        };

        parameter.value = value;

        let mut plugin = self.plugin.lock().unwrap();
        if let Some(event) = plugin.with_parameters(|parameters| {
            parameters.get(address as ParameterId).map(|parameter| Event::ParameterValue {
                sample_offset: 0,
                id: address as _,
                value: (value as f64 / parameter_multiplier(parameter.info())),
            })
        }) {
            if !self.sending_parameter_change_from_editor.load(::std::sync::atomic::Ordering::Acquire) {
                plugin.process_event(&event);
            }

            self.events_to_processor_sender.push(event).unwrap();
        }
    }

    /// # Safety
    /// 
    /// `string` must be a valid pointer to string with at least PLINTH_AUV3_MAX_STRING_LENGTH characters
    pub unsafe fn normalized_parameter_to_string(&self, address: u64, value: f32, string: *mut c_char) {
        let plugin = self.plugin.lock().unwrap();

        // TODO: Thread safety
        plugin.with_parameters(|parameters| {
            if let Some(parameter) = parameters.get(address as ParameterId) {
                let value = value as f64 / parameter_multiplier(parameter.info());
                let value_string = parameter.normalized_to_string(value);
                let string_slice = unsafe { std::slice::from_raw_parts_mut(string, PLINTH_AUV3_MAX_STRING_LENGTH) };

                copy_str_to_char8(&value_string, string_slice)
            }
        });
    }

    pub fn group_count(&self) -> u64 {
        self.parameter_groups.len() as _
    }

    pub fn group_info(&self, index: usize, info: &mut ParameterGroupInfo) {
        assert!(!info.name.is_null());
        assert!(!info.identifier.is_null());

        let group = &self.parameter_groups[index];

        let name_slice = unsafe { std::slice::from_raw_parts_mut(info.name as _, PLINTH_AUV3_MAX_STRING_LENGTH) };
        let identifier_slice = unsafe { std::slice::from_raw_parts_mut(info.identifier as _, PLINTH_AUV3_MAX_STRING_LENGTH) };
        copy_str_to_char8(&group.name, name_slice);
        copy_str_to_char8(&format!("Group{}", index), identifier_slice);

        info.parentGroupIndex = group.parent.as_ref()
            .map(|parent| self.parameter_groups.iter().position(|group| group == parent).unwrap() as i64)
            .unwrap_or(-1);
    }

    /// # Safety
    /// 
    /// `context` must be a pointer that can be passed to `read`
    pub unsafe fn load_state(
        &mut self,
        context: *mut c_void,
        read: unsafe extern "C-unwind" fn(*mut c_void, *mut u8, usize) -> usize,
    ) {
        let mut reader = Auv3Reader::new(context, read);

        let mut plugin = self.plugin.lock().unwrap();
        plugin.load_state(&mut reader).unwrap();

        // Send events to processor
        // TODO: Thread safety
        plugin.with_parameters(|parameters| {
            for &id in parameters.ids().iter() {
                let event = Event::ParameterValue {
                    sample_offset: 0,
                    id,
                    value: parameters.get(id).unwrap().normalized_value(),
                };

                self.events_to_processor_sender.push(event).unwrap();
            }
        });
    }

    /// # Safety
    /// 
    /// `context` must be a pointer that can be passed to `write`
    pub unsafe fn save_state(
        &self,
        context: *mut c_void,
        write: unsafe extern "C-unwind" fn(*mut c_void, *const u8, usize) -> usize,
    ) {
        let plugin = self.plugin.lock().unwrap();
        let mut writer = Auv3Writer::new(context, write);
        plugin.save_state(&mut writer).unwrap();
    }

    /// # Safety
    /// 
    /// `context` must be a pointer that can be passed to the callbacks
    pub unsafe fn create_editor(
        &mut self,
        context: *mut c_void,
        start_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, ParameterId),
        change_parameter_value: unsafe extern "C-unwind" fn(*mut c_void, ParameterId, f32),
        end_parameter_change: unsafe extern "C-unwind" fn(*mut c_void, ParameterId),
    ) {
        assert!(self.editor.is_none());

        let host = Auv3Host::new(
            context,
            start_parameter_change,
            change_parameter_value,
            end_parameter_change,
            self.sending_parameter_change_from_editor.clone(),
            self.cached_parameters.clone(),
            self.parameter_index_from_id.clone(),
        );

        let mut plugin = self.plugin.lock().unwrap();
        self.editor = Some(plugin.create_editor(Rc::new(host)));
    }

    /// # Safety
    /// 
    /// `parent` must be a valid pointer to a parent NSView
    pub unsafe fn open_editor(&mut self, parent: *mut c_void) {
        let raw_window_handle = AppKitWindowHandle::new(
            std::ptr::NonNull::new(parent as _).unwrap()
        );
        let parent_window_handle = RawWindowHandle::AppKit(raw_window_handle);

        self.editor.as_mut().unwrap().open(parent_window_handle);
    }

    pub fn close_editor(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.close();
        }
    }

    pub fn window_size(&self) -> (f64, f64) {
        let Some(editor) = self.editor.as_ref() else {
            return (0.0, 0.0);
        };

        editor.window_size()
    }

    pub fn set_window_size(&mut self, width: f64, height: f64) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };

        editor.set_window_size(width, height);
    }

    /// # Safety
    /// 
    /// All pointers must be valid
    /// `input`, `aux` and `output` must point to arrays with at least `channels` elements
    /// Each array must have at least `frames` elements
    #[expect(clippy::too_many_arguments)]
    pub unsafe fn process(
        &mut self,
        input: *const *const f32,
        aux: *const *const f32,
        output: *mut *mut f32,
        channels: u32,
        frames: u32,
        playing: bool,
        tempo: f64,
        position_samples: i64,
        first_event: *const AURenderEvent,
    ) {
        assert_eq!(channels, 2);

        let input = if input.is_null() || unsafe { any_null(input, channels as usize) } {
            None
        } else {
            Some(unsafe { PtrSignal::from_pointers(channels as usize, frames as usize, input) })
        };

        let mut output = if output.is_null() || unsafe { any_null_mut(output, channels as usize) } {
            None
        } else {
            Some(unsafe { PtrSignalMut::from_pointers(channels as usize, frames as usize, output) })
        };

        let aux = if aux.is_null() || unsafe { any_null(aux, channels as usize) } {
            None
        } else {
            Some(unsafe { PtrSignal::from_pointers(channels as usize, frames as usize, aux) })
        };

        let processor = self.processor.as_mut().unwrap();

        let event_count = self.events_to_processor_receiver.slots();
        if event_count > 0 {
            processor.process_events(self.events_to_processor_receiver.read_chunk(event_count).unwrap().into_iter());
        }

        let transport = Transport::new(playing, tempo, position_samples);

        if let (Some(input), Some(output)) = (input.as_ref(), output.as_mut()) {
            for ptr in input.pointers().iter() {
                assert!(!ptr.is_null());
            }
            for ptr in output.pointers().iter() {
                assert!(!ptr.is_null());
            }

            // If processing out-of-place, copy input to output
            if ::std::iter::zip(input.pointers().iter(), output.pointers().iter())
                .any(|(&input_ptr, &output_ptr)| input_ptr != unsafe { &*output_ptr })
            {
                use ::plinth_core::signals::signal::SignalMut;
                output.copy_from_signal(input);
            }
            
            let state = processor.process(
                output,
                aux.as_ref(),
                Some(transport),
                &mut EventIterator::new(first_event, &self.parameter_ids));

                let tail_length_samples = match state {
                    ProcessState::Error => {
                        log::error!("Processing error");
                        0
                    },

                    ProcessState::Normal => 0,
                    ProcessState::Tail(tail) => tail,
                    ProcessState::KeepAlive => usize::MAX,
                };

                let sample_rate = self.sample_rate.load(::std::sync::atomic::Ordering::Acquire);
                let tail_length_seconds = tail_length_samples as f64 / sample_rate;
                self.tail_length_seconds.store(tail_length_seconds, ::std::sync::atomic::Ordering::Release);
        } else {
            processor.process_events(&mut EventIterator::new(first_event, &self.parameter_ids));
        };
    }
}

impl<P: Auv3Plugin> Default for Auv3Wrapper<P> {
    fn default() -> Self {
        Self::new()
    }
}
