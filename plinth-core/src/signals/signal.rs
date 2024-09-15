use std::{iter::zip, ops::{RangeBounds, Range}};

use itertools::izip;

use crate::collections::{copy_from_slice::CopyFromSlice, interleave_iterator::InterleaveIterator};

use super::{channels::{ChannelsIterator, ChannelsIteratorMut}, frames_iterator::{FramesIterator, FramesIteratorMut}, signal_base::{SignalBase, SignalMutBase}, signal_frame::{SignalFrame, SignalFrameMut}, slice::{SignalSlice, SignalSliceMut}};

pub trait Signal : SignalBase {
    fn iter_channels(&self) -> ChannelsIterator<'_, Self>;
    fn frame(&self, index: usize) -> SignalFrame<'_, Self>;
    fn iter_frames(&self) -> FramesIterator<'_, Self>;

    fn channel(&self, channel: usize) -> &[f32] {
        unsafe { &*self.channel_ptr(channel) }
    }

    fn slice<T: RangeBounds<usize>>(&self, range: T) -> SignalSlice<'_, Self> {
        SignalSlice::new(self, range)
    }

    fn iter_interleaved(&self) -> InterleaveIterator<&f32, std::slice::Iter<'_, f32>> {
        InterleaveIterator::new(self.iter_channels().map(|channel| channel.iter()))
    }

    fn mix_to(&self, self_gain: f32, other: &impl Signal, other_gain: f32, target: &mut impl SignalMut) {
        for (self_channel, other_channel, target_channel) in izip!(self.iter_channels(), other.iter_channels(), target.iter_channels_mut()) {
            for (self_sample, other_sample, target_sample) in izip!(self_channel, other_channel, target_channel) {
                *target_sample = self_sample * self_gain + other_sample * other_gain;
            }
        }
    }

    fn apply_wrap(&self, index: usize, length: usize, mut function: impl FnMut(&SignalSlice<'_, Self>, Range<usize>)) {
        assert!(index < self.len(), "index out of bounds: {index}");

        let slice_len_1 = usize::min(length, self.len() - index);
        let slice_len_2 = length - slice_len_1;

        function(&self.slice(index..index + slice_len_1), 0..slice_len_1);
        if slice_len_2 > 0 {
            function(&self.slice(..slice_len_2), slice_len_1..slice_len_1 + slice_len_2);
        }
    }
}

pub trait SignalMut: Signal + SignalMutBase {
    fn iter_channels_mut(&mut self) -> ChannelsIteratorMut<'_, Self>;
    fn frame_mut(&mut self, index: usize) -> SignalFrameMut<'_, Self>;
    fn iter_frames_mut(&mut self) -> FramesIteratorMut<'_, Self>;

    fn channel_mut(&mut self, channel: usize) -> &mut [f32] {
        unsafe { &mut *self.channel_ptr_mut(channel) }
    }   

    fn slice_mut<T: RangeBounds<usize>>(&mut self, range: T) -> SignalSliceMut<'_, Self> {
        SignalSliceMut::new(self, range)
    }

    fn iter_interleaved_mut(&mut self) -> InterleaveIterator<&mut f32, std::slice::IterMut<'_, f32>> {
        InterleaveIterator::new(self.iter_channels_mut().map(|channel| channel.iter_mut()))
    }

    fn fill(&mut self, value: f32) {
        for channel in self.iter_channels_mut() {
            channel.fill(value);
        }
    }

    fn scale(&mut self, scale: f32) {
        for channel in self.iter_channels_mut() {
            for sample in channel.iter_mut() {
                *sample *= scale;
            }
        }
    }

    fn copy_from_signal(&mut self, source: &impl Signal) {
        assert_eq!(self.channels(), source.channels());
        assert_eq!(self.len(), source.len(), "Attempting to copy a signal of length {} into a signal of length {}", source.len(), self.len());

        for (target_channel, source_channel) in zip(self.iter_channels_mut(), source.iter_channels()) {
            target_channel.copy_from_slice(source_channel);
        }
    }

    fn copy_from_signal_and_fill(&mut self, source: &impl Signal, value: f32) {
        assert!(self.channels() == source.channels());

        for (target_channel, source_channel) in zip(self.iter_channels_mut(), source.iter_channels()) {
            target_channel.copy_from_slice_and_fill(source_channel, value);
        }
    }

    fn add_from_signal(&mut self, source: &impl Signal) {
        assert!(self.channels() == source.channels());

        for (target_channel, source_channel) in zip(self.iter_channels_mut(), source.iter_channels()) {
            for (target_sample, source_sample) in zip(target_channel, source_channel) {
                *target_sample += source_sample;
            }
        }
    }

    fn mix_signal(&mut self, gain_self: f32, source: &impl Signal, gain_source: f32) {
        for (self_channel, source_channel) in izip!(self.iter_channels_mut(), source.iter_channels()) {
            for (self_sample, source_sample) in zip(self_channel, source_channel) {
                *self_sample = *self_sample * gain_self + source_sample * gain_source;
            }
        }
    }

    fn apply_wrap_mut(&mut self, index: usize, length: usize, mut function: impl FnMut(&mut SignalSliceMut<'_, Self>, Range<usize>)) {
        assert!(index < self.len(), "index out of bounds: {index}/{}", self.len());

        let slice_len_1 = usize::min(length, self.len() - index);
        let slice_len_2 = length - slice_len_1;

        function(&mut self.slice_mut(index..index + slice_len_1), 0..slice_len_1);
        if slice_len_2 > 0 {
            function(&mut self.slice_mut(..slice_len_2), slice_len_1..slice_len_1 + slice_len_2);
        }
    }
}

impl<T: SignalBase> Signal for T {
    fn iter_channels(&self) -> ChannelsIterator<'_, Self> {
        ChannelsIterator::new(self)
    }

    fn frame(&self, index: usize) -> SignalFrame<'_, Self> {
        SignalFrame::new(self, index)
    }

    fn iter_frames(&self) -> FramesIterator<'_, Self> {
        FramesIterator::new(self)
    }
}

impl<T: Signal + SignalMutBase> SignalMut for T {
    fn iter_channels_mut(&mut self) -> ChannelsIteratorMut<'_, Self> {
        ChannelsIteratorMut::new(self)
    }

    fn frame_mut(&mut self, index: usize) -> SignalFrameMut<'_, Self> {
        SignalFrameMut::new(self, index)
    }

    fn iter_frames_mut(&mut self) -> FramesIteratorMut<'_, Self> {
        FramesIteratorMut::new(self)
    }
}

impl<T: AsRef<[f32]>> SignalBase for T {
    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn channels(&self) -> usize {
        1
    }

    fn channel_ptr(&self, channel: usize) -> *const [f32] {
        assert_eq!(channel, 0);
        self.as_ref() as *const [f32]
    }
}

impl<T: AsMut<[f32]> + AsRef<[f32]>> SignalMutBase for T {
    fn channel_ptr_mut(&mut self, channel: usize) -> *mut [f32] {
        assert_eq!(channel, 0);
        self.as_mut() as *mut [f32]
    }
}
