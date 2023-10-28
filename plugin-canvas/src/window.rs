use raw_window_handle::{RawWindowHandle, HasRawWindowHandle, HasWindowHandle, Active, HasDisplayHandle, HasRawDisplayHandle};

use crate::{platform::{window::OsWindow, interface::{OsWindowInterface, OsWindowHandle}}, error::Error, event::EventCallback, cursor::Cursor, dimensions::{LogicalSize, Scale}};

pub type WindowBuilder = Box<dyn FnOnce(Window) + Send>;

#[derive(Clone)]
pub struct WindowAttributes {
    pub(crate) size: LogicalSize,
    pub(crate) user_scale: Scale,
}

impl WindowAttributes {
    pub fn new(size: LogicalSize, user_scale: Scale) -> Self {
        Self {
            size,
            user_scale,
        }
    }

    pub fn with_size(size: LogicalSize) -> Self {
        Self::new(size, Scale::default())
    }

    pub fn size(&self) -> LogicalSize {
        self.size
    }

    pub fn user_scale(&self) -> Scale {
        self.user_scale
    }

    pub fn scaled_size(&self) -> LogicalSize {
        self.size * self.user_scale
    }
}

pub struct Window {
    attributes: WindowAttributes,

    os_window_handle: OsWindowHandle,
    active_tracker: Active,
}

impl Window {
    pub fn open(
        parent: impl HasRawWindowHandle,
        attributes: WindowAttributes,
        os_scale: f64,
        event_callback: Box<EventCallback>,
        window_builder: WindowBuilder,
    ) -> Result<(), Error> {
        OsWindow::open(
            parent.raw_window_handle(),
            attributes.clone(),
            os_scale,
            event_callback,
            {
                Box::new(move |os_window_handle| {
                    let active = Active::new();
                    unsafe { active.set_active(); }

                    let window = Self {
                        attributes,
                        os_window_handle,
                        active_tracker: Active::new(),
                    };

                    window_builder(window);
                })
            })?;

        Ok(())
    }

    pub fn attributes(&self) -> &WindowAttributes {
        &self.attributes
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        self.os_window_handle.window().set_cursor(cursor);
    }

    pub fn set_input_focus(&self, focus: bool) {
        self.os_window_handle.window().set_input_focus(focus);
    }
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.os_window_handle.raw_window_handle()
    }
}

unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        self.os_window_handle.raw_display_handle()
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let active_handle = self.active_tracker.handle().unwrap();
        let raw_window_handle = self.raw_window_handle();
        let window_handle = unsafe { raw_window_handle::WindowHandle::borrow_raw(raw_window_handle, active_handle) };
        Ok(window_handle)
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let raw_display_handle = self.raw_display_handle();
        let display_handle = unsafe { raw_window_handle::DisplayHandle::borrow_raw(raw_display_handle) };
        Ok(display_handle)
    }
}

unsafe impl Send for Window {}
