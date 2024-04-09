use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle, XlibDisplayHandle, XlibWindowHandle};
use x11rb::{connection::Connection, protocol::xproto::{ConnectionExt, CreateWindowAux, EventMask, WindowClass}, xcb_ffi::XCBConnection, COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT};

use crate::{dimensions::Size, error::Error, event::EventCallback, platform::interface::{OsWindowBuilder, OsWindowHandle, OsWindowInterface}, window::WindowAttributes};

pub struct OsWindow {
    connection: XCBConnection,
    display_handle: XlibDisplayHandle,
    window_handle: XlibWindowHandle,
}

impl OsWindowInterface for OsWindow {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
        window_builder: OsWindowBuilder,
    ) -> Result<(), Error>
    {
        let parent_window_id = match parent_window_handle {
            RawWindowHandle::Xlib(parent_window_handle) => parent_window_handle.window as u32,
            RawWindowHandle::Xcb(parent_window_handle) => parent_window_handle.window,
            _ => { return Err(Error::PlatformError("Not an X11 window".into())); }
        };

        // Create a connection through Xlib for OpenGL to work
        let dpy = unsafe { x11::xlib::XOpenDisplay(std::ptr::null()) };
        assert!(!dpy.is_null());

        let xcb_connection = unsafe { x11::xlib_xcb::XGetXCBConnection(dpy) };
        assert!(!xcb_connection.is_null());

        let screen = unsafe { x11::xlib::XDefaultScreen(dpy) } as i32;
        unsafe {
            x11::xlib_xcb::XSetEventQueueOwner(dpy, x11::xlib_xcb::XEventQueueOwner::XCBOwnsEventQueue)
        };
        
        let connection = unsafe { XCBConnection::from_raw_xcb_connection(xcb_connection as _, true)? };

        // Then we can proceed with creating the window
        let size = Size::with_logical_size(window_attributes.size, window_attributes.user_scale);

        let window_id = connection.generate_id()?;
        connection.create_window(
            COPY_DEPTH_FROM_PARENT,
            window_id,
            parent_window_id,
            0,
            0,
            size.physical_size().width as _,
            size.physical_size().height as _,
            0,
            WindowClass::INPUT_OUTPUT,
            COPY_FROM_PARENT,
            &CreateWindowAux::new()
                .event_mask(
                    EventMask::BUTTON_PRESS | 
                    EventMask::BUTTON_RELEASE | 
                    EventMask::KEY_PRESS | 
                    EventMask::KEY_RELEASE | 
                    EventMask::LEAVE_WINDOW | 
                    EventMask::POINTER_MOTION
                ),
        )?;

        connection.map_window(window_id)?;
        connection.flush()?;

        let mut display_handle = XlibDisplayHandle::empty();
        display_handle.display = dpy as _;
        display_handle.screen = screen;

        let mut window_handle = XlibWindowHandle::empty();
        window_handle.window = window_id as _;
        window_handle.visual_id = 0;

        let window = Self {
            connection,
            display_handle,
            window_handle,
        };

        let os_window_handle = OsWindowHandle::new(
            RawWindowHandle::Xlib(window_handle),
            RawDisplayHandle::Xlib(display_handle),
            window.into(),
        );

        window_builder(os_window_handle);

        Ok(())
    }

    fn set_cursor(&self, cursor: Option<cursor_icon::CursorIcon>) {
        // TODO
    }

    fn set_input_focus(&self, focus: bool) {
        // TODO
    }

    fn warp_mouse(&self, position: crate::LogicalPosition) {
        // TODO
    }
}

unsafe impl HasRawDisplayHandle for OsWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Xlib(self.display_handle)
    }
}

unsafe impl HasRawWindowHandle for OsWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Xlib(self.window_handle)
    }
}
