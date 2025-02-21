use std::cmp;

use vst3::{ComRef, Steinberg::{kResultOk, Vst::{IParamValueQueueTrait, IParameterChanges, IParameterChangesTrait}}};

use crate::event::Event;

pub struct ParameterChangeIterator<'a> {
    parameter_changes: Option<ComRef<'a, IParameterChanges>>,
    offset: usize,
    index: usize,
    finished: bool,
}

impl ParameterChangeIterator<'_> {
    pub fn new(parameter_changes: *mut IParameterChanges) -> Self {
        Self {
            parameter_changes: unsafe { ComRef::from_raw(parameter_changes) },
            offset: 0,
            index: 0,
            finished: false,
        }
    }
}

impl Iterator for ParameterChangeIterator<'_> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let parameter_changes = self.parameter_changes?;

        let parameter_count = unsafe { parameter_changes.getParameterCount() };
        assert!(parameter_count >= 0);
        if parameter_count == 0 {
            return None;
        }

        let current_offset = self.offset;
        let current_index = self.index;
        let mut nth = 0;

        let event = (0..unsafe { parameter_changes.getParameterCount() })
            .flat_map(|parameter_index| {
                let Some(value_queue) = (unsafe { ComRef::from_raw(parameter_changes.getParameterData(parameter_index)) }) else {
                    panic!();
                };

                let id = unsafe { value_queue.getParameterId() };

                (0..unsafe { value_queue.getPointCount() })
                .filter_map(move |point_index| {
                    let mut offset = 0;
                    let mut value = 0.0;
                    let result = unsafe { value_queue.getPoint(point_index, &mut offset, &mut value) };
                    if result != kResultOk {
                        panic!();
                    }

                    assert!(offset >= 0);
                    let offset = offset as usize;

                    match offset.cmp(&current_offset) {
                        cmp::Ordering::Equal => {
                            if nth >= current_index {
                                Some((id, offset, value))
                            } else {
                                nth += 1;
                                None
                            }    
                        },

                        cmp::Ordering::Greater => Some((id, offset, value)),

                        cmp::Ordering::Less => None,
                    }
                })
            })
            .filter(|(_, offset, _)| *offset >= current_offset)
            .min_by_key(|(_, offset, _)| *offset);

        let Some(event) = event else {
            self.finished = true;
            return None;
        };

        let (id, offset, value) = event;

        if offset > self.offset {
            self.offset = offset;
            self.index = 0;
        } else {
            self.index += 1;
        }

        let event = Event::ParameterValue {
            sample_offset: offset,
            id,
            value,
        };

        Some(event)
    }
}
