#[derive(Debug)]
pub enum Error {
    PlatformError(String),
    #[cfg(target_os="linux")]
    XcbError(xcb::Error),
    #[cfg(target_os="linux")]
    XcbConnectionError(xcb::ConnError),
    #[cfg(target_os="linux")]
    XcbProtocolError(xcb::ProtocolError),
    #[cfg(target_os="windows")]
    WindowsError(windows::core::Error),
}

#[cfg(target_os="linux")]
impl From<xcb::Error> for Error {
    fn from(error: xcb::Error) -> Self {
        Self::XcbError(error)
    }
}

#[cfg(target_os="linux")]
impl From<xcb::ConnError> for Error {
    fn from(error: xcb::ConnError) -> Self {
        Self::XcbConnectionError(error)
    }
}

#[cfg(target_os="linux")]
impl From<xcb::ProtocolError> for Error {
    fn from(error: xcb::ProtocolError) -> Self {
        Self::XcbProtocolError(error)
    }
}

#[cfg(target_os="windows")]
impl From<windows::core::Error> for Error {
    fn from(error: windows::core::Error) -> Self {
        Self::WindowsError(error)
    }
}
