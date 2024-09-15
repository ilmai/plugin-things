/// SAFETY: Caller is responsible that ptrs has at least `count` pointers
pub unsafe fn any_null<T>(ptrs: *const *const T, count: usize) -> bool {
    let slice = unsafe { std::slice::from_raw_parts(ptrs, count) };
    slice.iter().any(|ptr| ptr.is_null())
}

/// SAFETY: Caller is responsible that ptrs has at least `count` pointers
pub unsafe fn any_null_mut<T>(ptrs: *mut *mut T, count: usize) -> bool {
    let slice = unsafe { std::slice::from_raw_parts(ptrs, count) };
    slice.iter().any(|ptr| ptr.is_null())
}
