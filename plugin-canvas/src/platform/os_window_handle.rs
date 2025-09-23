use std::sync::Arc;
use std::ops::Deref;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::thread_bound::ThreadBound;

use super::window::OsWindow;

pub(crate) struct OsWindowHandle {
    os_window: Arc<ThreadBound<OsWindow>>,
}

impl OsWindowHandle {
    pub(super) fn new(os_window: Arc<ThreadBound<OsWindow>>) -> Self {
        Self {
            os_window,
        }
    }
}

impl Deref for OsWindowHandle {
    type Target = OsWindow;

    fn deref(&self) -> &Self::Target {
        self.os_window.as_ref()
    }
}

impl HasWindowHandle for OsWindowHandle {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.os_window.as_ref().window_handle()
    }
}

impl HasDisplayHandle for OsWindowHandle {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.os_window.as_ref().display_handle()
    }
}
