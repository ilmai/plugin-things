use std::io::{Read, Write};

use num_traits::FromPrimitive;
use vst3::{ComRef, Steinberg::{kResultOk, IBStream, IBStreamTrait}};

use super::Error;

pub struct Stream<'a> {
    raw: ComRef<'a, IBStream>,
}

impl<'a> Stream<'a> {
    pub fn new(raw: *mut IBStream) -> Option<Self> {
        let Some(raw) = (unsafe { ComRef::from_raw(raw) }) else {
            return None;
        };

        Some(Self {
            raw,
        })
    }
}

impl<'a> Read for Stream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut total_bytes_read = 0;
        while total_bytes_read < buf.len() {
            let mut bytes_read = 0;
            let result = unsafe { self.raw.read(buf.as_mut_ptr() as _, buf.len() as _, &mut bytes_read) };
            
            if bytes_read <= 0 {
                break;
            }

            if result == kResultOk {
                total_bytes_read += bytes_read as usize;
            } else {
                return Err(std::io::Error::other(Error::from_i32(result).unwrap()));
            }
        }

        Ok(total_bytes_read)
    }
}

impl<'a> Write for Stream<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut total_bytes_written = 0;
        
        while total_bytes_written < buf.len() {
            let mut bytes_written = 0;
            let result = unsafe { self.raw.write(buf.as_ptr() as _, buf.len() as _, &mut bytes_written) };

            if bytes_written <= 0 {
                break;
            }

            if result == kResultOk {
                total_bytes_written += bytes_written as usize;
            } else {
                return Err(std::io::Error::other(Error::from_i32(result).unwrap()));
            }
        }

        Ok(total_bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
