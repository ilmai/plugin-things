use std::collections::{btree_map, BTreeMap};
use std::sync::atomic::{AtomicBool, Ordering};

use portable_atomic::AtomicF64;

use crate::{Event, ParameterId, ParameterValue, Parameters};

#[derive(Default)]
struct ParameterEventInfo {
    value: AtomicF64,
    changed: AtomicBool,
}

pub(crate) struct StandaloneParameterEventMap {
    parameter_event_info: BTreeMap<ParameterId, ParameterEventInfo>,
}

impl StandaloneParameterEventMap {
    pub(crate) fn new(parameters: &impl Parameters) -> Self {
        let mut parameter_event_info = BTreeMap::new();

        for &id in parameters.ids() {
            parameter_event_info.insert(id, Default::default());
        }

        Self { parameter_event_info }
    }

    pub(crate) fn change_parameter_value(&self, id: ParameterId, value: ParameterValue) {
        let info = self.parameter_event_info.get(&id).unwrap();
        info.value.store(value, Ordering::Release);
        info.changed.store(true, Ordering::Release);
    }

    pub(crate) fn iter_events(&self) -> StandaloneParameterEventIterator<'_> {
        StandaloneParameterEventIterator {
            event_info_iterator: self.parameter_event_info.iter(),
        }
    }
}

pub(crate) struct StandaloneParameterEventIterator<'a> {
    event_info_iterator: btree_map::Iter<'a, ParameterId, ParameterEventInfo>,
}

impl Iterator for StandaloneParameterEventIterator<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (&id, info) = self.event_info_iterator.next()?;

            if info.changed.swap(false, Ordering::AcqRel) {
                return Some(Event::ParameterValue {
                    sample_offset: 0,
                    id,
                    value: info.value.load(Ordering::Acquire),
                });
            }
        }
    }
}
