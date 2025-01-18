#[cfg(target_os="linux")]
pub(crate) const FRAME_TIMER_MILLISECONDS: u64 = 16;

pub trait Editor {
    const DEFAULT_SIZE: (f64, f64);
    const CAN_RESIZE: bool = false;

    /// Returns current window size
    fn window_size(&self) -> (f64, f64) {
        Self::DEFAULT_SIZE
    }
    
    /// Called by the host to see if a window size is supported
    /// Return Some with a supported size if resizing based on the incoming size is supported
    /// Otherwise, return None
    fn check_window_size(&self, _size: (f64, f64)) -> Option<(f64, f64)> {
        None
    }

    /// Set new window size; should only be called when window is created and after a previous call to check_window_size()
    fn set_window_size(&mut self, _width: f64, _height: f64) {}

    fn on_frame(&mut self);
}

pub struct NoEditor;

impl Editor for NoEditor {
    const DEFAULT_SIZE: (f64, f64) = (0.0, 0.0);

    fn on_frame(&mut self) {}    
}
