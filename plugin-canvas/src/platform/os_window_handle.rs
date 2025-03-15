use std::{ops::Deref, rc::Rc};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::window::OsWindow;

pub(crate) struct OsWindowHandle {
    os_window: Rc<OsWindow>,
}

impl OsWindowHandle {
    pub(super) fn new(os_window: Rc<OsWindow>) -> Self {
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
