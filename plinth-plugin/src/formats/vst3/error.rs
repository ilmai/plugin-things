use num_derive::FromPrimitive;

#[repr(i32)]
#[derive(Debug, FromPrimitive, thiserror::Error)]
pub enum Error {
    #[error("No interface")]
    NoInterface = -1,
    #[error("False")]
    ResultFalse = 1,
    #[error("Invalid argument")]
    InvalidArgument = 2,
    #[error("Not implemented")]
    NotImplemented = 3,
    #[error("Internal error")]
    InternalError = 4,
    #[error("Not initialized")]
    NotInitialized = 5,
    #[error("Out of memory")]
    OutOfMemory = 6,
}
