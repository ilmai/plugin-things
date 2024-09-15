use std::ffi::c_void;

pub struct Auv3Writer {
    context: *mut c_void,
    write: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *const u8, usize) -> usize,
}

impl Auv3Writer {
    pub fn new(
        context: *mut c_void,
        write: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *const u8, usize) -> usize,
    ) -> Self {
        Self {
            context,
            write,
        }
    }
}

impl std::io::Write for Auv3Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bytes_written = unsafe { (self.write)(self.context, buf.as_ptr(), buf.len()) };
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
