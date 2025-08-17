use std::iter::zip;

pub trait Frame<'frame> {
    type Iterator: Iterator<Item = &'frame f32>;

    fn channels(&self) -> usize;
    fn channel(&self, index: usize) -> &f32;
    fn iter(&'frame self) -> Self::Iterator;

    fn max_amplitude(&'frame self) -> f32 {
        self.iter()
            .map(|sample| sample.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }
}

pub trait FrameMut<'frame> : Frame<'frame> {
    type IteratorMut: Iterator<Item = &'frame mut f32>;

    fn channel_mut(&mut self, index: usize) -> &mut f32;
    fn iter_mut(&'frame mut self) -> Self::IteratorMut;

    fn copy_from<'source, I>(&'frame mut self, source: &'source impl Frame<'source, Iterator = I>)
    where
        I: Iterator<Item = &'source f32>,
        'source: 'frame,
    {
        for (sample_self, sample_source) in zip(self.iter_mut(), source.iter()) {
            *sample_self = *sample_source;
        }
    }
}
