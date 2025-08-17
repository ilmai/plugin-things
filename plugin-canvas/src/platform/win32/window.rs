use std::{cell::RefCell, ffi::OsString, mem::{size_of, transmute}, num::NonZeroIsize, os::windows::prelude::OsStringExt, ptr::{null, null_mut}, rc::{Rc, Weak}, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc}, time::Duration};

use cursor_icon::CursorIcon;
use keyboard_types::Code;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle};
use uuid::Uuid;
use windows::{core::PCWSTR, Win32::UI::Input::KeyboardAndMouse::{VK_LWIN, VK_RWIN}};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::{Dwm::{DwmFlush, DwmIsCompositionEnabled}, Dxgi::{CreateDXGIFactory, IDXGIFactory, IDXGIOutput}, Gdi::{ClientToScreen, MonitorFromWindow, ScreenToClient, HBRUSH, MONITOR_DEFAULTTOPRIMARY}};
use windows::Win32::System::{Ole::{IDropTarget, OleInitialize, RegisterDragDrop, RevokeDragDrop}, Threading::GetCurrentThreadId};
use windows::Win32::UI::{Controls::WM_MOUSELEAVE, Input::KeyboardAndMouse::{GetAsyncKeyState, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_CONTROL, VK_MENU, VK_SHIFT}, WindowsAndMessaging::{CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, LoadCursorW, MoveWindow, PostMessageW, RegisterClassW, SendMessageW, SetCursor, SetCursorPos, SetWindowLongPtrW, SetWindowsHookExW, ShowCursor, UnhookWindowsHookEx, UnregisterClassW, CS_OWNDC, GWLP_USERDATA, HHOOK, HICON, IDC_ARROW, MOUSEHOOKSTRUCTEX, WH_MOUSE, WINDOW_EX_STYLE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WNDCLASSW, WS_CHILD, WS_VISIBLE}};

use crate::{dimensions::Size, error::Error, event::{Event, EventCallback, EventResponse, MouseButton}, keyboard::KeyboardModifiers, platform::{interface::OsWindowInterface, os_window_handle::OsWindowHandle}, window::WindowAttributes, LogicalPosition, LogicalSize, PhysicalPosition};

use super::{cursors::Cursors, drop_target::DropTarget, message_window::MessageWindow, to_wstr, version::is_windows10_or_greater, PLUGIN_HINSTANCE, WM_USER_CHAR, WM_USER_FRAME_TIMER, WM_USER_KEY_DOWN, WM_USER_KEY_UP};

pub struct OsWindow {
    window_class: u16,
    window_handle: Win32WindowHandle,
    hook_handle: HHOOK,
    event_callback: Box<EventCallback>,
    drop_target: RefCell<Option<Box<IDropTarget>>>,
    message_window: Arc<MessageWindow>,
    
    cursors: Cursors,
    buttons_down: AtomicUsize,

    running: Arc<AtomicBool>,
    moved: Arc<AtomicBool>,

    keyboard_modifiers: RefCell<KeyboardModifiers>,
}

impl OsWindow {
    pub(super) fn send_event(&self, event: Event) -> EventResponse {
        (self.event_callback)(event)
    }
    
    pub(super) fn hwnd(&self) -> HWND {
        HWND(self.window_handle.hwnd.get() as _)
    }

    fn button_down(&self, button: MouseButton, position: LogicalPosition) {
        if self.buttons_down.fetch_add(1, Ordering::Relaxed) == 0 {
            unsafe { SetCapture(self.hwnd()); }
        }

        self.send_event(Event::MouseButtonDown { button, position });
    }

    fn button_up(&self, button: MouseButton, position: LogicalPosition) {
        if self.buttons_down.fetch_sub(1, Ordering::Relaxed) == 1 {
            unsafe { SetCapture(HWND(null_mut())); }
        }

        self.send_event(Event::MouseButtonUp { button, position });    
    }

    fn logical_mouse_position(&self, lparam: LPARAM) -> LogicalPosition {
        let scale = self.os_scale();

        PhysicalPosition {
            x: (lparam.0 & 0xFFFF) as i16 as i32,
            y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
        }.to_logical(scale)
    }

    fn update_modifiers(&self) {
        let mut new_modifiers = KeyboardModifiers::empty();

        for (key, modifier) in [
            (VK_MENU, KeyboardModifiers::Alt),
            (VK_CONTROL, KeyboardModifiers::Control),
            (VK_LWIN, KeyboardModifiers::Meta),
            (VK_RWIN, KeyboardModifiers::Meta),
            (VK_SHIFT, KeyboardModifiers::Shift),
        ] {
            let pressed = unsafe { GetAsyncKeyState(key.0 as _) } != 0;
            new_modifiers.set(modifier, pressed);
        }

        let mut modifiers = self.keyboard_modifiers.borrow_mut();
        if new_modifiers != *modifiers {
            *modifiers = new_modifiers;
            self.send_event(Event::KeyboardModifiers { modifiers: new_modifiers });
        }
    }
}

impl OsWindowInterface for OsWindow {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
    ) -> Result<OsWindowHandle, Error> {
        let RawWindowHandle::Win32(parent_window_handle) = parent_window_handle else {
            return Err(Error::PlatformError("Not a win32 window".into()));
        };

        let class_name = to_wstr("plugin-canvas-".to_string() + &Uuid::new_v4().simple().to_string());
        let size = Size::with_logical_size(window_attributes.size, window_attributes.scale);

        let cursor = unsafe { LoadCursorW(None, IDC_ARROW).unwrap() };

        let window_class_attributes = WNDCLASSW {
            style: CS_OWNDC,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: PLUGIN_HINSTANCE.with(|hinstance| *hinstance),
            hIcon: HICON(null_mut()),
            hCursor: cursor,
            hbrBackground: HBRUSH(null_mut()),
            lpszMenuName: PCWSTR(null()),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };

        let window_class = unsafe { RegisterClassW(&window_class_attributes) };
        if window_class == 0 {
            return Err(Error::PlatformError("Failed to register window class".into()));
        }

        let hwnd = unsafe { CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(window_class as _),
            PCWSTR(null()),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            size.physical_size().width as i32,
            size.physical_size().height as i32,
            Some(HWND(parent_window_handle.hwnd.get() as _)),
            None,
            Some(PLUGIN_HINSTANCE.with(|hinstance| *hinstance)),
            None,
        ).unwrap() };

        let mut tracking_info = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        unsafe { TrackMouseEvent(&mut tracking_info).unwrap() };

        let window_handle = Win32WindowHandle::new(NonZeroIsize::new(hwnd.0 as _).unwrap());

        let running: Arc<AtomicBool> = Arc::new(true.into());
        let moved: Arc<AtomicBool> = Arc::new(false.into());

        let hook_handle = unsafe {
            SetWindowsHookExW(
                WH_MOUSE,
                Some(hook_proc),
                None,
                GetCurrentThreadId(),
            ).unwrap()
        };

        std::thread::spawn({
            let hwnd = hwnd.0 as usize;
            let running = running.clone();
            let moved = moved.clone();

            move || frame_pacing_thread(hwnd, running, moved)
        });

        let message_window = Arc::new(MessageWindow::new(hwnd).unwrap());

        std::thread::spawn({
            let message_window = message_window.clone();
            let running = running.clone();

            move || message_window.run(running)
        });

        let window = Rc::new(Self {
            window_class,
            window_handle,
            hook_handle,
            event_callback,
            drop_target: Default::default(),
            message_window,

            cursors: Cursors::new(),
            buttons_down: Default::default(),

            running,
            moved,

            keyboard_modifiers: Default::default(),
        });

        let drop_target: Box<IDropTarget> = Box::new(DropTarget::new(Rc::downgrade(&window)).into());

        unsafe {
            OleInitialize(None)?;
            RegisterDragDrop(hwnd, drop_target.as_ref())?;
        }

        *window.drop_target.borrow_mut() = Some(drop_target);

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Rc::downgrade(&window).into_raw() as _) };

        Ok(OsWindowHandle::new(window))
    }

    fn os_scale(&self) -> f64 {
        1.0
    }

    fn resized(&self, size: LogicalSize) {
        unsafe { MoveWindow(self.hwnd(), 0, 0, size.width as _, size.height as _, true).unwrap(); }
    }

    fn set_cursor(&self, cursor: Option<CursorIcon>) {
        if let Some(cursor) = cursor {
            let cursor = match cursor {
                CursorIcon::Default => self.cursors.arrow,
                CursorIcon::ContextMenu => self.cursors.arrow, // TODO
                CursorIcon::Help => self.cursors.help,
                CursorIcon::Pointer => self.cursors.hand,
                CursorIcon::Progress => self.cursors.app_starting,
                CursorIcon::Wait => self.cursors.wait,
                CursorIcon::Cell => self.cursors.cross,
                CursorIcon::Crosshair => self.cursors.cross,
                CursorIcon::Text => self.cursors.ibeam,
                CursorIcon::VerticalText => self.cursors.arrow, // TODO
                CursorIcon::Alias => self.cursors.arrow, // TODO
                CursorIcon::Copy => self.cursors.arrow, // TODO
                CursorIcon::Move => self.cursors.size_all,
                CursorIcon::NoDrop => self.cursors.no,
                CursorIcon::NotAllowed => self.cursors.no,
                CursorIcon::Grab => self.cursors.size_all, // TODO
                CursorIcon::Grabbing => self.cursors.size_all, // TODO
                CursorIcon::EResize => self.cursors.size_ew,
                CursorIcon::NResize => self.cursors.size_ns,
                CursorIcon::NeResize => self.cursors.size_nesw,
                CursorIcon::NwResize => self.cursors.size_nwse,
                CursorIcon::SResize => self.cursors.size_ns,
                CursorIcon::SeResize => self.cursors.size_nwse,
                CursorIcon::SwResize => self.cursors.size_nesw,
                CursorIcon::WResize => self.cursors.size_ew,
                CursorIcon::EwResize => self.cursors.size_ew,
                CursorIcon::NsResize => self.cursors.size_ns,
                CursorIcon::NeswResize => self.cursors.size_nesw,
                CursorIcon::NwseResize => self.cursors.size_nwse,
                CursorIcon::ColResize => self.cursors.size_ew, // TODO
                CursorIcon::RowResize => self.cursors.size_ns, // TODO
                CursorIcon::AllScroll => self.cursors.size_all,
                CursorIcon::ZoomIn => self.cursors.size_all, // TODO
                CursorIcon::ZoomOut => self.cursors.size_all, // TODO
                _ => todo!(),
            };
    
            unsafe {
                ShowCursor(true);
                SetCursor(Some(cursor));
            }
        } else {
            unsafe { ShowCursor(false); }
        }
    }

    fn set_input_focus(&self, focus: bool) {
        self.message_window.set_focus(focus);
    }

    fn warp_mouse(&self, position: LogicalPosition) {
        let scale = self.os_scale();
        let physical_position = position.to_physical(scale);

        let mut point = POINT {
            x: physical_position.x,
            y: physical_position.y,
        };

        unsafe {
            let result = ClientToScreen(self.hwnd(), &mut point);
            assert!(result.as_bool());

            SetCursorPos(point.x, point.y).unwrap();
        }
    }
    
    fn poll_events(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);

        unsafe {
            SetWindowLongPtrW(self.hwnd(), GWLP_USERDATA, 0);
            UnhookWindowsHookEx(self.hook_handle).unwrap();
            RevokeDragDrop(self.hwnd()).unwrap();
            DestroyWindow(self.hwnd()).unwrap();
            UnregisterClassW(PCWSTR(self.window_class as _), Some(PLUGIN_HINSTANCE.with(|hinstance| *hinstance))).unwrap();
        }
    }
}

impl HasWindowHandle for OsWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw_window_handle = RawWindowHandle::Win32(self.window_handle);
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw_window_handle) })
    }
}

impl HasDisplayHandle for OsWindow {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::windows())
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let window_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut OsWindow;
    if window_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let window_weak = unsafe { Weak::from_raw(window_ptr) };

    let result = if let Some(window) = window_weak.upgrade() {
        match msg {
            WM_LBUTTONDOWN => {
                window.update_modifiers();
                window.button_down(MouseButton::Left, window.logical_mouse_position(lparam));
                LRESULT(0)
            },
    
            WM_LBUTTONUP => {
                window.update_modifiers();
                window.button_up(MouseButton::Left, window.logical_mouse_position(lparam));
                LRESULT(0)
            },
    
            WM_MBUTTONDOWN => {
                window.update_modifiers();
                window.button_down(MouseButton::Middle, window.logical_mouse_position(lparam));
                LRESULT(0)
            },
    
            WM_MBUTTONUP => {
                window.update_modifiers();
                window.button_up(MouseButton::Middle, window.logical_mouse_position(lparam));
                LRESULT(0)
            },
    
            WM_RBUTTONDOWN => {
                window.update_modifiers();
                window.button_down(MouseButton::Right, window.logical_mouse_position(lparam));
                LRESULT(0)
            },
    
            WM_RBUTTONUP => {
                window.update_modifiers();
                window.button_up(MouseButton::Right, window.logical_mouse_position(lparam));
                LRESULT(0)
            },
    
            WM_MOVE => {
                window.moved.store(true, Ordering::Release);
                LRESULT(0)
            },
    
            WM_MOUSELEAVE => {
                window.send_event(Event::MouseExited);
                LRESULT(0)
            },
    
            WM_MOUSEMOVE => {
                window.update_modifiers();
                window.send_event(Event::MouseMoved { position: window.logical_mouse_position(lparam) });
                LRESULT(0)
            },
    
            WM_MOUSEWHEEL => {
                window.update_modifiers();

                let wheel_delta: i16 = u16::cast_signed((wparam.0 >> 16) as u16);
                let x: i16 = u16::cast_signed(((lparam.0 as usize) & 0xFFFF) as u16);
                let y: i16 = u16::cast_signed(((lparam.0 as usize) >> 16) as u16);
    
                let mut position = POINT { x: x as i32, y: y as i32 };
                let result = unsafe { ScreenToClient(hwnd, &mut position) };
                assert!(result.as_bool());
    
                window.send_event(Event::MouseWheel {
                    position: LogicalPosition { x: position.x as f64, y: position.y as f64 },
                    delta_x: 0.0,
                    delta_y: wheel_delta as f64 / 120.0,
                });
    
                LRESULT(0)
            },
    
            WM_USER_CHAR => {
                let string = OsString::from_wide(&[wparam.0 as _]);
                
                window.send_event(Event::KeyDown {
                    key_code: Code::Unidentified,
                    text: Some(string.to_string_lossy().to_string()),
                });

                LRESULT(0)
            },
    
            WM_USER_KEY_DOWN => {
                window.send_event(Event::KeyDown {
                    key_code: unsafe { transmute::<u8, keyboard_types::Code>(wparam.0 as u8) },
                    text: None,
                });

                LRESULT(0)
            },
    
            WM_USER_KEY_UP => {
                window.send_event(Event::KeyUp {
                    key_code: unsafe { transmute::<u8, keyboard_types::Code>(wparam.0 as u8) },
                    text: None,
                });

                LRESULT(0)
            },

            WM_USER_FRAME_TIMER => {
                window.update_modifiers();
                window.send_event(Event::Draw);

                LRESULT(0)
            },
    
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    } else {
        LRESULT(0)
    };

    // Leak the weak reference so it's not dropped
    let _ = window_weak.into_raw();

    result
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let mouse_hook_struct_ptr: *const MOUSEHOOKSTRUCTEX = lparam.0 as _;
    let mouse_hook_struct = unsafe { &*mouse_hook_struct_ptr };
    let hwnd = mouse_hook_struct.Base.hwnd;

    #[expect(clippy::single_match)]
    match wparam.0 as u32 {
        WM_MOUSEWHEEL => {
            let position = &mouse_hook_struct.Base.pt;
            let x: u16 = i16::cast_unsigned(position.x as i16);
            let y: u16 = i16::cast_unsigned(position.y as i16);

            // TODO: Convert modifiers            
            let wparam = WPARAM(mouse_hook_struct.mouseData as usize & 0xFFFF0000);            
            let lparam = LPARAM(usize::cast_signed(x as usize + ((y as usize) << 16)));
            unsafe { PostMessageW(Some(hwnd), WM_MOUSEWHEEL, wparam, lparam).unwrap() };
        },
        _ => {},
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn frame_pacing_thread(hwnd: usize, running: Arc<AtomicBool>, moved: Arc<AtomicBool>) {
    let hwnd = HWND(hwnd as _);
    let mut maybe_output: Option<IDXGIOutput> = None;

    while running.load(Ordering::Acquire) {
        if moved.swap(false, Ordering::AcqRel) {
            maybe_output = None;
        }

        unsafe {
            // If we're on Windows 10 or later, prefer using DXGI for frame pacing
            let waited = is_windows10_or_greater() && wait_for_vblank_dxgi(hwnd, &mut maybe_output);

            // Fall back to DWM
            let waited = waited || (DwmIsCompositionEnabled().unwrap_or_default().as_bool() && DwmFlush().is_ok());

            // Fall back to waiting
            if !waited {
                std::thread::sleep(Duration::from_millis(10));
            }

            // Send draw message
            SendMessageW(hwnd, WM_USER_FRAME_TIMER, None, None);
        }
    }
}

fn wait_for_vblank_dxgi(hwnd: HWND, maybe_output: &mut Option<IDXGIOutput>) -> bool {
    unsafe {
        if maybe_output.is_none() {
            let dxgi_factory = CreateDXGIFactory::<IDXGIFactory>().unwrap();
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        
            let mut adapter_index = 0;
    
            'outer: while let Ok(adapter) = dxgi_factory.EnumAdapters(adapter_index) {
                let mut output_index = 0;
                while let Ok(output) = adapter.EnumOutputs(output_index) {
                    let desc = output.GetDesc().unwrap();
                    if desc.Monitor == monitor {
                        *maybe_output = Some(output);
                        break 'outer;
                    }
    
                    output_index += 1;
                }
    
                adapter_index += 1;
            }
        }
    
        if let Some(output) = maybe_output.as_ref() {
            output.WaitForVBlank().unwrap();
            true    
        } else {
            false
        }
    }
}
