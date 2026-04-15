use raw_window_handle::RawWindowHandle;

#[cfg(target_os="linux")]
pub(crate) const FRAME_TIMER_MILLISECONDS: u64 = 16;

pub trait Editor {
    const DEFAULT_SIZE: (f64, f64);

    fn open(&mut self, parent: RawWindowHandle);
    fn close(&mut self);

    /// Returns current window size
    fn window_size(&self) -> (f64, f64) {
        Self::DEFAULT_SIZE
    }

    fn can_resize(&self) -> bool {
        false
    }

    /// Called by the host to see if a window size is supported
    /// Return Some with a supported size if resizing based on the incoming size is supported
    /// Otherwise, return None
    fn check_window_size(&self, _size: (f64, f64)) -> Option<(f64, f64)> {
        None
    }

    /// Set new window size; should only be called when window is created and after a previous call to check_window_size()
    // NOTE: This doesn't take a mutable reference because on Linux, if you resize the window from on_frame() we get a RefCell borrow error
    fn set_window_size(&self, _width: f64, _height: f64) {}

    /// Set window scale; this is a suggestion that can be ignored, but it's probably a good default scale for the plugin based on OS DPI
    // NOTE: This doesn't take a mutable reference because on Linux, if you resize the window from on_frame() we get a RefCell borrow error
    fn set_scale(&self, _scale: f64) {}

    fn on_frame(&self);
}

pub struct NoEditor;

impl Editor for NoEditor {
    const DEFAULT_SIZE: (f64, f64) = (0.0, 0.0);

    fn open(&mut self, _parent: RawWindowHandle) {}
    fn close(&mut self) {}

    fn on_frame(&self) {}
}
