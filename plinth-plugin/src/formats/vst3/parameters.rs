use std::cmp;

use vst3::{ComRef, Steinberg::{kResultOk, Vst::{IParamValueQueueTrait, IParameterChanges, IParameterChangesTrait, ParamID, ParamValue}}};

use crate::{event::Event, ParameterId};

pub(super) fn parameter_change_to_event(id: ParamID, value: ParamValue, offset: usize, pitch_bend_parameter_ids: &[ParameterId; 16]) -> Event {
    if let Some(channel) = pitch_bend_parameter_ids.iter().position(|&pitch_bend_id| pitch_bend_id == id) {
        let semitones = (value - 0.5) * 4.0;

        Event::PitchBend {
            channel: channel as _,
            key: -1, // TODO
            note: -1, // TODO
            semitones,
        }
    } else {
        Event::ParameterValue {
            sample_offset: offset,
            id,
            value,
        }
    }
}

pub struct ParameterChangeIterator<'a> {
    parameter_changes: Option<ComRef<'a, IParameterChanges>>,
    pitch_bend_parameter_ids: [ParameterId; 16],
    offset: usize,
    index: usize,
    finished: bool,
}

impl ParameterChangeIterator<'_> {
    pub fn new(parameter_changes: *mut IParameterChanges, pitch_bend_parameter_ids: [ParameterId; 16]) -> Self {
        Self {
            parameter_changes: unsafe { ComRef::from_raw(parameter_changes) },
            pitch_bend_parameter_ids,
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

        let event = parameter_change_to_event(id, value, offset, &self.pitch_bend_parameter_ids);

        Some(event)
    }
}
