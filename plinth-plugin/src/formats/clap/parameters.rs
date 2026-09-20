use std::{collections::{btree_map, BTreeMap}, ptr::null_mut, sync::atomic::{AtomicBool, Ordering}};

use clap_sys::events::{clap_event_header, clap_event_param_gesture, clap_event_param_value, clap_output_events, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_IS_LIVE, CLAP_EVENT_PARAM_GESTURE_BEGIN, CLAP_EVENT_PARAM_GESTURE_END, CLAP_EVENT_PARAM_VALUE};
use portable_atomic::AtomicF64;

use crate::{parameters::info::ParameterInfo, Event, ParameterId, ParameterValue, Parameters};

#[derive(Default)]
pub struct ParameterEventInfo {
    value: AtomicF64,
    modulation: AtomicF64,
    change_started: AtomicBool,
    value_changed: AtomicBool,
    modulation_changed: AtomicBool,
    change_ended: AtomicBool,
}

impl Clone for ParameterEventInfo {
    fn clone(&self) -> Self {
        Self {
            value: self.value.load(Ordering::Acquire).into(),
            modulation: self.modulation.load(Ordering::Acquire).into(),
            change_started: self.change_started.load(Ordering::Acquire).into(),
            value_changed: self.value_changed.load(Ordering::Acquire).into(),
            modulation_changed: self.modulation_changed.load(Ordering::Acquire).into(),
            change_ended: self.change_ended.load(Ordering::Acquire).into(),
        }
    }
}

#[derive(Clone)]
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

    pub fn start_parameter_change(&self, id: ParameterId) {
        self.parameter_event_info(id).change_started.store(true, Ordering::Release);
    }

    pub fn change_parameter_value(&self, id: ParameterId, normalized: ParameterValue) {
        let parameter_event_info = self.parameter_event_info(id);
        parameter_event_info.value.store(normalized, Ordering::Release);
        parameter_event_info.value_changed.store(true, Ordering::Release);
    }

    pub fn change_parameter_modulation(&self, id: ParameterId, normalized: ParameterValue) {
        let parameter_event_info = self.parameter_event_info(id);
        parameter_event_info.modulation.store(normalized, Ordering::Release);
        parameter_event_info.modulation_changed.store(true, Ordering::Release);
    }

    pub fn end_parameter_change(&self, id: ParameterId) {
        self.parameter_event_info(id).change_ended.store(true, Ordering::Release);
    }

    pub fn iter_and_send<'a>(
        &'a self,
        parameter_info: &'a BTreeMap<ParameterId, ParameterInfo>,
        out_events: *const clap_output_events,
    ) -> ParameterEventMapIterator<'a>
    {
        ParameterEventMapIterator {
            event_info_iterator: self.parameter_event_info.iter(),
            parameter_info,
            out_events,

            parameter_id: Default::default(),
            start_change: false,
            end_change: false,
            value: None,
            modulation: None,
        }
    }
}

pub struct ParameterEventMapIterator<'a> {
    event_info_iterator: btree_map::Iter<'a, ParameterId, ParameterEventInfo>,
    parameter_info: &'a BTreeMap<ParameterId, ParameterInfo>,
    out_events: *const clap_output_events,

    parameter_id: ParameterId,
    start_change: bool,
    end_change: bool,
    value: Option<ParameterValue>,
    modulation: Option<ParameterValue>,
}

impl Iterator for ParameterEventMapIterator<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Parameter start/end requests don't get turned into notifications but only get sent to the host
            if std::mem::take(&mut self.start_change) && !self.out_events.is_null() {
                let out_events = unsafe { &*self.out_events };

                let clap_event = clap_event_param_gesture {
                    header: clap_event_header {
                        size: size_of::<clap_event_param_gesture>() as _,
                        time: 0,
                        space_id: CLAP_CORE_EVENT_SPACE_ID,
                        type_: CLAP_EVENT_PARAM_GESTURE_BEGIN,
                        flags: CLAP_EVENT_IS_LIVE,
                    },
                    param_id: self.parameter_id,
                };

                unsafe { (out_events.try_push.unwrap())(out_events, &clap_event as *const clap_event_param_gesture as _) };
            }

            // Send value in between start and end if any
            let event = if let Some(value) = self.value.take() {
                if !self.out_events.is_null() {
                    let out_events = unsafe { &*self.out_events };

                    let parameter_info = self.parameter_info.get(&self.parameter_id).unwrap();
                    let value = map_parameter_value_to_clap(parameter_info, value);

                    let clap_event = clap_event_param_value {
                        header: clap_event_header {
                            size: size_of::<clap_event_param_value>() as _,
                            time: 0,
                            space_id: CLAP_CORE_EVENT_SPACE_ID,
                            type_: CLAP_EVENT_PARAM_VALUE,
                            flags: CLAP_EVENT_IS_LIVE,
                        },
                        param_id: self.parameter_id,
                        cookie: null_mut(),
                        note_id: 0,
                        port_index: 0,
                        channel: 0,
                        key: 0,
                        value,
                    };

                    unsafe { (out_events.try_push.unwrap())(out_events, &clap_event as *const clap_event_param_value as _) };
                }

                Some(Event::ParameterValue {
                    sample_offset: 0,
                    id: self.parameter_id,
                    value,
                })
            } else if let Some(amount) = self.modulation.take() {
                Some(Event::ParameterModulation {
                    sample_offset: 0,
                    id: self.parameter_id,
                    amount,
                })
            } else {
                None
            };

            if std::mem::take(&mut self.end_change) && !self.out_events.is_null() {
                let out_events = unsafe { &*self.out_events };

                let clap_event = clap_event_param_gesture {
                    header: clap_event_header {
                        size: size_of::<clap_event_param_gesture>() as _,
                        time: 0,
                        space_id: CLAP_CORE_EVENT_SPACE_ID,
                        type_: CLAP_EVENT_PARAM_GESTURE_END,
                        flags: CLAP_EVENT_IS_LIVE,
                    },
                    param_id: self.parameter_id,
                };

                unsafe { (out_events.try_push.unwrap())(out_events, &clap_event as *const clap_event_param_gesture as _) };
            }

            if let Some(event) = event {
                return Some(event);
            }

            let (&id, info) = self.event_info_iterator.next()?;

            self.parameter_id = id;
            self.start_change = info.change_started.swap(false, Ordering::AcqRel);
            self.end_change = info.change_ended.swap(false, Ordering::AcqRel);

            if info.value_changed.swap(false, Ordering::AcqRel) {
                self.value = Some(info.value.load(Ordering::Acquire))
            }

            if info.modulation_changed.swap(false, Ordering::AcqRel) {
                self.modulation = Some(info.modulation.load(Ordering::Acquire))
            }
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
