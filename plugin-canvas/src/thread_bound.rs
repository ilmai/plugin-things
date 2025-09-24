use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::thread::ThreadId;

pub struct ThreadBound<T> {
    inner: UnsafeCell<T>,
    thread: ThreadId,
}

impl<T> ThreadBound<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: inner.into(),
            thread: std::thread::current().id(),
        }
    }

    fn assert_thread(&self) {
        assert_eq!(std::thread::current().id(), self.thread, "Tried to access inner ThreadBound value from an invalid thread");
    }
}

impl<T> Deref for ThreadBound<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.assert_thread();
        unsafe { &*self.inner.get() }
    }
}

impl<T> DerefMut for ThreadBound<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.assert_thread();
        self.inner.get_mut()
    }
}

impl<T> From<T> for ThreadBound<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

unsafe impl<T> Send for ThreadBound<T> {}
unsafe impl<T> Sync for ThreadBound<T> {}
