#[derive(Debug)]
pub enum Error {
    ParameterIdError,
    ParameterRangeError,
    SerializationError,
    IoError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}
