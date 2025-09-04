use std::{ops::Deref, rc::Rc};

use dispatch2::MainThreadBound;
use objc2::MainThreadMarker;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::window::OsWindow;

pub(crate) struct OsWindowHandle {
    os_window: MainThreadBound<Rc<OsWindow>>,
}

impl OsWindowHandle {
    pub(super) fn new(os_window: Rc<OsWindow>, mtm: MainThreadMarker) -> Self {
        Self {
            os_window: MainThreadBound::new(os_window, mtm),
        }
    }
}

impl Deref for OsWindowHandle {
    type Target = OsWindow;

    fn deref(&self) -> &Self::Target {
        // TODO: Should we do actual error handling or is it always our bug if this fails?
        let mtm = MainThreadMarker::new().unwrap();
        self.os_window.get(mtm)
    }
}

impl HasWindowHandle for OsWindowHandle {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // TODO: Should we do actual error handling or is it always our bug if this fails?
        let mtm = MainThreadMarker::new().unwrap();
        self.os_window.get(mtm).window_handle()
    }
}

impl HasDisplayHandle for OsWindowHandle {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        // TODO: Should we do actual error handling or is it always our bug if this fails?
        let mtm = MainThreadMarker::new().unwrap();
        self.os_window.get(mtm).display_handle()
    }
}
