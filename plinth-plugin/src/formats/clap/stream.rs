use std::io::{Read, Write};

use clap_sys::stream::{clap_istream, clap_ostream};

pub struct InputStream {
    raw: *const clap_istream,
}

impl InputStream {
    pub fn new(raw: *const clap_istream) -> Self {
        Self {
            raw,
        }
    }
}

impl Read for InputStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut index = 0;

        while index < buf.len() {
            let remaining_bytes = buf.len() - index;

            let bytes_read = unsafe {
                let istream = &*self.raw;
                (istream.read.unwrap())(self.raw, buf.as_ptr().add(index) as _, remaining_bytes as _)
            };

            if bytes_read < 0 {
                return Err(std::io::Error::other("CLAP stream read error"));
            }
            if bytes_read == 0 {
                break;
            }

            index += bytes_read as usize;
        }


        Ok(index)
    }
}

pub struct OutputStream {
    raw: *const clap_ostream,
}

impl OutputStream {
    pub fn new(raw: *const clap_ostream) -> Self {
        Self {
            raw,
        }
    }
}

impl Write for OutputStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut index = 0;

        while index < buf.len() {
            let remaining_bytes = buf.len() - index;

            let bytes_written = unsafe {
                let ostream = &*self.raw;
                (ostream.write.unwrap())(self.raw, buf.as_ptr().add(index) as _, remaining_bytes as _)
            };    

            if bytes_written < 0 {
                return Err(std::io::Error::other("CLAP stream write error"));
            }
            if bytes_written == 0 {
                break;
            }

            index += bytes_written as usize;
        }

        Ok(index)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
