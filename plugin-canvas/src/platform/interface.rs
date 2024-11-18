use std::rc::Rc;

use cursor_icon::CursorIcon;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};

use crate::{error::Error, event::EventCallback, window::WindowAttributes, LogicalPosition, LogicalSize};

use super::window::OsWindow;

pub(crate) trait OsWindowInterface: HasDisplayHandle + HasWindowHandle + Sized {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
    ) -> Result<OsWindowHandle, Error>;

    fn os_scale(&self) -> f64;

    fn set_cursor(&self, cursor: Option<CursorIcon>);
    fn set_input_focus(&self, focus: bool);
    fn warp_mouse(&self, position: LogicalPosition);
    fn set_size(&self, _size: LogicalSize) {}

    fn poll_events(&self) -> Result<(), Error>;
}

pub struct OsWindowHandle {
    os_window: Option<Rc<OsWindow>>,
}

impl OsWindowHandle {
    pub(super) fn new(
        os_window: Rc<OsWindow>
    ) -> Self {
        Self {
            os_window: Some(os_window),
        }
    }

    pub(crate) fn window(&self) -> &OsWindow {
        self.os_window.as_ref().unwrap()
    }
}

impl Drop for OsWindowHandle {
    fn drop(&mut self) {
        let ref_count = Rc::strong_count(self.os_window.as_ref().unwrap());
        // If reference count before drop is 2, it means the only remaining reference
        // to OsWindow is the one that's held by OsWindow itself since external code
        // holds a pointer to it. It should be safe to manually drop the last two references
        // at that point.
        if ref_count == 2 {
            let os_window = self.os_window.take().unwrap();
            let ptr = Rc::into_raw(os_window);
            unsafe {
                Rc::decrement_strong_count(ptr);
                Rc::decrement_strong_count(ptr);
            }
        }
    }
}

impl HasWindowHandle for OsWindowHandle {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.os_window.as_ref().unwrap().window_handle()
    }
}

impl HasDisplayHandle for OsWindowHandle {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.os_window.as_ref().unwrap().display_handle()
    }
}
