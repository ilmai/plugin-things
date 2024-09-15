pub trait CopyFromSlice<T: Copy> {
    fn copy_from_slice_and_fill(&mut self, src: &[T], value: T);
}

impl<T: Copy> CopyFromSlice<T> for [T] {
    fn copy_from_slice_and_fill(&mut self, src: &[T], value: T)
    {
        let copy_len = usize::min(self.len(), src.len());

        if copy_len > 0 {
            self[..copy_len].copy_from_slice(&src[..copy_len]);
        }        
        if copy_len < self.len() {
            self[copy_len..].fill(value);
        }
    }
}
