use x11rb::x11_utils;

#[derive(Debug)]
pub enum Error {
    PlatformError(String),
    #[cfg(target_os="linux")]
    X11ConnectError(x11rb::errors::ConnectError),
    #[cfg(target_os="linux")]
    X11ConnectionError(x11rb::errors::ConnectionError),
    #[cfg(target_os="linux")]
    X11Error(x11_utils::X11Error),
    #[cfg(target_os="linux")]
    X11IdsExhausted,
    #[cfg(target_os="linux")]
    XcbConnectionError(xcb::ConnError),
    #[cfg(target_os="windows")]
    WindowsError(windows::core::Error),
}

#[cfg(target_os="linux")]
impl From<x11rb::errors::ConnectError> for Error {
    fn from(error: x11rb::errors::ConnectError) -> Self {
        Self::X11ConnectError(error)
    }
}

#[cfg(target_os="linux")]
impl From<x11rb::errors::ConnectionError> for Error {
    fn from(error: x11rb::errors::ConnectionError) -> Self {
        Self::X11ConnectionError(error)
    }
}

#[cfg(target_os="linux")]
impl From<x11rb::errors::ReplyOrIdError> for Error {
    fn from(error: x11rb::errors::ReplyOrIdError) -> Self {
        match error {
            x11rb::errors::ReplyOrIdError::IdsExhausted => Self::X11IdsExhausted,
            x11rb::errors::ReplyOrIdError::ConnectionError(error) => Self::X11ConnectionError(error),
            x11rb::errors::ReplyOrIdError::X11Error(error) => Self::X11Error(error),
        }
    }
}

#[cfg(target_os="linux")]
impl From<xcb::ConnError> for Error {
    fn from(error: xcb::ConnError) -> Self {
        Self::XcbConnectionError(error)
    }
}

#[cfg(target_os="windows")]
impl From<windows::core::Error> for Error {
    fn from(error: windows::core::Error) -> Self {
        Self::WindowsError(error)
    }
}
