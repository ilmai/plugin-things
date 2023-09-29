use std::rc::Rc;

use raw_window_handle::{RawWindowHandle, HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle};

use crate::{error::Error, window::WindowAttributes, event::EventCallback, cursor::Cursor};

use super::window::OsWindow;

pub type OsWindowBuilder = Box<dyn FnOnce(OsWindowHandle) + Send>;

pub(crate) trait OsWindowInterface: HasRawDisplayHandle + HasRawWindowHandle + Sized {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
        window_builder: OsWindowBuilder,
    ) -> Result<(), Error>;

    fn set_cursor(&self, cursor: Cursor);
    fn set_input_focus(&self, focus: bool);
}

pub struct OsWindowHandle {
    raw_window_handle: RawWindowHandle,
    raw_display_handle: RawDisplayHandle,
    os_window: Option<Rc<OsWindow>>,
}

impl OsWindowHandle {
    pub(super) fn new(
        raw_window_handle: RawWindowHandle,
        raw_display_handle: RawDisplayHandle,
        os_window: Rc<OsWindow>
    ) -> Self {
        Self {
            raw_window_handle,
            raw_display_handle,
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

unsafe impl HasRawWindowHandle for OsWindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.raw_window_handle
    }
}

unsafe impl HasRawDisplayHandle for OsWindowHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.raw_display_handle
    }
}
