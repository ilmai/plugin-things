use std::ops::{RangeBounds, Range};

use crate::util::range::range_from_bounds;

use super::{signal::{Signal, SignalMut}, signal_base::{SignalBase, SignalMutBase}};

pub struct SignalSlice<'signal, S: Signal + ?Sized> {
    signal: &'signal S,
    range: Range<usize>,
}

impl<S: Signal + ?Sized> SignalSlice<'_, S> {
    pub fn new<T: RangeBounds<usize>>(signal: &S, range: T) -> SignalSlice<'_, S> {
        SignalSlice {
            signal,
            range: range_from_bounds(range, signal.len()),
        }
    }
}

impl<S: Signal + ?Sized> SignalBase for SignalSlice<'_, S> {
    fn len(&self) -> usize {
        assert!(self.range.end >= self.range.start, "Can't use reverse ranges for SignalSlice, got {}..{}", self.range.start, self.range.end);
        self.range.end - self.range.start
    }

    fn channels(&self) -> usize {
        self.signal.channels()
    }

    fn channel_ptr(&self, channel: usize) -> *const [f32] {
        let channel_ref = unsafe { &*self.signal.channel_ptr(channel) };
        &channel_ref[self.range.start..self.range.end]
    }
}

pub struct SignalSliceMut<'signal, S: SignalMut + ?Sized> {
    signal: &'signal mut S,
    range: Range<usize>,
}

impl<S: SignalMut + ?Sized> SignalSliceMut<'_, S> {
    pub fn new<T: RangeBounds<usize>>(signal: &mut S, range: T) -> SignalSliceMut<'_, S> {
        let signal_len = signal.len();

        SignalSliceMut {
            signal,
            range: range_from_bounds(range, signal_len),
        }
    }
}

impl<S: SignalMut + ?Sized> SignalBase for SignalSliceMut<'_, S> {
    fn len(&self) -> usize {
        assert!(self.range.end >= self.range.start, "Can't use reverse ranges for SignalSliceMut, got {}..{}", self.range.start, self.range.end);
        self.range.end - self.range.start
    }

    fn channels(&self) -> usize {
        self.signal.channels()
    }

    fn channel_ptr(&self, channel: usize) -> *const [f32] {
        let channel_ref = unsafe { &*self.signal.channel_ptr(channel) };
        &channel_ref[self.range.start..self.range.end]
    }
}

impl<S: SignalMut + ?Sized> SignalMutBase for SignalSliceMut<'_, S> {
    fn channel_ptr_mut(&mut self, channel: usize) -> *mut [f32] {
        let channel_ref = unsafe { &mut *self.signal.channel_ptr_mut(channel) };
        &mut channel_ref[self.range.start..self.range.end]
    }
}

#[cfg(test)]
mod tests {
    use crate::{buffers::buffer::Buffer, signals::signal::{Signal, SignalMut}};

    #[test]
    fn read_bounded() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let slice = buffer.slice(1..3);
        assert_eq!(slice.channel(0), &[2.0, 3.0]);
        assert_eq!(slice.channel(1), &[6.0, 7.0]);
    }

    #[test]
    fn read_lower_bound() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let slice = buffer.slice(2..);
        assert_eq!(slice.channel(0), &[3.0, 4.0]);
        assert_eq!(slice.channel(1), &[7.0, 8.0]);
    }

    #[test]
    fn read_upper_bound() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let slice = buffer.slice(..2);
        assert_eq!(slice.channel(0), &[1.0, 2.0]);
        assert_eq!(slice.channel(1), &[5.0, 6.0]);
    }

    #[test]
    fn read_upper_unbounded() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let slice = buffer.slice(..);
        assert_eq!(slice.channel(0), &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(slice.channel(1), &[5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn write_bounded() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let mut slice = buffer.slice_mut(1..3);
        slice.channel_mut(0).copy_from_slice(&[20.0, 30.0]);
        slice.channel_mut(1).copy_from_slice(&[60.0, 70.0]);

        assert_eq!(buffer.channel(0), &[1.0, 20.0, 30.0, 4.0]);
        assert_eq!(buffer.channel(1), &[5.0, 60.0, 70.0, 8.0]);
    }

    #[test]
    fn write_lower_bound() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let mut slice = buffer.slice_mut(2..);
        slice.channel_mut(0).copy_from_slice(&[30.0, 40.0]);
        slice.channel_mut(1).copy_from_slice(&[70.0, 80.0]);

        assert_eq!(buffer.channel(0), &[1.0, 2.0, 30.0, 40.0]);
        assert_eq!(buffer.channel(1), &[5.0, 6.0, 70.0, 80.0]);
    }

    #[test]
    fn write_upper_bound() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let mut slice = buffer.slice_mut(..2);
        slice.channel_mut(0).copy_from_slice(&[10.0, 20.0]);
        slice.channel_mut(1).copy_from_slice(&[50.0, 60.0]);

        assert_eq!(buffer.channel(0), &[10.0, 20.0, 3.0, 4.0]);
        assert_eq!(buffer.channel(1), &[50.0, 60.0, 7.0, 8.0]);
    }

    #[test]
    fn write_unbounded() {
        let mut buffer = Buffer::new(2, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);

        let mut slice = buffer.slice_mut(..);
        slice.channel_mut(0).copy_from_slice(&[10.0, 20.0, 30.0, 40.0]);
        slice.channel_mut(1).copy_from_slice(&[50.0, 60.0, 70.0, 80.0]);

        assert_eq!(buffer.channel(0), &[10.0, 20.0, 30.0, 40.0]);
        assert_eq!(buffer.channel(1), &[50.0, 60.0, 70.0, 80.0]);
    }

    #[test]
    fn copy_from_signal() {
        let mut buffer1 = Buffer::new(2, 4);
        let mut buffer2 = Buffer::new(2, 4);

        buffer2.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer2.channel_mut(1).copy_from_slice(&[4.0, 5.0, 6.0, 7.0]);

        buffer1.slice_mut(2..).copy_from_signal(&buffer2.slice(1..3));

        assert_eq!(buffer1.channel(0), &[0.0, 0.0, 2.0, 3.0]);
        assert_eq!(buffer1.channel(1), &[0.0, 0.0, 5.0, 6.0]);
    }
}
