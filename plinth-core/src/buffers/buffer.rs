use crate::signals::{signal::Signal, signal_base::{SignalBase, SignalMutBase}};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Buffer {
    samples: Vec<Vec<f32>>,
}

impl Buffer {
    pub fn new(channels: usize, length: usize) -> Self {
        assert!(channels > 0);

        Self {
            samples: vec![vec![0.0; length]; channels],
        }
    }

    pub fn with_capacity(channels: usize, capacity: usize) -> Self {
        assert!(channels > 0);

        Self {
            samples: vec![Vec::with_capacity(capacity); channels],
        }
    }

    pub fn from_signal(signal: &impl Signal) -> Self {
        let samples: Vec<_> = signal.iter_channels()
            .map(|channel| channel.to_vec())
            .collect();

        Self {
            samples,
        }
    }

    pub fn capacity(&self) -> usize {
        self.samples[0].capacity()
    }

    pub fn resize(&mut self, length: usize) {
        for channel in self.samples.iter_mut() {
            channel.resize(length, 0.0);
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        for channel in self.samples.iter_mut() {
            channel.reserve(additional);
        }
    }
}

impl From<Vec<Vec<f32>>> for Buffer {
    fn from(value: Vec<Vec<f32>>) -> Self {
        Buffer {
            samples: value,
        }
    }
}

impl SignalBase for Buffer {
    fn channels(&self) -> usize {
        self.samples.len()
    }

    fn len(&self) -> usize {
        self.samples[0].len()
    }

    fn channel_ptr(&self, channel: usize) -> *const [f32] {
        self.samples[channel].as_slice()
    }
}

impl SignalMutBase for Buffer {
    fn channel_ptr_mut(&mut self, channel: usize) -> *mut [f32] {
        self.samples[channel].as_mut_slice()
    }
}

impl PartialEq<Buffer> for Buffer {
    fn eq(&self, other: &Buffer) -> bool {
        self.samples == other.samples
    }
}

#[cfg(test)]
mod tests {
    use crate::signals::{frame::{Frame, FrameMut}, signal::{Signal, SignalMut}, signal_base::SignalBase};

    use super::Buffer;

    #[test]
    fn create() {
        let buffer = Buffer::new(1, 2);
        assert_eq!(buffer.iter_channels().count(), 1);
        assert_eq!(buffer.channel(0), &[0.0, 0.0]);
    }

    #[test]
    fn read_write_1_channel_2_samples() {
        let mut buffer = Buffer::new(1, 2);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0]);
        assert_eq!(buffer.channel(0), [1.0, 2.0]);
    }

    #[test]
    fn copy_from_signal() {
        let mut buffer1 = Buffer::new(2, 2);
        let mut buffer2 = Buffer::new(2, 2);

        buffer2.channel_mut(0).copy_from_slice(&[1.0, 2.0]);
        buffer2.channel_mut(1).copy_from_slice(&[3.0, 4.0]);

        buffer1.copy_from_signal(&buffer2);

        assert_eq!(buffer1, buffer2);
    }

    #[test]
    fn apply_wrap() {
        let mut buffer1 = Buffer::new(2, 2);
        let mut buffer2 = Buffer::new(2, 2);

        buffer2.channel_mut(0).copy_from_slice(&[1.0, 2.0]);
        buffer2.channel_mut(1).copy_from_slice(&[3.0, 4.0]);

        let buffer2_len = buffer2.len();
        buffer2.apply_wrap(1, buffer2_len, |signal, range| buffer1.slice_mut(range).copy_from_signal(signal));
        
        assert_eq!(buffer1.channel(0), &[2.0, 1.0]);
        assert_eq!(buffer1.channel(1), &[4.0, 3.0]);
    }

    #[test]
    fn apply_wrap_mut() {
        let mut buffer1 = Buffer::new(2, 2);
        let mut buffer2 = Buffer::new(2, 2);

        buffer2.channel_mut(0).copy_from_slice(&[1.0, 2.0]);
        buffer2.channel_mut(1).copy_from_slice(&[3.0, 4.0]);

        let buffer2_len = buffer2.len();
        buffer1.apply_wrap_mut(1, buffer2_len, |signal, range| signal.copy_from_signal(&buffer2.slice(range)));
        
        assert_eq!(buffer1.channel(0), &[2.0, 1.0]);
        assert_eq!(buffer1.channel(1), &[4.0, 3.0]);
    }

    #[test]
    fn truncate() {
        let mut buffer = Buffer::new(3, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        buffer.channel_mut(1).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);
        buffer.channel_mut(2).copy_from_slice(&[9.0, 10.0, 11.0, 12.0]);

        buffer.resize(2);
        assert_eq!(buffer.channel(0), [1.0, 2.0]);
        assert_eq!(buffer.channel(1), [5.0, 6.0]);
        assert_eq!(buffer.channel(2), [9.0, 10.0]);
    }

    #[test]
    fn grow() {
        let mut buffer = Buffer::new(3, 2);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0]);
        buffer.channel_mut(1).copy_from_slice(&[3.0, 4.0]);
        buffer.channel_mut(2).copy_from_slice(&[5.0, 6.0]);

        buffer.resize(4);
        assert_eq!(buffer.channel(0), [1.0, 2.0, 0.0, 0.0]);
        assert_eq!(buffer.channel(1), [3.0, 4.0, 0.0, 0.0]);
        assert_eq!(buffer.channel(2), [5.0, 6.0, 0.0, 0.0]);
    }

    #[test]
    fn iter_frames() {
        let mut buffer = Buffer::new(1, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);

        let mut iterator = buffer.iter_frames();
        assert_eq!(*iterator.next().unwrap().channel(0), 1.0);
        assert_eq!(*iterator.next().unwrap().channel(0), 2.0);
        assert_eq!(*iterator.next().unwrap().channel(0), 3.0);
        assert_eq!(*iterator.next().unwrap().channel(0), 4.0);
        assert!(iterator.next().is_none());
    }

    #[test]
    fn iter_frames_rev() {
        let mut buffer = Buffer::new(1, 4);
        buffer.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);

        let mut iterator = buffer.iter_frames().rev();
        assert_eq!(*iterator.next().unwrap().channel(0), 4.0);
        assert_eq!(*iterator.next().unwrap().channel(0), 3.0);
        assert_eq!(*iterator.next().unwrap().channel(0), 2.0);
        assert_eq!(*iterator.next().unwrap().channel(0), 1.0);
        assert!(iterator.next().is_none());
    }

    #[test]
    fn iter_frames_mut() {
        let mut buffer = Buffer::new(1, 4);
        let mut iterator = buffer.iter_frames_mut();
        *iterator.next().unwrap().channel_mut(0) = 1.0;
        *iterator.next().unwrap().channel_mut(0) = 2.0;
        *iterator.next().unwrap().channel_mut(0) = 3.0;
        *iterator.next().unwrap().channel_mut(0) = 4.0;
        assert!(iterator.next().is_none());

        assert_eq!(buffer.channel(0), [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn iter_frames_mut_rev() {
        let mut buffer = Buffer::new(1, 4);
        let mut iterator = buffer.iter_frames_mut().rev();
        *iterator.next().unwrap().channel_mut(0) = 1.0;
        *iterator.next().unwrap().channel_mut(0) = 2.0;
        *iterator.next().unwrap().channel_mut(0) = 3.0;
        *iterator.next().unwrap().channel_mut(0) = 4.0;
        assert!(iterator.next().is_none());

        assert_eq!(buffer.channel(0), [4.0, 3.0, 2.0, 1.0]);
    }
}
