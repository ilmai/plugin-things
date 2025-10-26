use std::mem::transmute;

use super::{signal::{Signal, SignalMut}, signal_frame::{SignalFrame, SignalFrameMut}};

pub struct FramesIterator<'signal, S: Signal + ?Sized> {
    signal: &'signal S,
    frame_index_front: usize,
    frame_index_back: usize,
    finished: bool,
}

impl<S: Signal> FramesIterator<'_, S> {
    pub fn new(signal: &S) -> FramesIterator<'_, S> {
        let frame_index_back = if signal.is_empty() { 0 } else { signal.len() - 1 };

        FramesIterator {
            signal,
            frame_index_front: 0,
            frame_index_back,
            finished: false,
        }
    }
}

impl<'signal, S: Signal> Iterator for FramesIterator<'signal, S> {
    type Item = SignalFrame<'signal, S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let result = self.signal.frame(self.frame_index_front);
        if self.frame_index_front < self.frame_index_back {
            self.frame_index_front += 1;
        } else {
            self.finished = true;
        }

        Some(result)
    }
}

impl<'signal, S: Signal> DoubleEndedIterator for FramesIterator<'signal, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let result = self.signal.frame(self.frame_index_back);
        if self.frame_index_back > self.frame_index_front {
            self.frame_index_back -= 1;
        } else {
            self.finished = true;
        }

        Some(result)
    }
}

pub struct FramesIteratorMut<'signal, S: SignalMut + ?Sized> {
    signal: &'signal mut S,
    frame_index_front: usize,
    frame_index_back: usize,
    finished: bool,
}

impl<S: SignalMut> FramesIteratorMut<'_, S> {
    pub fn new(signal: &mut S) -> FramesIteratorMut<'_, S> {
        let frame_index_back = if signal.is_empty() { 0 } else { signal.len() - 1 };

        FramesIteratorMut {
            signal,
            frame_index_front: 0,
            frame_index_back,
            finished: false,
        }
    }
}

impl<'signal, S: SignalMut> Iterator for FramesIteratorMut<'signal, S> {
    type Item = SignalFrameMut<'signal, S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let result = self.signal.frame_mut(self.frame_index_front);
        if self.frame_index_front < self.frame_index_back {
            self.frame_index_front += 1;
        } else {
            self.finished = true;
        }

        // Re-borrow to the correct lifetime, which is safe since self.signal has the same lifetime
        let result = unsafe { transmute::<SignalFrameMut<'_, S>, SignalFrameMut<'_, S>>(result) };
        Some(result)
    }
}

impl<'signal, S: SignalMut> DoubleEndedIterator for FramesIteratorMut<'signal, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let result = self.signal.frame_mut(self.frame_index_back);
        if self.frame_index_back > self.frame_index_front {
            self.frame_index_back -= 1;
        } else {
            self.finished = true;
        }

        // Re-borrow to the correct lifetime, which is safe since self.signal has the same lifetime
        let result = unsafe { transmute::<SignalFrameMut<'_, S>, SignalFrameMut<'_, S>>(result) };
        Some(result)
    }
}
