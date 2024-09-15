use super::{frame::{Frame, FrameMut}, frame_iterator::{FrameIterator, FrameIteratorMut}, signal::{Signal, SignalMut}};

pub struct SignalFrame<'signal, S: Signal + ?Sized> {
    signal: &'signal S,
    frame_index: usize,
}

impl<S: Signal> SignalFrame<'_, S> {
    pub fn new(signal: &S, frame_index: usize) -> SignalFrame<'_, S> {
        SignalFrame {
            signal,
            frame_index,
        }
    }
}

impl<'frame, S> Frame<'frame> for SignalFrame<'frame, S>
where
    S: Signal,
{
    type Iterator = FrameIterator<'frame, S>;

    fn channels(&self) -> usize {
        self.signal.channels()
    }

    fn channel(&self, index: usize) -> &f32 {
        &self.signal.channel(index)[self.frame_index]
    }

    fn iter(&self) -> FrameIterator<'frame, S> {
        FrameIterator::new(self.signal, self.frame_index)
    }
}

pub struct SignalFrameMut<'signal, S: SignalMut + ?Sized> {
    signal: &'signal mut S,
    frame_index: usize,
}

impl<S: SignalMut> SignalFrameMut<'_, S> {
    pub fn new(signal: &mut S, frame_index: usize) -> SignalFrameMut<'_, S> {
        SignalFrameMut {
            signal,
            frame_index,
        }
    }
}

impl<'frame, S: SignalMut + 'frame> Frame<'frame> for SignalFrameMut<'_, S> {
    type Iterator = FrameIterator<'frame, S>;

    fn channels(&self) -> usize {
        self.signal.channels()
    }

    fn channel(&self, index: usize) -> &f32 {
        &self.signal.channel(index)[self.frame_index]
    }

    fn iter(&'frame self) -> FrameIterator<'frame, S> {
        FrameIterator::new(self.signal, self.frame_index)
    }
}

impl<'frame, S: SignalMut + 'frame> FrameMut<'frame> for SignalFrameMut<'_, S> {
    type IteratorMut = FrameIteratorMut<'frame, S>;

    fn channel_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.signal.channel_mut(index)[self.frame_index]
    }

    fn iter_mut(&'frame mut self) -> FrameIteratorMut<'frame, S> {
        FrameIteratorMut::new(self.signal, self.frame_index)
    }
}
