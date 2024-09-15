use crate::util::ptr::any_null;

use super::signal_base::{SignalBase, SignalMutBase};

pub struct PtrSignal {
    channels: usize,
    length: usize,
    channels_pointers: *const *const f32,
}

impl PtrSignal {
    // SAFETY: Caller is responsible for channels and length matching the pointers,
    // and for taking care the pointers live long enough
    pub unsafe fn from_pointers(channels: usize, length: usize, channels_pointers: *const *const f32) -> Self {
        assert!(!channels_pointers.is_null());
        assert!(unsafe { !any_null(channels_pointers, channels) });

        Self {
            channels,
            length,
            channels_pointers,
        }
    }

    pub fn pointers(&self) -> &[*const f32] {
        unsafe { std::slice::from_raw_parts(self.channels_pointers, self.channels) }
    }
}

impl SignalBase for PtrSignal {
    fn len(&self) -> usize {
        self.length
    }

    fn channels(&self) -> usize {
        self.channels
    }

    fn channel_ptr(&self, channel: usize) -> *const [f32] {
        unsafe {
            let channel_pointers = std::slice::from_raw_parts(self.channels_pointers, self.channels);
            let channel_pointer = std::slice::from_raw_parts(channel_pointers[channel], self.length);
            channel_pointer as _
        }
    }
}

pub struct PtrSignalMut {
    channels: usize,
    length: usize,
    channels_pointers: *mut *mut f32,
}

impl PtrSignalMut {
    // SAFETY: Caller is responsible for channels and length matching the pointers,
    // and for taking care the pointers live long enough
    pub unsafe fn from_pointers(channels: usize, length: usize, channels_pointers: *mut *mut f32) -> Self {
        Self {
            channels,
            length,
            channels_pointers,
        }
    }

    pub fn pointers(&self) -> &[*mut f32] {
        unsafe { std::slice::from_raw_parts(self.channels_pointers, self.channels) }
    }
}

impl SignalBase for PtrSignalMut {
    fn len(&self) -> usize {
        self.length
    }

    fn channels(&self) -> usize {
        self.channels
    }

    fn channel_ptr(&self, channel: usize) -> *const [f32] {
        unsafe {
            let channel_pointers = std::slice::from_raw_parts(self.channels_pointers, self.channels);
            let channel_pointer = std::slice::from_raw_parts(channel_pointers[channel], self.length);
            channel_pointer as _
        }
    }
}

impl SignalMutBase for PtrSignalMut {
    fn channel_ptr_mut(&mut self, channel: usize) -> *mut [f32] {
        unsafe {
            let channel_pointers = std::slice::from_raw_parts_mut(self.channels_pointers, self.channels);
            let channel_pointer = std::slice::from_raw_parts_mut(channel_pointers[channel], self.length);
            channel_pointer as _
        }
    }
}
