use std::{cell::RefCell, ffi::OsStr};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle, XlibDisplayHandle, XlibWindowHandle};
use sys_locale::get_locale;
use x11rb::{connection::Connection, protocol::xproto::{ConnectionExt, CreateWindowAux, EventMask, WindowClass}, xcb_ffi::XCBConnection, COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT};
use xkbcommon::xkb;

use crate::{dimensions::Size, error::Error, event::EventCallback, platform::interface::{OsWindowBuilder, OsWindowHandle, OsWindowInterface}, window::WindowAttributes, Event, MouseButton, PhysicalPosition};

pub struct OsWindow {
    window_attributes: WindowAttributes,
    event_callback: Box<EventCallback>,

    connection: XCBConnection,
    xkb_state: RefCell<xkb::State>,
    xkb_compose_state: RefCell<xkb::compose::State>,

    display_handle: XlibDisplayHandle,
    window_handle: XlibWindowHandle,
}

impl OsWindow {
    fn handle_event(&self, event: x11rb::protocol::Event) -> Result<(), Error> {
        match event {
            x11rb::protocol::Event::ButtonPress(event) => {
                let position = PhysicalPosition {
                    x: event.event_x as i32,
                    y: event.event_y as i32,
                }.to_logical(self.window_attributes.user_scale);

                if let Some(button) = Self::mouse_button_from_detail(event.detail) {
                    (self.event_callback)(Event::MouseButtonDown {
                        button,
                        position,
                    });    
                } else if [4, 5].contains(&event.detail) {
                    let delta_y = if event.detail == 4 {
                        -1.0
                    } else {
                        1.0
                    };

                    (self.event_callback)(Event::MouseWheel {
                        position,
                        delta_x: 0.0,
                        delta_y,
                    });
                }
            }

            x11rb::protocol::Event::ButtonRelease(event) => {
                let position = PhysicalPosition {
                    x: event.event_x as i32,
                    y: event.event_y as i32,
                }.to_logical(self.window_attributes.user_scale);

                if let Some(button) = Self::mouse_button_from_detail(event.detail) {
                    (self.event_callback)(Event::MouseButtonUp {
                        button,
                        position,
                    });    
                }
            }

            x11rb::protocol::Event::KeyPress(event) => {
                let keycode = xkb::Keycode::new(event.detail as u32);
                let mut text = String::new();

                let mut xkb_state = self.xkb_state.borrow_mut();
                let mut xkb_compose_state = self.xkb_compose_state.borrow_mut();

                if let Some(text) = match xkb_state.get_keymap().key_get_name(keycode) {
                    Some("UP") => Some("\u{f700}"),
                    Some("DOWN") => Some("\u{f701}"),
                    Some("LEFT") => Some("\u{f702}"),
                    Some("RGHT") => Some("\u{f703}"),
                    _ => None,
                } {
                    let text = text.to_string();
                    (self.event_callback)(Event::KeyDown { text });
                }

                for keysym in xkb_state.key_get_syms(keycode) {
                    xkb_compose_state.feed(*keysym);
                    if xkb_compose_state.status() == xkb::Status::Composed {
                        // We're assuming here that a single key press can only generate one piece of text, is this true?
                        text = xkb_compose_state.utf8().unwrap();
                    }
                }

                if text.is_empty() {
                    text = xkb_state.key_get_utf8(keycode);
                }

                xkb_state.update_key(keycode, xkb::KeyDirection::Down);

                if !text.is_empty() {
                    (self.event_callback)(Event::KeyDown { text });
                }
            }

            x11rb::protocol::Event::KeyRelease(event) => {
                let keycode = xkb::Keycode::new(event.detail as u32);
                let mut xkb_state = self.xkb_state.borrow_mut();

                let text = xkb_state.key_get_utf8(keycode);
                xkb_state.update_key(keycode, xkb::KeyDirection::Up);
                
                if !text.is_empty() {
                    (self.event_callback)(Event::KeyUp { text });
                }
            }

            x11rb::protocol::Event::LeaveNotify(_) => {
                (self.event_callback)(Event::MouseExited);
            }

            x11rb::protocol::Event::MotionNotify(event) => {
                let position = PhysicalPosition {
                    x: event.event_x as i32,
                    y: event.event_y as i32,
                }.to_logical(self.window_attributes.user_scale);

                (self.event_callback)(Event::MouseMoved { position });
            }
            
            _ => {},
        }

        Ok(())
    }

    fn mouse_button_from_detail(detail: u8) -> Option<MouseButton> {
        match detail {
            1 => Some(MouseButton::Left),
            2 => Some(MouseButton::Middle),
            3 => Some(MouseButton::Right),
            _ => None,
        }
    }
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

        // Init xkbcommon
        let xkb_context = xkb::Context::new(0);
        let keyboard_device = xkb::x11::get_core_keyboard_device_id(&connection);
        let keymap = xkb::x11::keymap_new_from_device(&xkb_context, &connection, keyboard_device, 0);
        let xkb_state = xkb::x11::state_new_from_device(&keymap, &connection, keyboard_device);

        let locale = get_locale().unwrap_or_else(|| String::from("en-US"));
        let compose_table = xkb::compose::Table::new_from_locale(&xkb_context, OsStr::new(&locale), 0).unwrap();
        let xkb_compose_state = xkb::compose::State::new(&compose_table, 0);

        let mut display_handle = XlibDisplayHandle::empty();
        display_handle.display = dpy as _;
        display_handle.screen = screen;

        let mut window_handle = XlibWindowHandle::empty();
        window_handle.window = window_id as _;
        window_handle.visual_id = 0;

        let window = Self {
            window_attributes,
            event_callback,

            connection,
            xkb_state: xkb_state.into(),
            xkb_compose_state: xkb_compose_state.into(),

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

    fn poll_events(&self) -> Result<(), Error> {
        while let Some(event) = self.connection.poll_for_event()? {
            self.handle_event(event)?;
        }

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
