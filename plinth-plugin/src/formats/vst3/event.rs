use std::mem;

use vst3::{ComRef, Steinberg::{kResultOk, Vst::{self, IEventList, IEventListTrait}}};

use crate::Event;

pub struct EventIterator<'a> {
    event_list: Option<ComRef<'a, IEventList>>,
    index: usize,
}

impl<'a> EventIterator<'a> {
    pub fn new(event_list: *mut IEventList) -> Self {
        Self {
            event_list: unsafe { ComRef::from_raw(event_list) },
            index: 0,
        }        
    }
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = Event;
    
    fn next(&mut self) -> Option<Self::Item> {
        let Some(event_list) = self.event_list else {
            return None;
        };

        if self.index >= unsafe { event_list.getEventCount() } as usize {
            return None;
        }

        let mut event: vst3::Steinberg::Vst::Event = unsafe { mem::zeroed() };
        let result = unsafe { event_list.getEvent(self.index as _, &mut event) };
        if result != kResultOk {
            return None;
        }

        match event.r#type as _ {
            Vst::Event_::EventTypes_::kNoteOnEvent => unsafe {
                Some(Event::NoteOn {
                    channel: event.__field0.noteOn.channel,
                    key: event.__field0.noteOn.pitch,
                    velocity: event.__field0.noteOn.velocity as _,
                })
            },

            Vst::Event_::EventTypes_::kNoteOffEvent => unsafe {
                Some(Event::NoteOff {
                    channel: event.__field0.noteOff.channel,
                    key: event.__field0.noteOff.pitch,
                    velocity: event.__field0.noteOff.velocity as _,
                })
            },

            _ => None
        }
    }
}
