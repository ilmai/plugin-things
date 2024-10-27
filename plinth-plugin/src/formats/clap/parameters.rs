use std::{collections::{btree_map, BTreeMap}, sync::atomic::{AtomicBool, Ordering}};

use portable_atomic::AtomicF64;

use crate::{parameters::info::ParameterInfo, Event, ParameterId, Parameters};

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
            event_info_iterator: self.parameter_event_info.iter()
        }
    }
}

pub struct ParameterEventMapIterator<'a> {
    event_info_iterator: btree_map::Iter<'a, ParameterId, ParameterEventInfo>,
}

impl<'a> Iterator for ParameterEventMapIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((&id, info)) = self.event_info_iterator.next() {
            if !info.changed.swap(false, Ordering::AcqRel) {
                continue;
            }

            return Some(Event::ParameterValue {
                sample_offset: 0,
                id,
                value: info.value.load(Ordering::Acquire),
            })
        };

        None
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
