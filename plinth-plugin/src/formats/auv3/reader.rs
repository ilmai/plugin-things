use std::ffi::c_void;

pub struct Auv3Reader {
    context: *mut c_void,
    read: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *mut u8, usize) -> usize,
}

impl Auv3Reader {
    pub fn new(
        context: *mut c_void,
        read: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *mut u8, usize) -> usize,
    ) -> Self {
        Self {
            context,
            read,
        }
    }
}

impl std::io::Read for Auv3Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes_read = unsafe { (self.read)(self.context, buf.as_mut_ptr(), buf.len()) };
        Ok(bytes_read)
    }
}
