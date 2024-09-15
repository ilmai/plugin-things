use crate::{Event, ParameterId};

use super::au_render_event::{AURenderEvent, AURenderEventType};

pub struct EventIterator<'ids> {
    next_event: *const AURenderEvent,
    parameter_ids: &'ids [ParameterId],
}

impl<'ids> EventIterator<'ids> {
    pub fn new(first_event: *const AURenderEvent, parameter_ids: &'ids [ParameterId]) -> Self {
        Self {
            next_event: first_event,
            parameter_ids,
        }
    }
}

impl<'ids> Iterator for EventIterator<'ids> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.next_event.is_null() {
            let next_event = unsafe { &*self.next_event };
            let header = unsafe { &next_event.header };
            
            self.next_event = header.next;

            match header.event_type {
                AURenderEventType::AURenderEventParameter | AURenderEventType::AURenderEventParameterRamp => {
                    let parameter_event = unsafe { &next_event.parameter };

                    // auval will use invalid parameter addresses when testing for parameter ramping so
                    // just ignore those events
                    if !self.parameter_ids.contains(&(parameter_event.parameter_address as _)) {
                        continue;
                    }

                    // We don't deal too well with time travel so restrict the range
                    // For example auval will send events with some wild values here
                    let sample_offset = i64::max(0, parameter_event.event_sample_time);

                    return Some(Event::ParameterValue {
                        sample_offset: sample_offset as _,
                        id: parameter_event.parameter_address as _,
                        value: parameter_event.value as _,
                    });
                },

                _ => {}
            }
        }

        None
    }
}
