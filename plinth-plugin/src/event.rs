use std::marker::PhantomData;


use plinth_core::signals::{signal::SignalMut, slice::SignalSliceMut};

use crate::parameters::ParameterId;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Event {
    // Note events
    NoteOn {
        channel: i16,
        key: i16,
        velocity: f64,
    },

    NoteOff {
        channel: i16,
        key: i16,
        velocity: f64,
    },

    // Parameter events
    StartParameterChange {
        id: ParameterId,
    },

    EndParameterChange {
        id: ParameterId,
    },

    ParameterValue {
        sample_offset: usize,
        id: ParameterId,
        value: f64,
    },

    ParameterModulation {
        sample_offset: usize,
        id: ParameterId,
        amount: f64,
    },
}

impl Event {
    pub fn split_signal_at_events<'signal, I, S>(signal: &'signal mut S, events: I) -> SignalSplitter<'signal, I, S>
    where
        I: Iterator<Item = Event>,
        S: SignalMut,
    {
        SignalSplitter::new(signal, events)
    }

    pub fn sample_offset(&self) -> usize {
        match self {
            Event::ParameterValue { sample_offset, .. } => *sample_offset,
            Event::ParameterModulation { sample_offset, .. } => *sample_offset,

            _ => 0
        }
    }
}

pub struct SignalSplitter<'signal, I, S>
where
    I: Iterator<Item = Event>,
    S: SignalMut,
{
    signal: *mut S,
    events: I,
    offset: usize,
    
    _phantom_lifetime: PhantomData<&'signal S>,
}

impl<'signal, I, S> SignalSplitter<'signal, I, S>
where
    I: Iterator<Item = Event>,
    S: SignalMut,
{
    pub fn new(signal: &'signal mut S, events: I) -> Self {
        Self {
            signal,
            events,
            offset: 0,

            _phantom_lifetime: PhantomData,
        }
    }
}

impl<'signal, I, S> Iterator for SignalSplitter<'signal, I, S>
where
    I: Iterator<Item = Event>,
    S: SignalMut,
{
    type Item = (SignalSliceMut<'signal, S>, Option<Event>);

    fn next(&mut self) -> Option<Self::Item> {
        let signal = unsafe { &mut *self.signal };

        loop {
            let Some(next_event) = self.events.next() else {
                if self.offset < signal.len() {
                    let signal_len = signal.len();
                    let signal_slice = signal.slice_mut(self.offset..);
                    self.offset = signal_len;
    
                    return Some((signal_slice, None));
                } else {
                    return None;
                }
            };
    
            match next_event {
                Event::ParameterValue { sample_offset, .. } |
                Event::ParameterModulation { sample_offset, .. } => {
                    let sample_offset = usize::min(sample_offset, signal.len());

                    let result = (signal.slice_mut(self.offset..sample_offset), Some(next_event));
                    self.offset = sample_offset;
                    return Some(result);
                },
    
                _ => { continue; },
            }    
        }
    }
}
