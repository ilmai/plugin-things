use super::signal::{Signal, SignalMut};

pub struct ChannelsIterator<'signal, S: Signal + ?Sized> {
    signal: &'signal S,
    channel_index: usize,
}

impl<'signal, S: Signal> ChannelsIterator<'signal, S> {
    pub fn new(signal: &'signal S) -> ChannelsIterator<'signal, S> {
        ChannelsIterator {
            signal,
            channel_index: 0,
        }
    }
}

impl<S: Signal> Clone for ChannelsIterator<'_, S> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal,
            channel_index: 0,
        }
    }
}

impl<'signal, S: Signal + ?Sized> Iterator for ChannelsIterator<'signal, S> {
    type Item = &'signal [f32];

    fn next(&mut self) -> Option<Self::Item> {
        if self.channel_index < self.signal.channels() {
            let result = self.signal.channel(self.channel_index);
            self.channel_index += 1;
            Some(result)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n <= self.channel_index {
            return None
        }

        self.channel_index = n + 1;

        if n < self.signal.channels() {
            Some(self.signal.channel(n))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.signal.channels();        
        (size, Some(size))
    }
}

pub struct ChannelsIteratorMut<'signal, S: SignalMut + ?Sized> {
    signal: &'signal mut S,
    channel_index: usize,
}

impl<'signal, S: SignalMut> ChannelsIteratorMut<'signal, S> {
    pub fn new(signal: &'signal mut S) -> ChannelsIteratorMut<'signal, S> {
        ChannelsIteratorMut {
            signal,
            channel_index: 0,
        }
    }
}

impl<'signal, S: SignalMut + ?Sized> Iterator for ChannelsIteratorMut<'signal, S> {
    type Item = &'signal mut [f32];

    fn next(&mut self) -> Option<Self::Item> {
        if self.channel_index < self.signal.channels() {
            let ptr = self.signal.channel_ptr_mut(self.channel_index);
            self.channel_index += 1;
            Some(unsafe { &mut *ptr })
        } else {
            None
        }
    }

    fn count(self) -> usize {
        self.signal.channels()
    }
}
