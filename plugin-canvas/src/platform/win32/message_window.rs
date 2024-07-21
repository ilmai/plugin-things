use std::{mem, ptr::{null, null_mut}};

use uuid::Uuid;
use windows::{Win32::{UI::{WindowsAndMessaging::{WNDCLASSW, CS_OWNDC, DefWindowProcW, HICON, HCURSOR, RegisterClassW, CreateWindowExW, WS_EX_NOACTIVATE, HMENU, GetMessageW, TranslateMessage, DispatchMessageW, WM_CHAR, PostMessageW, SetWindowLongPtrW, GWLP_USERDATA, GetWindowLongPtrW, DestroyWindow, UnregisterClassW, WS_CHILD, WM_KEYDOWN, WM_KEYUP}, Input::KeyboardAndMouse::{SetFocus, VIRTUAL_KEY}}, Graphics::Gdi::HBRUSH, Foundation::{HWND, WPARAM, LPARAM, LRESULT, BOOL}}, core::PCWSTR};

use crate::error::Error;

use super::{to_wstr, PLUGIN_HINSTANCE, WM_USER_KEY_DOWN, key_codes::virtual_key_to_char, WM_USER_KEY_UP};

pub struct MessageWindow {
    hwnd: usize,
    main_window_hwnd: usize,
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
            hInstance: PLUGIN_HINSTANCE.with(|hinstance| hinstance.clone()),
            hIcon: HICON(null_mut()),
            hCursor: HCURSOR(null_mut()),
            hbrBackground: HBRUSH(null_mut()),
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
            HMENU(null_mut()),
            PLUGIN_HINSTANCE.with(|hinstance| hinstance.clone()),
            None,
        ).unwrap() };

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, main_window_hwnd.0 as _) };

        Ok(Self {
            hwnd: hwnd.0 as _,
            main_window_hwnd: main_window_hwnd.0 as _,
            window_class,
        })
    }

    pub fn run(&self) {
        unsafe {
            let hwnd = HWND(self.hwnd as _);
            let mut msg = mem::zeroed();

            loop {
                match GetMessageW(&mut msg, hwnd, 0, 0) {
                    BOOL(-1) => {
                        panic!()
                    }

                    BOOL(0) => {
                        return;
                    }

                    _ => {}
                }

                // We can ignore the return value
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }    
        }
    }

    pub fn set_focus(&self, focus: bool) {
        let hwnd = HWND(if focus {
            self.hwnd
        } else {
            self.main_window_hwnd
        } as _);

        unsafe { SetFocus(hwnd).unwrap(); }
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(HWND(self.hwnd as _)).unwrap();
            UnregisterClassW(PCWSTR(self.window_class as _), PLUGIN_HINSTANCE.with(|hinstance| hinstance.clone())).unwrap();
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let main_window_hwnd = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as _) };

    match msg {
        WM_CHAR => {
            PostMessageW(main_window_hwnd, WM_USER_KEY_DOWN, wparam, lparam).unwrap();
            LRESULT(0)
        },

        WM_KEYDOWN => {
            if let Some(character) = virtual_key_to_char(VIRTUAL_KEY(wparam.0 as u16)) {
                PostMessageW(main_window_hwnd, WM_USER_KEY_DOWN, WPARAM(character), LPARAM(0)).unwrap();
            }
            
            LRESULT(0)
        }

        WM_KEYUP => {
            if let Some(character) = virtual_key_to_char(VIRTUAL_KEY(wparam.0 as u16)) {
                PostMessageW(main_window_hwnd, WM_USER_KEY_UP, WPARAM(character), LPARAM(0)).unwrap();
            }
            
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
