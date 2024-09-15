use clap_sys::events::{clap_event_note, clap_event_param_mod, clap_event_param_value, clap_input_events, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_NOTE_OFF, CLAP_EVENT_NOTE_ON, CLAP_EVENT_PARAM_MOD, CLAP_EVENT_PARAM_VALUE};

use crate::{Event, Parameters};

use super::{parameters::map_parameter_value_from_clap, ClapPlugin};

pub struct EventIterator<'a, P: ClapPlugin> {
    plugin: &'a P,
    events: &'a clap_input_events,
    index: u32,
}

impl<'a, P: ClapPlugin> EventIterator<'a, P> {
    pub fn new(plugin: &'a P, events: &'a clap_input_events) -> Self {
        Self {
            plugin,
            events,
            index: 0,
        }
    }
}

impl<'a, P: ClapPlugin> Iterator for EventIterator<'a, P> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        let events_size = unsafe { (self.events.size.unwrap())(self.events) };

        loop {
            if self.index >= events_size {
                return None;
            }
    
            let header = unsafe { (self.events.get.unwrap())(self.events, self.index) };
            self.index += 1;

            if unsafe { *header }.space_id != CLAP_CORE_EVENT_SPACE_ID {
                continue;
            }

            let event = match (unsafe { *header }).type_ {
                CLAP_EVENT_NOTE_ON => {
                    let event = unsafe { &*(header as *const clap_event_note) };

                    Event::NoteOn {
                        channel: event.channel,
                        key: event.key,
                        velocity: event.velocity,
                    }
                }

                CLAP_EVENT_NOTE_OFF => {
                    let event = unsafe { &*(header as *const clap_event_note) };

                    Event::NoteOff {
                        channel: event.channel,
                        key: event.key,
                        velocity: event.velocity,
                    }
                }

                CLAP_EVENT_PARAM_VALUE => {
                    let event = unsafe { &*(header as *const clap_event_param_value) };
                    let value = self.plugin.with_parameters(|parameters| {
                        let parameter = parameters.get(event.param_id).unwrap();
                        map_parameter_value_from_clap(parameter.info(), event.value)
                    });

                    Event::ParameterValue {
                        sample_offset: event.header.time as _,
                        id: event.param_id,
                        value,
                    }
                },
    
                CLAP_EVENT_PARAM_MOD => {
                    let event = unsafe { &*(header as *const clap_event_param_mod) };
                    let amount = self.plugin.with_parameters(|parameters| {
                        let parameter = parameters.get(event.param_id).unwrap();
                        map_parameter_value_from_clap(parameter.info(), event.amount)
                    });

                    Event::ParameterModulation {
                        sample_offset: event.header.time as _,
                        id: event.param_id,
                        amount,
                    }
                },
    
                _ => {
                    continue;
                }
            };

            return Some(event);
        }
    }
}
