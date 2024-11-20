#[cfg(target_os="linux")]
pub(crate) const FRAME_TIMER_MILLISECONDS: u64 = 16;

pub trait Editor {
    const SIZE: (f64, f64);
    const IS_RESIZABLE: bool = true;

    fn set_scale(&mut self, scale: f64);
    fn on_frame(&mut self);

    fn set_window_size(&mut self, _width: f64, _height: f64) {}
    fn window_size(&self) -> (f64, f64) {
        Self::SIZE
    }
}

pub struct NoEditor;

impl Editor for NoEditor {
    const SIZE: (f64, f64) = (0.0, 0.0);
    const IS_RESIZABLE: bool = false;
    
    fn set_window_size(&mut self, _width: f64, _height: f64) {}
    fn set_scale(&mut self, _scale: f64) {}
    fn on_frame(&mut self) {}    
}
