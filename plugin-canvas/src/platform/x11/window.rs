use std::{cell::RefCell, ffi::OsStr, ptr::NonNull};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, XlibDisplayHandle, XlibWindowHandle};
use sys_locale::get_locales;
use x11rb::{connection::Connection, protocol::xproto::{ConfigureWindowAux, ConnectionExt, CreateWindowAux, EventMask, GrabMode, WindowClass}, xcb_ffi::XCBConnection, COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT};
use xkbcommon::xkb;

use crate::{dimensions::Size, error::Error, event::{EventCallback, EventResponse}, platform::interface::{OsWindowHandle, OsWindowInterface}, window::WindowAttributes, Event, MouseButton, PhysicalPosition};

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
    pub(super) fn send_event(&self, event: Event) -> EventResponse {
        (self.event_callback)(event)
    }

    fn handle_event(&self, event: x11rb::protocol::Event) -> Result<(), Error> {
        match event {
            x11rb::protocol::Event::ButtonPress(event) => {
                let position = PhysicalPosition {
                    x: event.event_x as i32,
                    y: event.event_y as i32,
                }.to_logical(self.window_attributes.scale);

                if let Some(button) = Self::mouse_button_from_detail(event.detail) {
                    self.send_event(Event::MouseButtonDown {
                        button,
                        position,
                    });    
                } else if [4, 5].contains(&event.detail) {
                    let delta_y = if event.detail == 4 {
                        -1.0
                    } else {
                        1.0
                    };

                    self.send_event(Event::MouseWheel {
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
                }.to_logical(self.window_attributes.scale);

                if let Some(button) = Self::mouse_button_from_detail(event.detail) {
                    self.send_event(Event::MouseButtonUp {
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
                    self.send_event(Event::KeyDown { text });
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
                    self.send_event(Event::KeyDown { text });
                }
            }

            x11rb::protocol::Event::KeyRelease(event) => {
                let keycode = xkb::Keycode::new(event.detail as u32);
                let mut xkb_state = self.xkb_state.borrow_mut();

                let text = xkb_state.key_get_utf8(keycode);
                xkb_state.update_key(keycode, xkb::KeyDirection::Up);
                
                if !text.is_empty() {
                    self.send_event(Event::KeyUp { text });
                }
            }

            x11rb::protocol::Event::LeaveNotify(_) => {
                self.send_event(Event::MouseExited);
            }

            x11rb::protocol::Event::MotionNotify(event) => {
                let position = PhysicalPosition {
                    x: event.event_x as i32,
                    y: event.event_y as i32,
                }.to_logical(self.window_attributes.scale);

                self.send_event(Event::MouseMoved { position });
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
    ) -> Result<OsWindowHandle, Error>
    {
        let parent_window_id = match parent_window_handle {
            RawWindowHandle::Xlib(parent_window_handle) => parent_window_handle.window as u32,
            RawWindowHandle::Xcb(parent_window_handle) => parent_window_handle.window.get(),
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
        let size = Size::with_logical_size(window_attributes.size, window_attributes.scale);

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

        // Go through possible locales until we find one with a keyboard compose table
        // Fall back to the "C" locale
        let mut locales: Vec<_> = get_locales().collect();
        locales.push("C".into());

        let mut xkb_compose_state = None;
        
        for locale in locales.iter() {
            if let Ok(compose_table) = xkb::compose::Table::new_from_locale(&xkb_context, OsStr::new(&locale), 0) {
                xkb_compose_state = Some(xkb::compose::State::new(&compose_table, 0));
                break;
            }
        }

        assert!(xkb_compose_state.is_some(), "Couldn't find keyboard compose table for any of the locales: {locales:?}");

        let display_handle = XlibDisplayHandle::new(Some(NonNull::new(dpy as _).unwrap()), screen);
        let window_handle = XlibWindowHandle::new(window_id as _);

        let window = Self {
            window_attributes,
            event_callback,

            connection,
            xkb_state: xkb_state.into(),
            xkb_compose_state: xkb_compose_state.unwrap().into(),

            display_handle,
            window_handle,
        };

        Ok(OsWindowHandle::new(window.into()))
    }

    fn os_scale(&self) -> f64 {
        1.0
    }

    fn set_cursor(&self, _cursor: Option<cursor_icon::CursorIcon>) {
        // TODO
    }

    fn set_input_focus(&self, focus: bool) {
        if focus {
            self.connection.grab_keyboard(
                false,
                self.window_handle.window as u32,
                x11rb::CURRENT_TIME,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            ).unwrap();
        } else {
            self.connection.ungrab_keyboard(x11rb::CURRENT_TIME).unwrap();
        }
    }

    fn warp_mouse(&self, _position: crate::LogicalPosition) {
        // TODO
    }

    fn poll_events(&self) -> Result<(), Error> {
        while let Some(event) = self.connection.poll_for_event()? {
            self.handle_event(event)?;
        }

        Ok(())
    }

    fn set_size(&self, size: crate::LogicalSize) {
        let size = Size::with_logical_size(size, self.window_attributes.scale);
        let values = ConfigureWindowAux::default()
            .width(size.physical_size().width)
            .height(size.physical_size().height);
        let _ = self.connection.configure_window(self.window_handle.window as u32, &values);
    }
}

impl HasDisplayHandle for OsWindow {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        let raw_display_handle = RawDisplayHandle::Xlib(self.display_handle);
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(raw_display_handle) })
    }
}

impl HasWindowHandle for OsWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw_window_handle = RawWindowHandle::Xlib(self.window_handle);
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw_window_handle) })
    }
}
