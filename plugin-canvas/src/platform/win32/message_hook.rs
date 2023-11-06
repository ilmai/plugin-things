use std::cell::RefCell;

use windows::Win32::{UI::{WindowsAndMessaging::{HHOOK, SetWindowsHookExW, WH_GETMESSAGE, CallNextHookEx, UnhookWindowsHookEx, HC_ACTION, WM_KEYDOWN, PostMessageW, WM_KEYUP, PM_REMOVE, MSG}, Input::KeyboardAndMouse::VIRTUAL_KEY}, Foundation::{LRESULT, WPARAM, LPARAM, HINSTANCE, HWND}, System::Threading::GetCurrentThreadId};

use super::{key_codes::virtual_key_to_char, WM_USER_KEY_DOWN, WM_USER_KEY_UP};

thread_local! {
    static MESSAGE_HOOK: RefCell<Option<MessageHook>> = RefCell::new(None);
}

pub struct MessageHook {
    handle: HHOOK,
    hwnd: HWND,
}

impl MessageHook {
    pub(super) fn install(hwnd: HWND) {
        let handle = unsafe { SetWindowsHookExW(WH_GETMESSAGE, Some(get_msg_proc), HINSTANCE(0), GetCurrentThreadId()).unwrap() };

        let hook = Self {
            handle,
            hwnd,
        };

        MESSAGE_HOOK.set(Some(hook));
    }

    pub(super) fn uninstall() {
        let hook = MESSAGE_HOOK.take().unwrap();
        unsafe { UnhookWindowsHookEx(hook.handle).unwrap() };
    }
}

unsafe extern "system" fn get_msg_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {    
    if code == HC_ACTION as i32 && wparam.0 == PM_REMOVE.0 as usize {
        let msg = unsafe { &*(lparam.0 as *const MSG) };
        let hwnd = MESSAGE_HOOK.with_borrow(|hook| hook.as_ref().unwrap().hwnd);

        match msg.message {
            WM_KEYDOWN => {
                if let Some(character) = virtual_key_to_char(VIRTUAL_KEY(msg.wParam.0 as u16)) {
                    PostMessageW(hwnd, WM_USER_KEY_DOWN, WPARAM(character), LPARAM(0)).unwrap();
                }
            }
    
            WM_KEYUP => {
                if let Some(character) = virtual_key_to_char(VIRTUAL_KEY(msg.wParam.0 as u16)) {
                    PostMessageW(hwnd, WM_USER_KEY_UP, WPARAM(character), LPARAM(0)).unwrap();
                }
            }
    
            _ => {},
        }
    }

    CallNextHookEx(HHOOK(0), code, wparam, lparam)
}
