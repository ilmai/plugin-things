use std::cell::UnsafeCell;
use std::sync::{Arc, Weak};
use std::thread::ThreadId;

use crate::platform::window::OsWindow;

pub(crate) struct OsWindowHandle {
    os_window: Arc<UnsafeCell<OsWindow>>,
    thread: ThreadId,
}

impl OsWindowHandle {
    // OsWindow is not Send + Sync but we are
    #[expect(clippy::arc_with_non_send_sync)]
    pub(super) fn new(os_window: OsWindow) -> Self {
        Self {
            os_window: Arc::new(os_window.into()),
            thread: std::thread::current().id(),
        }
    }

    pub fn as_window(&self) -> &OsWindow {
        assert_eq!(std::thread::current().id(), self.thread);

        // SAFETY: It's checked above that we're not on another thread
        unsafe { &*self.os_window.get() }
    }

    pub(super) fn downgrade(&self) -> OsWindowWeak {
        OsWindowWeak {
            os_window: Arc::downgrade(&self.os_window),
            thread: self.thread,
        }
    }
}

unsafe impl Send for OsWindowHandle {}
unsafe impl Sync for OsWindowHandle {}

#[derive(Clone)]
pub(super) struct OsWindowWeak {
    os_window: Weak<UnsafeCell<OsWindow>>,
    thread: ThreadId,
}

impl OsWindowWeak {
    pub fn upgrade(&self) -> Option<OsWindowHandle> {
        let os_window = self.os_window.upgrade()?;
        
        Some(OsWindowHandle {
            os_window,
            thread: self.thread,
        })
    }

    pub fn into_raw(self) -> *const UnsafeCell<OsWindow> {
        Weak::into_raw(self.os_window)
    }

    /// SAFETY:
    /// Must be called from the same thread as into_raw() was called from
    pub unsafe fn from_raw(ptr: *const UnsafeCell<OsWindow>) -> Self {
        Self {
            os_window: unsafe { Weak::from_raw(ptr) },
            thread: std::thread::current().id(),
        }
    }
}
