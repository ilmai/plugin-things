use std::{collections::{btree_map, BTreeMap}, ptr::null_mut, sync::atomic::{AtomicBool, Ordering}};

use clap_sys::events::{clap_event_header, clap_event_param_gesture, clap_event_param_value, clap_output_events, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_IS_LIVE, CLAP_EVENT_PARAM_GESTURE_BEGIN, CLAP_EVENT_PARAM_GESTURE_END, CLAP_EVENT_PARAM_VALUE};
use portable_atomic::AtomicF64;

use crate::{parameters::info::ParameterInfo, Event, ParameterId, ParameterValue, Parameters};

#[derive(Default)]
pub struct ParameterEventInfo {
    pub(super) value: AtomicF64,
    pub(super) change_started: AtomicBool,
    pub(super) changed: AtomicBool,
    pub(super) change_ended: AtomicBool,
}

pub struct ParameterEventMap {
    parameter_event_info: BTreeMap<ParameterId, ParameterEventInfo>,
}

impl ParameterEventMap {
    pub fn new(parameters: &impl Parameters) -> Self {
        let mut parameter_event_info = BTreeMap::new();

        for &id in parameters.ids() {
            parameter_event_info.insert(id, Default::default());
        }

        Self {
            parameter_event_info,
        }
    }

    pub fn parameter_event_info(&self, id: ParameterId) -> &ParameterEventInfo {
        self.parameter_event_info.get(&id).unwrap()
    }

    pub fn iter_and_send_to_host<'a>(
        &'a self,
        parameter_info: &'a BTreeMap<ParameterId, ParameterInfo>,
        out_events: *const clap_output_events,
    ) -> ParameterEventMapIterator<'a>
    {
        ParameterEventMapIterator {
            event_info_iterator: self.parameter_event_info.iter(),
            parameter_info,
            out_events,

            pending_parameter_id: Default::default(),
            pending_parameter_value: Default::default(),
            pending_start_change: false,
            pending_change: false,
            pending_end_change: false,
        }
    }
}

pub struct ParameterEventMapIterator<'a> {
    event_info_iterator: btree_map::Iter<'a, ParameterId, ParameterEventInfo>,
    parameter_info: &'a BTreeMap<ParameterId, ParameterInfo>,
    out_events: *const clap_output_events,

    pending_parameter_id: ParameterId,
    pending_parameter_value: ParameterValue,
    pending_start_change: bool,
    pending_change: bool,
    pending_end_change: bool,
}

impl ParameterEventMapIterator<'_> {
    pub fn send_event_to_host(&self, event: &Event) {
        let out_events = unsafe { &*self.out_events };

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
                let parameter_info = self.parameter_info.get(id).unwrap();
                let value = map_parameter_value_to_clap(parameter_info, *value);

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
}

impl Iterator for ParameterEventMapIterator<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let event = if std::mem::take(&mut self.pending_start_change) {
                Some(Event::StartParameterChange { id: self.pending_parameter_id })
            } else  if std::mem::take(&mut self.pending_change) {
                Some(Event::ParameterValue {
                    sample_offset: 0,
                    id: self.pending_parameter_id,
                    value: self.pending_parameter_value,
                })
            } else if std::mem::take(&mut self.pending_end_change) {
                Some(Event::EndParameterChange { id: self.pending_parameter_id })
            } else {
                None
            };

            if let Some(event) = event {
                self.send_event_to_host(&event);
                return Some(event);
            }

            let (&id, info) = self.event_info_iterator.next()?;

            self.pending_parameter_id = id;
            self.pending_parameter_value = info.value.load(Ordering::Acquire);
            self.pending_start_change = info.change_started.swap(false, Ordering::AcqRel);
            self.pending_change = info.changed.swap(false, Ordering::AcqRel);
            self.pending_end_change = info.change_ended.swap(false, Ordering::AcqRel);
        }
    }
}

pub fn map_parameter_value_to_clap(info: &ParameterInfo, value: f64) -> f64 {
    let steps = info.steps();
    if steps > 0 {
        (value * steps as f64).round()
    } else {
        value
    }
}

pub fn map_parameter_value_from_clap(info: &ParameterInfo, value: f64) -> f64 {
    let steps = info.steps();
    if steps > 0 {
        value / steps as f64
    } else {
        value
    }
}
