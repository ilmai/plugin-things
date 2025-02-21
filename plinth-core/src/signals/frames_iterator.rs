use std::mem::transmute;

use super::{signal::{Signal, SignalMut}, signal_frame::{SignalFrame, SignalFrameMut}};

pub struct FramesIterator<'signal, S: Signal + ?Sized> {
    signal: &'signal S,
    frame_index: usize,
}

impl<S: Signal> FramesIterator<'_, S> {
    pub fn new(signal: &S) -> FramesIterator<'_, S> {
        FramesIterator {
            signal,
            frame_index: 0,
        }
    }
}

impl<'signal, S: Signal> Iterator for FramesIterator<'signal, S> {
    type Item = SignalFrame<'signal, S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_index >= self.signal.len() {
            return None;
        }

        let result = self.signal.frame(self.frame_index);
        self.frame_index += 1;

        Some(result)
    }
}

pub struct FramesIteratorMut<'signal, S: SignalMut + ?Sized> {
    signal: &'signal mut S,
    frame_index: usize,
}

impl<S: SignalMut> FramesIteratorMut<'_, S> {
    pub fn new(signal: &mut S) -> FramesIteratorMut<'_, S> {
        FramesIteratorMut {
            signal,
            frame_index: 0,
        }
    }
}

impl<'signal, S: SignalMut> Iterator for FramesIteratorMut<'signal, S> {
    type Item = SignalFrameMut<'signal, S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_index >= self.signal.len() {
            return None;
        }

        let result = self.signal.frame_mut(self.frame_index);
        self.frame_index += 1;

        // Re-borrow to the correct lifetime, which is safe since self.signal has the same lifetime
        let result = unsafe { transmute::<SignalFrameMut<'_, S>, SignalFrameMut<'_, S>>(result) };
        Some(result)
    }
}
