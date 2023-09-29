pub struct RawWindowHandleAdapter(raw_window_handle_0_4::RawWindowHandle);

impl From<raw_window_handle_0_4::RawWindowHandle> for RawWindowHandleAdapter {
    fn from(handle: raw_window_handle_0_4::RawWindowHandle) -> Self {
        Self(handle)
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for RawWindowHandleAdapter {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        match self.0 {
            raw_window_handle_0_4::RawWindowHandle::AppKit(raw_window_handle_0_4::AppKitHandle { ns_window, ns_view, .. }) => {
                let mut window_handle = raw_window_handle::AppKitWindowHandle::empty();
                window_handle.ns_window = ns_window;
                window_handle.ns_view = ns_view;

                raw_window_handle::RawWindowHandle::AppKit(window_handle)
            },

            raw_window_handle_0_4::RawWindowHandle::Win32(raw_window_handle_0_4::Win32Handle { hwnd, hinstance, .. }) => {
                let mut window_handle = raw_window_handle::Win32WindowHandle::empty();
                window_handle.hwnd = hwnd;
                window_handle.hinstance = hinstance;

                raw_window_handle::RawWindowHandle::Win32(window_handle)
            }

            raw_window_handle_0_4::RawWindowHandle::Xcb(raw_window_handle_0_4::XcbHandle { window, visual_id, .. }) =>  {
                let mut window_handle = raw_window_handle::XcbWindowHandle::empty();
                window_handle.window = window;
                window_handle.visual_id = visual_id;

                raw_window_handle::RawWindowHandle::Xcb(window_handle)
            }

            _ => unimplemented!()
        }
    }
}

unsafe impl raw_window_handle::HasRawDisplayHandle for RawWindowHandleAdapter {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        match self.0 {
            raw_window_handle_0_4::RawWindowHandle::AppKit(_) => {
                raw_window_handle::RawDisplayHandle::AppKit(raw_window_handle::AppKitDisplayHandle::empty())
            },

            raw_window_handle_0_4::RawWindowHandle::Xcb(raw_window_handle_0_4::XcbHandle { connection, .. }) => {
                let mut display_handle = raw_window_handle::XcbDisplayHandle::empty();
                display_handle.connection = connection;
                // TODO: Do we need to figure out the screen here?

                raw_window_handle::RawDisplayHandle::Xcb(display_handle)
            }

            raw_window_handle_0_4::RawWindowHandle::Win32(_) => {
                raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::empty())                
            }
            
            _ => unimplemented!()
        }
    }
}
