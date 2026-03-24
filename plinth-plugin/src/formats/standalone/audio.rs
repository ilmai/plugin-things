use std::sync::{Arc, mpsc::Receiver};

use cpal::{FromSample, Sample};
use plinth_core::{buffers::buffer::Buffer, signals::{signal::{Signal, SignalMut}, signal_base::SignalBase}};

use super::parameters::StandaloneParameterEventMap;
use super::plugin::StandalonePlugin;
use crate::{Event, Processor};

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
            self.pending_events.push(event);
        }
        for event in self.parameter_event_map.iter_events() {
            self.pending_events.push(event);
        }

        // Process audio, ensuring we don't call process with more than P::MAX_BLOCK_SIZE frames
        let mut frame_offset = 0;
        while frame_offset < frame_count {
            let chunk_size = (frame_count - frame_offset).min(P::MAX_BLOCK_SIZE);

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
