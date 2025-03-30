use plinth_core::signals::signal::{Signal, SignalMut};

use crate::{event::Event, transport::Transport};

#[derive(Clone, Default)]
pub struct ProcessorConfig {
    pub sample_rate: f64,
    pub min_block_size: usize,
    pub max_block_size: usize,
    pub process_mode: ProcessMode,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ProcessMode {
    #[default]
    Realtime,
    Offline,
}

pub enum ProcessState {
    Error,
    Normal,
    Tail(usize),
    KeepAlive,
}

pub trait Processor: Send {
    fn reset(&mut self);
    fn process(&mut self, buffer: &mut impl SignalMut, aux: Option<&impl Signal>, transport: Option<Transport>, events: impl Iterator<Item = Event>) -> ProcessState;
    // Called when there's no audio to process
    fn process_events(&mut self, events: impl Iterator<Item = Event>);
}
