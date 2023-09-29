use std::{ptr::null, mem};

use uuid::Uuid;
use windows::{Win32::{UI::{WindowsAndMessaging::{WNDCLASSW, CS_OWNDC, DefWindowProcW, HICON, HCURSOR, RegisterClassW, CreateWindowExW, WS_EX_NOACTIVATE, HMENU, GetMessageW, TranslateMessage, DispatchMessageW, WM_CHAR, PostMessageW, SetWindowLongPtrW, GWLP_USERDATA, GetWindowLongPtrW, DestroyWindow, UnregisterClassW, WS_CHILD, WM_KEYDOWN}, Input::KeyboardAndMouse::{SetFocus, VK_LEFT, VK_DELETE, VIRTUAL_KEY, VK_UP, VK_DOWN, VK_RIGHT}}, Graphics::Gdi::HBRUSH, Foundation::{HWND, WPARAM, LPARAM, LRESULT, BOOL}}, core::PCWSTR};

use crate::error::Error;

use super::{to_wstr, PLUGIN_HINSTANCE, WM_USER_CHAR};

pub struct MessageWindow {
    hwnd: HWND,
    main_window_hwnd: HWND,
    window_class: u16,
}

impl MessageWindow {
    pub fn new(main_window_hwnd: HWND) -> Result<Self, Error> {
        let class_name = to_wstr("plugin-canvas-message-window-".to_string() + &Uuid::new_v4().simple().to_string());
        let window_name = to_wstr("Message window");

        let window_class_attributes = WNDCLASSW {
            style: CS_OWNDC,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: *PLUGIN_HINSTANCE,
            hIcon: HICON(0),
            hCursor: HCURSOR(0),
            hbrBackground: HBRUSH(0),
            lpszMenuName: PCWSTR(null()),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };

        let window_class = unsafe { RegisterClassW(&window_class_attributes) };
        if window_class == 0 {
            return Err(Error::PlatformError("Failed to register window class".into()));
        }

        let hwnd = unsafe { CreateWindowExW(
            WS_EX_NOACTIVATE,
            PCWSTR(window_class as _),
            PCWSTR(window_name.as_ptr() as _),
            WS_CHILD,
            0,
            0,
            0,
            0,
            main_window_hwnd,
            HMENU(0),
            *PLUGIN_HINSTANCE,
            None,
        ) };

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, main_window_hwnd.0) };

        Ok(Self {
            hwnd,
            main_window_hwnd,
            window_class,
        })
    }

    pub fn run(&self) {
        unsafe {
            let mut msg = mem::zeroed();

            loop {
                match GetMessageW(&mut msg, self.hwnd, 0, 0) {
                    BOOL(-1) => {
                        panic!()
                    }

                    BOOL(0) => {
                        return;
                    }

                    _ => {}
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }    
        }
    }

    pub fn set_focus(&self, focus: bool) {
        let hwnd = if focus {
            self.hwnd
        } else {
            self.main_window_hwnd
        };

        unsafe { SetFocus(hwnd); }
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd).unwrap();
            UnregisterClassW(PCWSTR(self.window_class as _), *PLUGIN_HINSTANCE).unwrap();
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let main_window_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA)) };

    match msg {
        WM_CHAR => {
            PostMessageW(main_window_hwnd, WM_USER_CHAR, wparam, lparam).unwrap();
            LRESULT(0)
        },

        WM_KEYDOWN => {
            if let Some(character) = match VIRTUAL_KEY(wparam.0 as u16) {
                VK_DELETE   => Some(0x7f),
                VK_UP       => Some(0xf700),
                VK_DOWN     => Some(0xf701),
                VK_LEFT     => Some(0xf702),
                VK_RIGHT    => Some(0xf703),
                _ => None,
            } {
                PostMessageW(main_window_hwnd, WM_USER_CHAR, WPARAM(character), LPARAM(0)).unwrap();
            }
            
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
