#[cfg(target_os="linux")]
pub(crate) const FRAME_TIMER_MILLISECONDS: u64 = 16;

pub trait Editor {
    const SIZE: (f64, f64);

    fn set_scale(&mut self, scale: f64);
    fn on_frame(&mut self);
}

pub struct NoEditor;

impl Editor for NoEditor {
    const SIZE: (f64, f64) = (0.0, 0.0);
    
    fn set_scale(&mut self, _scale: f64) {}

    fn on_frame(&mut self) {}    
}
