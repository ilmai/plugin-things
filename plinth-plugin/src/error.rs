use crate::ParameterId;

#[derive(Debug)]
pub enum Error {
    ParameterIdError(ParameterId),
    ParameterRangeError,
    SerializationError,
    IoError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}
