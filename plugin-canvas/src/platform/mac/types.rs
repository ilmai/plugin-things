use std::{sync::atomic::AtomicPtr, ffi::c_void, ops::Deref};

use objc2::{Encode, Encoding};

// Workaround for an objc2 limitation: https://github.com/madsmtm/objc2/issues/511
#[repr(transparent)]
#[derive(Default)]
pub struct AtomicVoidPtr(AtomicPtr<c_void>);

unsafe impl Encode for AtomicVoidPtr {
    const ENCODING: Encoding = Encoding::Atomic(&Encoding::Pointer(&Encoding::Void));
}

impl Deref for AtomicVoidPtr {
    type Target = AtomicPtr<c_void>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
