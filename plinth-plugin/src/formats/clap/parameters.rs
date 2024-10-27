use std::{collections::{btree_map, BTreeMap}, sync::atomic::{AtomicBool, Ordering}};

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

    pub fn iter(&self) -> ParameterEventMapIterator<'_> {
        ParameterEventMapIterator {
            event_info_iterator: self.parameter_event_info.iter(),

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

    pending_parameter_id: ParameterId,
    pending_parameter_value: ParameterValue,
    pending_start_change: bool,
    pending_change: bool,
    pending_end_change: bool,
}

impl<'a> Iterator for ParameterEventMapIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if std::mem::take(&mut self.pending_start_change) {
                return Some(Event::StartParameterChange { id: self.pending_parameter_id });
            }
            if std::mem::take(&mut self.pending_change) {
                return Some(Event::ParameterValue {
                    sample_offset: 0,
                    id: self.pending_parameter_id,
                    value: self.pending_parameter_value,
                })
            }
            if std::mem::take(&mut self.pending_end_change) {
                return Some(Event::EndParameterChange { id: self.pending_parameter_id });
            }

            let Some((&id, info)) = self.event_info_iterator.next() else {
                return None;
            };

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
