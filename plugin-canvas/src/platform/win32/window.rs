use std::{ptr::null, ffi::{OsString, c_void}, os::windows::prelude::OsStringExt, time::Duration, sync::{atomic::{AtomicBool, Ordering, AtomicUsize}, Arc}, rc::Rc, mem::{self, size_of}, cell::RefCell};

use cursor_icon::CursorIcon;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, WindowsDisplayHandle, RawDisplayHandle, Win32WindowHandle, HasRawDisplayHandle};
use uuid::Uuid;
use windows::{Win32::{UI::{WindowsAndMessaging::{WNDCLASSW, RegisterClassW, HICON, LoadCursorW, IDC_ARROW, CS_OWNDC, CreateWindowExW, WS_CHILD, WS_VISIBLE, HMENU, DefWindowProcW, PostMessageW, SetWindowLongPtrW, GWLP_USERDATA, GetWindowLongPtrW, UnregisterClassW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOVE, DestroyWindow, SetCursor, WM_MOUSEMOVE, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, SetWindowsHookExW, WH_MOUSE, CallNextHookEx, HHOOK, WM_MOUSEWHEEL, MOUSEHOOKSTRUCTEX, UnhookWindowsHookEx, ShowCursor, WINDOW_EX_STYLE, SendMessageW, SetCursorPos}, Input::KeyboardAndMouse::{SetCapture, TRACKMOUSEEVENT, TME_LEAVE, TrackMouseEvent}, Controls::WM_MOUSELEAVE}, Foundation::{HWND, WPARAM, LPARAM, LRESULT, HINSTANCE, POINT}, Graphics::{Gdi::{HBRUSH, MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY, ScreenToClient, ClientToScreen}, Dxgi::{CreateDXGIFactory, IDXGIFactory, DXGI_OUTPUT_DESC, IDXGIOutput}, Dwm::{DwmIsCompositionEnabled, DwmFlush}}, System::{Threading::GetCurrentThreadId, Ole::{OleInitialize, RegisterDragDrop, IDropTarget, RevokeDragDrop}}}, core::PCWSTR};

use crate::{error::Error, platform::interface::{OsWindowInterface, OsWindowHandle, OsWindowBuilder}, event::{Event, MouseButton, EventCallback, EventResponse}, window::WindowAttributes, dimensions::Size, LogicalPosition, PhysicalPosition};

use super::{PLUGIN_HINSTANCE, to_wstr, message_window::MessageWindow, cursors::Cursors, WM_USER_KEY_DOWN, WM_USER_FRAME_TIMER, version::is_windows10_or_greater, drop_target::DropTarget, message_hook::MessageHook, WM_USER_KEY_UP};

pub struct OsWindow {
    window_attributes: WindowAttributes,
    
    window_class: u16,
    window_handle: Win32WindowHandle,
    hook_handle: HHOOK,
    event_callback: Box<EventCallback>,
    drop_target: RefCell<Option<Box<IDropTarget>>>,
    message_window: Arc<MessageWindow>,
    
    cursors: Cursors,
    buttons_down: AtomicUsize,

    moved: Arc<AtomicBool>,
}

impl OsWindow {
    pub(super) fn window_attributes(&self) -> &WindowAttributes {
        &self.window_attributes
    }

    pub(super) fn send_event(&self, event: Event) -> EventResponse {
        (self.event_callback)(event)
    }
    
    pub(super) fn hinstance(&self) -> HINSTANCE {
        HINSTANCE(self.window_handle.hinstance as isize)
    }

    pub(super) fn hwnd(&self) -> HWND {
        HWND(self.window_handle.hwnd as isize)
    }

    fn button_down(&self, button: MouseButton, position: LogicalPosition) {
        if self.buttons_down.fetch_add(1, Ordering::Relaxed) == 0 {
            unsafe { SetCapture(self.hwnd()); }
        }

        self.send_event(Event::MouseButtonDown { button, position });
    }

    fn button_up(&self, button: MouseButton, position: LogicalPosition) {
        if self.buttons_down.fetch_sub(1, Ordering::Relaxed) == 1 {
            unsafe { SetCapture(HWND(0)); }
        }

        self.send_event(Event::MouseButtonUp { button, position });    
    }

    fn logical_mouse_position(&self, lparam: LPARAM) -> LogicalPosition {
        let user_scale: f64 = self.window_attributes.user_scale.into();

        PhysicalPosition {
            x: (lparam.0 & 0xFFFF) as i16 as i32,
            y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
        }.to_logical(user_scale)
    }
}

impl OsWindowInterface for OsWindow {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
        window_builder: OsWindowBuilder,
    ) -> Result<(), Error> {
        let RawWindowHandle::Win32(parent_window_handle) = parent_window_handle else {
            return Err(Error::PlatformError("Not a win32 window".into()));
        };

        let class_name = to_wstr("plugin-canvas-".to_string() + &Uuid::new_v4().simple().to_string());
        let size = Size::with_logical_size(window_attributes.size, window_attributes.user_scale);

        let cursor = unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).unwrap() };

        let window_class_attributes = WNDCLASSW {
            style: CS_OWNDC,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: *PLUGIN_HINSTANCE,
            hIcon: HICON(0),
            hCursor: cursor,
            hbrBackground: HBRUSH(0),
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
            HWND(parent_window_handle.hwnd as _),
            HMENU(0),
            *PLUGIN_HINSTANCE,
            None,
        ) };

        let mut tracking_info = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        unsafe { TrackMouseEvent(&mut tracking_info).unwrap() };

        let mut window_handle = Win32WindowHandle::empty();
        window_handle.hinstance = PLUGIN_HINSTANCE.0 as *mut c_void;
        window_handle.hwnd = hwnd.0 as *mut c_void;

        let raw_window_handle = RawWindowHandle::Win32(window_handle);
        let raw_display_handle = RawDisplayHandle::Windows(WindowsDisplayHandle::empty());

        let moved: Arc<AtomicBool> = Default::default();

        let hook_handle = unsafe {
            SetWindowsHookExW(
                WH_MOUSE,
                Some(hook_proc),
                HINSTANCE(0),
                GetCurrentThreadId(),
            ).unwrap()
        };

        std::thread::spawn({
            let moved = moved.clone();
            move || frame_pacing_thread(hwnd, moved)
        });

        MessageHook::install(hwnd);

        let message_window = Arc::new(MessageWindow::new(hwnd).unwrap());

        std::thread::spawn({
            let message_window = message_window.clone();
            move || message_window.run()
        });

        let window = Rc::new(Self {
            window_attributes,

            window_class,
            window_handle,
            hook_handle,
            event_callback,
            drop_target: Default::default(),
            message_window,

            cursors: Cursors::new(),
            buttons_down: Default::default(),

            moved,
        });

        let drop_target: Box<IDropTarget> = Box::new(DropTarget::new(window.clone()).into());

        unsafe {
            OleInitialize(None)?;
            RegisterDragDrop(hwnd, drop_target.as_ref())?;
        }

        *window.drop_target.borrow_mut() = Some(drop_target);

        let window_ptr = Rc::into_raw(window);
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_ptr as _) };
        
        let window = unsafe { Rc::from_raw(window_ptr) };

        window_builder(OsWindowHandle::new(raw_window_handle, raw_display_handle, window));

        Ok(())
    }

    fn set_cursor(&self, cursor: Option<CursorIcon>) {
        if let Some(cursor) = cursor {
            let cursor = match cursor {
                CursorIcon::Default => self.cursors.arrow,
                CursorIcon::ContextMenu => self.cursors.arrow, // TODO
                CursorIcon::Help => self.cursors.arrow, // TODO
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
                SetCursor(cursor);
            }
        } else {
            unsafe { ShowCursor(false); }
        }
    }

    fn set_input_focus(&self, focus: bool) {
        self.message_window.set_focus(focus);
    }

    fn warp_mouse(&self, position: LogicalPosition) {
        let user_scale: f64 = self.window_attributes.user_scale.into();
        let physical_position = position.to_physical(user_scale);

        let mut point = POINT {
            x: physical_position.x,
            y: physical_position.y,
        };

        unsafe {
            ClientToScreen(self.hwnd(), &mut point);
            SetCursorPos(point.x, point.y).unwrap();
        }
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        unsafe {
            MessageHook::uninstall();

            RevokeDragDrop(self.hwnd()).unwrap();
            SetWindowLongPtrW(self.hwnd(), GWLP_USERDATA, 0);
            UnhookWindowsHookEx(self.hook_handle).unwrap();
            DestroyWindow(self.hwnd()).unwrap();
            UnregisterClassW(PCWSTR(self.window_class as _), self.hinstance()).unwrap();
        }
    }
}

unsafe impl HasRawWindowHandle for OsWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::Win32(self.window_handle)
    }
}

unsafe impl HasRawDisplayHandle for OsWindow {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Windows(WindowsDisplayHandle::empty())
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OsWindow;
    if window_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let window = unsafe { &*window_ptr };

    match msg {
        WM_LBUTTONDOWN => {
            window.button_down(MouseButton::Left, window.logical_mouse_position(lparam));
            LRESULT(0)
        },

        WM_LBUTTONUP => {
            window.button_up(MouseButton::Left, window.logical_mouse_position(lparam));
            LRESULT(0)
        },

        WM_MBUTTONDOWN => {
            window.button_down(MouseButton::Middle, window.logical_mouse_position(lparam));
            LRESULT(0)
        },

        WM_MBUTTONUP => {
            window.button_up(MouseButton::Middle, window.logical_mouse_position(lparam));
            LRESULT(0)
        },

        WM_RBUTTONDOWN => {
            window.button_down(MouseButton::Right, window.logical_mouse_position(lparam));
            LRESULT(0)
        },

        WM_RBUTTONUP => {
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
            window.send_event(Event::MouseMoved { position: window.logical_mouse_position(lparam) });
            LRESULT(0)
        },

        WM_MOUSEWHEEL => {
            let wheel_delta: i16 = mem::transmute((wparam.0 >> 16) as u16);
            let x: i16 = mem::transmute(((lparam.0 as usize) & 0xFFFF) as u16);
            let y: i16 = mem::transmute(((lparam.0 as usize) >> 16) as u16);

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

        WM_USER_KEY_DOWN => {
            let string = OsString::from_wide(&[wparam.0 as u16]);
            window.send_event(Event::KeyDown { text: string.to_string_lossy().to_string() });
            LRESULT(0)
        },

        WM_USER_KEY_UP => {
            let string = OsString::from_wide(&[wparam.0 as u16]);
            window.send_event(Event::KeyUp { text: string.to_string_lossy().to_string() });
            LRESULT(0)
        },

        WM_USER_FRAME_TIMER => {
            window.send_event(Event::Draw);
            LRESULT(0)
        },

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }   
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(HHOOK(0), code, wparam, lparam);
    }

    let mouse_hook_struct_ptr: *const MOUSEHOOKSTRUCTEX = lparam.0 as _;
    let mouse_hook_struct = unsafe { &*mouse_hook_struct_ptr };
    let hwnd = mouse_hook_struct.Base.hwnd;

    match wparam.0 as u32 {
        WM_MOUSEWHEEL => {
            let position = &mouse_hook_struct.Base.pt;
            let x: u16 = mem::transmute(position.x as i16);
            let y: u16 = mem::transmute(position.y as i16);

            // TODO: Convert modifiers            
            let wparam = WPARAM(mouse_hook_struct.mouseData as usize & 0xFFFF0000);            
            let lparam = LPARAM(mem::transmute(x as usize + (y as usize) << 16));
            PostMessageW(hwnd, WM_MOUSEWHEEL, wparam, lparam).unwrap();
        },
        _ => {},
    }

    CallNextHookEx(HHOOK(0), code, wparam, lparam)
}

fn frame_pacing_thread(hwnd: HWND, moved: Arc<AtomicBool>) {
    let mut maybe_output: Option<IDXGIOutput> = None;

    loop {
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
            SendMessageW(hwnd, WM_USER_FRAME_TIMER, WPARAM(0), LPARAM(0));
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
                    let mut desc = DXGI_OUTPUT_DESC {
                        ..std::mem::zeroed()
                    };
    
                    output.GetDesc(&mut desc as _).unwrap();
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
