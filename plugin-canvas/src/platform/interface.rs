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

    fn resized(&self, size: LogicalSize);

    fn set_cursor(&self, cursor: Option<CursorIcon>);
    fn set_input_focus(&self, focus: bool);
    fn warp_mouse(&self, position: LogicalPosition);

    fn poll_events(&self) -> Result<(), Error>;
}

pub struct OsWindowHandle {
    os_window: Rc<OsWindow>,
}

impl OsWindowHandle {
    pub(super) fn new(os_window: Rc<OsWindow>) -> Self {
        Self {
            os_window,
        }
    }

    pub(crate) fn window(&self) -> &OsWindow {
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
