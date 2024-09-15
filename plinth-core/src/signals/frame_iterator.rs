use super::signal::{Signal, SignalMut};

pub struct FrameIterator<'signal, S: Signal> {
    signal: &'signal S,
    frame_index: usize,
    channel_index: usize,
}

impl<S: Signal> FrameIterator<'_, S> {
    pub fn new(signal: &S, frame_index: usize) -> FrameIterator<'_, S> {
        FrameIterator {
            signal,
            frame_index,
            channel_index: 0,
        }
    }
}

impl<'signal, S: Signal> Iterator for FrameIterator<'signal, S> {
    type Item = &'signal f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.channel_index >= self.signal.channels() {
            return None;
        }

        let result = self.signal.channel(self.channel_index).get(self.frame_index).unwrap();
        self.channel_index += 1;

        Some(result)
    }
}

pub struct FrameIteratorMut<'signal, S: SignalMut> {
    signal: &'signal mut S,
    frame_index: usize,
    channel_index: usize,
}

impl<S: SignalMut> FrameIteratorMut<'_, S> {
    pub fn new(signal: &mut S, frame_index: usize) -> FrameIteratorMut<'_, S> {
        FrameIteratorMut {
            signal,
            frame_index,
            channel_index: 0,
        }
    }
}

impl<'signal, S: SignalMut> Iterator for FrameIteratorMut<'signal, S> {
    type Item = &'signal mut f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.channel_index >= self.signal.channels() {
            return None;
        }

        let ptr = self.signal.channel_ptr_mut(self.channel_index) as *mut f32;
        let ptr = unsafe { ptr.add(self.frame_index) };
        let result = unsafe { &mut *ptr };

        self.channel_index += 1;

        Some(result)
    }
}
