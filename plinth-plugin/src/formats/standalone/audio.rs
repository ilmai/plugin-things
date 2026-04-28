use std::sync::{Arc, mpsc::Receiver};

use cpal::{FromSample, Sample};
use plinth_core::{ buffers::buffer::Buffer, signals::{ signal::{Signal, SignalMut}, signal_base::SignalBase } };

use super::parameters::StandaloneParameterEventMap;
use super::plugin::StandalonePlugin;
use crate::{Event, Processor};

/// Push events to a event list vec, printing a warning when preallocated memory exceeded.
trait EventListPush {
    type EventType;
    fn push_event(&mut self, event: Self::EventType);
}

impl EventListPush for Vec<Event> {
    type EventType = Event;
    fn push_event(&mut self, event: Event) {
        if self.len() == self.capacity() {
            log::warn!(
                "Event queue exceeded preallocated capacity of {} - allocating more. \
                Increase EVENT_QUEUE_LEN to avoid allocation on the audio thread.",
                self.capacity()
            );
            self.reserve(128);
        }
        self.push(event);
    }
}

/// Runs a plinth processor on a CPAL audio stream
pub struct AudioState<P: StandalonePlugin> {
    pub processor: P::Processor,
    pub buffer: Buffer,
    pub channels: usize,
    pub midi_receiver: Receiver<Event>,
    pub parameter_event_map: Arc<StandaloneParameterEventMap>,
    pending_events: Vec<Event>,
}

impl<P: StandalonePlugin> AudioState<P> {
    pub fn new(
        processor: P::Processor,
        channels: usize,
        midi_receiver: Receiver<Event>,
        parameter_event_map: Arc<StandaloneParameterEventMap>,
    ) -> Self {
        Self {
            processor,
            buffer: Buffer::new(channels, P::MAX_BLOCK_SIZE),
            channels,
            midi_receiver,
            parameter_event_map,
            pending_events: Vec::with_capacity(P::EVENT_QUEUE_LEN),
        }
    }

    pub fn process<T>(&mut self, data: &mut [T], channels: usize)
    where
        T: Sample + FromSample<f32>,
        f32: FromSample<T>,
    {
        let frame_count = data.len() / channels;

        // Drain MIDI events
        self.pending_events.clear();
        while let Ok(event) = self.midi_receiver.try_recv() {
            self.pending_events.push_event(event);
        }

        // Collect pending parameter change events
        for event in self.parameter_event_map.iter_events() {
            self.pending_events.push_event(event);
        }

        // Process audio, ensuring we don't call process with more than P::MAX_BLOCK_SIZE frames
        debug_assert!(
            self.buffer.capacity() == P::MAX_BLOCK_SIZE,
            "Buffer must be preallocated to avoid allocation on the audio thread"
        );

        let mut frame_offset = 0;
        while frame_offset < frame_count {
            let chunk_size = (frame_count - frame_offset).min(P::MAX_BLOCK_SIZE);

            // Truncate or extend buffer to fit the chunk
            if self.buffer.len() != chunk_size {
                self.buffer.resize(chunk_size);
            }

            // Deinterleave chunk from CPAL buffer
            for frame in 0..chunk_size {
                for ch in 0..self.channels {
                    self.buffer.channel_mut(ch)[frame] =
                        f32::from_sample(data[(frame_offset + frame) * self.channels + ch]);
                }
            }

            // Process and drain all events on first run, assuming they have no time tags
            let aux: Option<&Buffer> = None;
            self.processor
                .process(&mut self.buffer, aux, None, self.pending_events.drain(..));

            // Reinterleave chunk back into CPAL buffer
            for frame in 0..chunk_size {
                for ch in 0..self.channels {
                    data[(frame_offset + frame) * self.channels + ch] =
                        T::from_sample(self.buffer.channel(ch)[frame]);
                }
            }

            frame_offset += chunk_size;
        }
    }
}
