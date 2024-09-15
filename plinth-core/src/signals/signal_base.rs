pub trait SignalBase {
    fn len(&self) -> usize;
    fn channels(&self) -> usize;
    fn channel_ptr(&self, channel: usize) -> *const [f32];

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub trait SignalMutBase {
    fn channel_ptr_mut(&mut self, channel: usize) -> *mut [f32];
}
