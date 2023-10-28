use std::{rc::Rc, sync::{mpsc::{self, Sender}, Arc, Mutex}, os::fd::{AsRawFd, BorrowedFd}, time::{Instant, Duration}, ffi::OsStr};

use nix::poll::{poll, PollFd, PollFlags};
use raw_window_handle::{RawWindowHandle, HasRawWindowHandle, HasRawDisplayHandle, RawDisplayHandle, XlibWindowHandle, XlibDisplayHandle};
use sys_locale::get_locale;
use xcb::{x::{self, GrabStatus}, XidNew, Xid};
use xkbcommon::xkb;

use crate::{window::WindowAttributes, event::EventCallback, error::Error, platform::interface::{OsWindowInterface, OsWindowHandle, OsWindowBuilder}, dimensions::Size, Event, MouseButton, LogicalPosition, cursor::Cursor};

enum OsWindowEvent {
    Error(Error),
}

struct Context {
    event_callback: Box<EventCallback>,
    connection: xcb::Connection,
    xkb_state: xkb::State,
    xkb_compose_state: xkb::compose::State,
}

pub struct OsWindow {
    window_handle: XlibWindowHandle,
    display_handle: XlibDisplayHandle,

    cursor_none: x::Cursor,
    cursor_arrow: x::Cursor,
    cursor_bottom_left_corner: x::Cursor,
    cursor_bottom_right_corner: x::Cursor,
    cursor_bottom_side: x::Cursor,
    cursor_crosshair: x::Cursor,
    cursor_double_arrow: x::Cursor,
    cursor_fleur: x::Cursor,
    cursor_h_double_arrow: x::Cursor,
    cursor_left_side: x::Cursor,
    cursor_plus: x::Cursor,
    cursor_pointer: x::Cursor,
    cursor_question_arrow: x::Cursor,
    cursor_right_side: x::Cursor,
    cursor_top_left_corner: x::Cursor,
    cursor_top_right_corner: x::Cursor,
    cursor_top_side: x::Cursor,
    cursor_watch: x::Cursor,
    cursor_x: x::Cursor,
    cursor_xterm: x::Cursor,

    new_cursor: Arc<Mutex<Option<x::Cursor>>>,
    set_input_focus: Arc<Mutex<Option<bool>>>,
}

impl OsWindow {
    fn window_thread(
        parent_window_id: u32,
        window_attributes: WindowAttributes,
        os_scale: f64,
        event_callback: Box<EventCallback>,
        window_event_sender: Sender<OsWindowEvent>,
        build_window: OsWindowBuilder,
    ) {
        let new_cursor: Arc<Mutex<Option<x::Cursor>>> = Default::default();
        let set_input_focus: Arc<Mutex<Option<bool>>> = Default::default();

        let (connection, window_id, xkb_state, xkb_compose_state) = match Self::create_window(parent_window_id, window_attributes.clone(), os_scale, build_window, new_cursor.clone(), set_input_focus.clone()) {
            Ok(connection) => connection,
            Err(error) => {
                window_event_sender.send(OsWindowEvent::Error(error)).unwrap();
                return;
            }
        };
        drop(window_event_sender);

        let mut context = Context {
            event_callback,
            connection,
            xkb_state,
            xkb_compose_state,
        };

        let connection_fd = unsafe { BorrowedFd::borrow_raw(context.connection.as_raw_fd()) };
        let mut poll_fds = [PollFd::new(&connection_fd, PollFlags::POLLIN)];

        // TODO: Real sync please
        let frame_time = Duration::from_millis(16);
        let mut next_frame_time = Instant::now();

        loop {
            if let Some(cursor) = new_cursor.lock().unwrap().take() {
                // TODO: Error handling
                context.connection.send_and_check_request(&x::ChangeWindowAttributes {
                    window: window_id,
                    value_list: &[x::Cw::Cursor(cursor)],
                }).unwrap();
            }

            if let Some(set_input_focus) = set_input_focus.lock().unwrap().take() {
                // TODO: Error handling
                if set_input_focus {
                    let cookie = context.connection.send_request(&x::GrabKeyboard {
                        owner_events: false,
                        grab_window: window_id,
                        time: x::CURRENT_TIME,
                        pointer_mode: x::GrabMode::Async,
                        keyboard_mode: x::GrabMode::Async,
                    });
                    let reply = context.connection.wait_for_reply(cookie).unwrap();
                    assert_eq!(reply.status(), GrabStatus::Success);
                } else {
                    context.connection.send_and_check_request(&x::UngrabKeyboard {
                        time: x::CURRENT_TIME,
                    }).unwrap();
                }
            }

            // Handle events before drawing to get up to date state
            Self::handle_events(&mut context);

            if Instant::now() >= next_frame_time {
                (context.event_callback)(Event::Draw);

                // This is stupid but should work
                while next_frame_time < Instant::now() {
                    next_frame_time += frame_time;            
                }
            };

            // Handle events before going to sleep so they're not delayed
            Self::handle_events(&mut context);

            let time_until_next_frame = next_frame_time.saturating_duration_since(Instant::now());
            if !time_until_next_frame.is_zero() {
                // TODO: Error handling
                poll(&mut poll_fds, time_until_next_frame.as_millis() as i32).unwrap();
            }
        }
    }
    
    fn create_window(
        parent_window_id: u32,
        window_attributes: WindowAttributes,
        os_scale: f64,
        build_window: OsWindowBuilder,
        new_cursor: Arc<Mutex<Option<x::Cursor>>>,
        set_input_focus: Arc<Mutex<Option<bool>>>,
    ) -> Result<(xcb::Connection, x::Window, xkb::State, xkb::compose::State), Error> {
        let parent_window_id = unsafe { x::Window::new(parent_window_id) };
        let size = Size::with_logical_size(window_attributes.size, window_attributes.scale * os_scale);
    
        let (connection, screen_number) = xcb::Connection::connect_with_xlib_display_and_extensions(
            &[], // Mandatory
            &[], // Optional
        )?;

        connection.set_event_queue_owner(xcb::EventQueueOwner::Xcb);
    
        let window_id: x::Window = connection.generate_id();

        connection.send_and_check_request(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: window_id,
            parent: parent_window_id,
            x: 0,
            y: 0,
            width: size.physical_size().width as u16,
            height: size.physical_size().height as u16,
            border_width: 0,
            class: x::WindowClass::InputOutput,
            visual: x::COPY_FROM_PARENT,
            value_list: &[x::Cw::EventMask(
                x::EventMask::BUTTON_PRESS | 
                x::EventMask::BUTTON_RELEASE | 
                x::EventMask::KEY_PRESS | 
                x::EventMask::KEY_RELEASE | 
                x::EventMask::LEAVE_WINDOW | 
                x::EventMask::POINTER_MOTION
            )],
        })?;

        // Init xkbcommon
        let xkb_context = xkb::Context::new(0);
        let keyboard_device = xkb::x11::get_core_keyboard_device_id(&connection);
        let keymap = xkb::x11::keymap_new_from_device(&xkb_context, &connection, keyboard_device, 0);
        let xkb_state = xkb::x11::state_new_from_device(&keymap, &connection, keyboard_device);

        let locale = get_locale().unwrap_or_else(|| String::from("en-US"));
        let compose_table = xkb::compose::Table::new_from_locale(&xkb_context, OsStr::new(&locale), 0).unwrap();
        let xkb_compose_state = xkb::compose::State::new(&compose_table, 0);
        
        // Show window
        connection.send_and_check_request(&x::MapWindow { window: window_id })?;
    
        let mut window_handle = XlibWindowHandle::empty();
        window_handle.window = window_id.resource_id() as _;
        window_handle.visual_id = 0;
    
        let mut display_handle = XlibDisplayHandle::empty();
        display_handle.display = connection.get_raw_dpy() as _;
        display_handle.screen = screen_number;
    
        let raw_window_handle = RawWindowHandle::Xlib(window_handle);
        let raw_display_handle = RawDisplayHandle::Xlib(display_handle);
        
        let cursor_font: x::Font = connection.generate_id();

        connection.send_and_check_request(&x::OpenFont {
            fid: cursor_font,
            name: b"cursor",
        })?;

        let window = Rc::new(OsWindow {
            window_handle,
            display_handle,

            cursor_none: connection.generate_id(),
            cursor_arrow: Self::create_cursor(&connection, cursor_font, 2)?,
            cursor_bottom_left_corner: Self::create_cursor(&connection, cursor_font, 12)?,
            cursor_bottom_right_corner: Self::create_cursor(&connection, cursor_font, 14)?,
            cursor_bottom_side: Self::create_cursor(&connection, cursor_font, 16)?,
            cursor_crosshair: Self::create_cursor(&connection, cursor_font, 34)?,
            cursor_double_arrow: Self::create_cursor(&connection, cursor_font, 42)?,
            cursor_fleur: Self::create_cursor(&connection, cursor_font, 52)?,
            cursor_h_double_arrow: Self::create_cursor(&connection, cursor_font, 108)?,
            cursor_left_side: Self::create_cursor(&connection, cursor_font, 70)?,
            cursor_plus: Self::create_cursor(&connection, cursor_font, 90)?,
            cursor_pointer: Self::create_cursor(&connection, cursor_font, 60)?,
            cursor_question_arrow: Self::create_cursor(&connection, cursor_font, 92)?,
            cursor_right_side: Self::create_cursor(&connection, cursor_font, 96)?,
            cursor_top_left_corner: Self::create_cursor(&connection, cursor_font, 134)?,
            cursor_top_right_corner: Self::create_cursor(&connection, cursor_font, 136)?,
            cursor_top_side: Self::create_cursor(&connection, cursor_font, 138)?,
            cursor_watch: Self::create_cursor(&connection, cursor_font, 150)?,
            cursor_x: Self::create_cursor(&connection, cursor_font, 0)?,
            cursor_xterm: Self::create_cursor(&connection, cursor_font, 152)?,

            new_cursor,
            set_input_focus,
        });
    
        let os_window_handle = OsWindowHandle::new(raw_window_handle, raw_display_handle, window);
        build_window(os_window_handle);
    
        Ok((connection, window_id, xkb_state, xkb_compose_state))
    }

    fn create_cursor(connection: &xcb::Connection, font: x::Font, char: u16) -> Result<x::Cursor, Error> {
        let id: x::Cursor = connection.generate_id();

        connection.send_and_check_request(&x::CreateGlyphCursor {
            cid: id,
            source_font: font,
            mask_font: font,
            source_char: char,
            mask_char: char + 1,
            fore_red: 0,
            fore_green: 0,
            fore_blue: 0,
            back_red: u16::MAX,
            back_green: u16::MAX,
            back_blue: u16::MAX,
        })?;

        Ok(id)
    }

    fn handle_events(context: &mut Context) {
        // TODO: Error handling
        while let Some(event) = context.connection.poll_for_event().unwrap() {
            Self::handle_event(event, context);
        }
    }

    fn handle_event(event: xcb::Event, context: &mut Context) {
        match event {
            xcb::Event::X(x::Event::ButtonPress(event)) => {
                let position = LogicalPosition {
                    x: event.event_x() as f64,
                    y: event.event_y() as f64,
                };

                if let Some(button) = Self::mouse_button_from_detail(event.detail()) {
                    (context.event_callback)(Event::MouseButtonDown {
                        button,
                        position,
                    });    
                } else if [4, 5].contains(&event.detail()) {
                    let delta_y = if event.detail() == 4 {
                        -1.0
                    } else {
                        1.0
                    };

                    (context.event_callback)(Event::MouseWheel {
                        position,
                        delta_x: 0.0,
                        delta_y,
                    });
                }
            }

            xcb::Event::X(x::Event::ButtonRelease(event)) => {
                let position = LogicalPosition {
                    x: event.event_x() as f64,
                    y: event.event_y() as f64,
                };

                if let Some(button) = Self::mouse_button_from_detail(event.detail()) {
                    (context.event_callback)(Event::MouseButtonUp {
                        button,
                        position,
                    });    
                }
            }

            xcb::Event::X(x::Event::KeyPress(event)) => {
                let keycode = xkb::Keycode::new(event.detail() as u32);
                let mut text = String::new();

                if let Some(text) = match context.xkb_state.get_keymap().key_get_name(keycode) {
                    Some("UP") => Some("\u{f700}"),
                    Some("DOWN") => Some("\u{f701}"),
                    Some("LEFT") => Some("\u{f702}"),
                    Some("RGHT") => Some("\u{f703}"),
                    _ => None,
                } {
                    let text = text.to_string();
                    (context.event_callback)(Event::KeyDown { text });
                }

                for keysym in context.xkb_state.key_get_syms(keycode) {
                    context.xkb_compose_state.feed(*keysym);
                    if context.xkb_compose_state.status() == xkb::Status::Composed {
                        // We're assuming here that a single key press can only generate one piece of text, is this true?
                        text = context.xkb_compose_state.utf8().unwrap();
                    }
                }

                if text.is_empty() {
                    text = context.xkb_state.key_get_utf8(keycode);
                }

                context.xkb_state.update_key(keycode, xkb::KeyDirection::Down);

                if !text.is_empty() {
                    (context.event_callback)(Event::KeyDown { text });
                }
            }

            xcb::Event::X(x::Event::KeyRelease(event)) => {
                let keycode = xkb::Keycode::new(event.detail() as u32);
                let text = context.xkb_state.key_get_utf8(keycode);
                context.xkb_state.update_key(keycode, xkb::KeyDirection::Up);
                
                if !text.is_empty() {
                    (context.event_callback)(Event::KeyUp { text });
                }
            }

            xcb::Event::X(x::Event::LeaveNotify(_)) => {
                (context.event_callback)(Event::MouseExited);
            }

            xcb::Event::X(x::Event::MotionNotify(event)) => {
                let position = LogicalPosition {
                    x: event.event_x() as f64,
                    y: event.event_y() as f64,
                };

                (context.event_callback)(Event::MouseMoved { position });
            }
            
            _ => {},
        }
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
        os_scale: f64,
        event_callback: Box<EventCallback>,
        window_builder: OsWindowBuilder,
    ) -> Result<(), Error>
    {
        let parent_window_id = match parent_window_handle {
            RawWindowHandle::Xlib(parent_window_handle) => parent_window_handle.window as u32,
            RawWindowHandle::Xcb(parent_window_handle) => parent_window_handle.window,
            _ => { return Err(Error::PlatformError("Not an X11 window".into())); }
        };
        
        let (event_sender, event_receiver) = mpsc::channel();
        std::thread::spawn({
            move || Self::window_thread(
                parent_window_id,
                window_attributes,
                os_scale,
                event_callback,
                event_sender,
                Box::new(move |os_window_handle| window_builder(os_window_handle))
            )
        });

        while let Ok(event) = event_receiver.recv() {
            match event {
                OsWindowEvent::Error(error) => return Err(error),
            }
        }

        Ok(())
    }

    fn set_cursor(&self, cursor: Cursor) {
        let cursor = match cursor {
            Cursor::None => self.cursor_none,
            Cursor::Arrow => self.cursor_arrow,
            Cursor::Copy => self.cursor_plus,
            Cursor::Crosshair => self.cursor_crosshair,
            Cursor::Help => self.cursor_question_arrow,
            Cursor::Move => self.cursor_fleur,
            Cursor::NoDrop => self.cursor_x,
            Cursor::NotAllowed => self.cursor_x,
            Cursor::Pointer => self.cursor_pointer,
            Cursor::Text => self.cursor_xterm,
            Cursor::Wait => self.cursor_watch,

            Cursor::ResizeNorth => self.cursor_top_side,
            Cursor::ResizeNorthEast => self.cursor_top_right_corner,
            Cursor::ResizeEast => self.cursor_right_side,
            Cursor::ResizeSouthEast => self.cursor_bottom_left_corner,
            Cursor::ResizeSouth => self.cursor_bottom_side,
            Cursor::ResizeSouthWest => self.cursor_bottom_right_corner,
            Cursor::ResizeWest => self.cursor_left_side,
            Cursor::ResizeNorthWest => self.cursor_top_left_corner,
            Cursor::ResizeEastWest => self.cursor_h_double_arrow,
            Cursor::ResizeNorthSouth => self.cursor_double_arrow,

            _ => self.cursor_arrow,
        };

        *self.new_cursor.lock().unwrap() = Some(cursor);
    }

    fn set_input_focus(&self, focus: bool) {
        *self.set_input_focus.lock().unwrap() = Some(focus);
    }
}

unsafe impl HasRawWindowHandle for OsWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Xlib(self.window_handle)
    }
}

unsafe impl HasRawDisplayHandle for OsWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Xlib(self.display_handle)
    }
}
