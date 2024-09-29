use std::{ffi::c_void, sync::{atomic::AtomicBool, Arc, Mutex}};

use portable_atomic::AtomicF64;

use crate::{auv3::plugin::Auv3Plugin, parameters::{self, group::ParameterGroupRef, parameters::has_duplicates}, Event, ParameterId, Parameters};

const MAX_EVENTS: usize = 1024 * 10;

pub struct Auv3Wrapper<P: Auv3Plugin> {
    pub plugin: Mutex<P>,
    pub processor: Option<P::Processor>,
    pub editor: Option<P::Editor>,

    pub parameter_ids: Vec<ParameterId>,
    pub parameter_groups: Vec<ParameterGroupRef>,

    pub sample_rate: AtomicF64,
    pub tail_length_seconds: AtomicF64,

    pub sending_parameter_change_from_editor: Arc<AtomicBool>,

    pub events_to_processor_sender: rtrb::Producer<Event>,
    pub events_to_processor_receiver: rtrb::Consumer<Event>,
}

impl<P: Auv3Plugin> Auv3Wrapper<P> {
    pub fn new() -> Self {
        let (events_to_processor_sender, events_to_processor_receiver) = rtrb::RingBuffer::new(MAX_EVENTS);

        let plugin = P::default();

        let parameter_ids: Vec<_> = plugin.with_parameters(|parameters| parameters.ids().into());
        assert!(!has_duplicates(&parameter_ids));

        let parameter_groups = plugin.with_parameters(|parameters| {
            parameters::group::from_parameters(parameters)
        });

        Self {
            plugin: plugin.into(),
            processor: None,
            editor: None,

            parameter_ids,
            parameter_groups,

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
}
