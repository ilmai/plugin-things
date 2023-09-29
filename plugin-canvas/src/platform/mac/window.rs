use std::{ffi::c_void, sync::atomic::Ordering, rc::Rc, cell::RefCell, ptr::null_mut};

use icrate::{AppKit::{NSTrackingArea, NSView, NSWindow, NSTrackingMouseEnteredAndExited, NSTrackingMouseMoved, NSTrackingActiveAlways, NSTrackingInVisibleRect, NSCursor}, Foundation::{CGPoint, CGSize, CGRect, NSInvocationOperation, NSOperationQueue}};
use objc2::{ClassType, msg_send_id, rc::Id, sel};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, AppKitWindowHandle, HasRawDisplayHandle, RawDisplayHandle, AppKitDisplayHandle};

use crate::{error::Error, platform::interface::{OsWindowInterface, OsWindowHandle, OsWindowBuilder}, event::EventCallback, window::WindowAttributes, Event, cursor::Cursor};

use super::display_link::{CVDisplayLinkRef, CVTimeStamp, CVReturn, self};
use super::view::OsWindowView;

pub struct OsWindow {
    window_handle: AppKitWindowHandle,
    display_link: RefCell<Option<CVDisplayLinkRef>>,
    event_callback: Box<EventCallback>,
}

impl OsWindow {
    unsafe fn from_ptr<'a>(ptr: *mut c_void) -> &'a mut Self {
        assert!(!ptr.is_null());
        let window_ptr = ptr as *mut OsWindow;
        unsafe { &mut *window_ptr }
    }

    pub(super) fn send_event(&self, event: Event) {
        (self.event_callback)(event);
    }

    fn view(&self) -> &OsWindowView {
        assert!(!self.window_handle.ns_view.is_null());
        let window_view: *const OsWindowView = self.window_handle.ns_view as _;
        unsafe { &*window_view }
    }
}

impl OsWindowInterface for OsWindow {
    fn open(
        parent_window_handle: RawWindowHandle,
        attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
        window_builder: OsWindowBuilder,
    ) -> Result<(), Error> {
        let RawWindowHandle::AppKit(parent_window_handle) = parent_window_handle else {
            return Err(Error::PlatformError("Not an AppKit window".into()));
        };

        let view_rect = CGRect::new(
            CGPoint { x: 0.0, y: 0.0 },
            CGSize { width: attributes.size.width, height: attributes.size.height },
        );

        let (view, window_handle) = unsafe {
            let view: Id<OsWindowView> = msg_send_id![OsWindowView::alloc(), initWithFrame: view_rect];
        
            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                view_rect,
                NSTrackingMouseEnteredAndExited |
                NSTrackingMouseMoved |
                NSTrackingActiveAlways |
                NSTrackingInVisibleRect,
                Some(&view),
                None,
            );
            view.addTrackingArea(&tracking_area);

            let parent_view: &mut NSView = &mut *(parent_window_handle.ns_view as *mut NSView);
            parent_view.addSubview(&view);
    
            let mut window_handle = AppKitWindowHandle::empty();
            window_handle.ns_window = parent_view.window().unwrap().as_ref() as *const NSWindow as _;
            window_handle.ns_view = view.as_ref() as *const OsWindowView as _;
    
            (view, window_handle)
        };

        let raw_window_handle = RawWindowHandle::AppKit(window_handle);
        let raw_display_handle = RawDisplayHandle::AppKit(AppKitDisplayHandle::empty());

        let window = Rc::new(Self {
            window_handle,
            display_link: Default::default(),
            event_callback,
        });

        let window_clone = window.clone();
        let window_ptr = Rc::into_raw(window);

        view.os_window_ptr.store(window_ptr as _, Ordering::Relaxed);

        let displays = display_link::get_displays_with_rect(view_rect);
        assert!(!displays.is_empty());

        let mut cv_display_link = display_link::create_with_active_cg_displays();
        display_link::set_output_callback(&mut cv_display_link, display_link_callback, window_ptr as _);
        display_link::set_current_display(&mut cv_display_link, displays[0]);
        display_link::start(&mut cv_display_link);

        unsafe {
            let window = &*window_ptr;
            *window.display_link.borrow_mut() = Some(cv_display_link);
        }

        window_builder(OsWindowHandle::new(raw_window_handle, raw_display_handle, window_clone));

        Ok(())
    }

    fn set_cursor(&self, cursor: Cursor) {
        unsafe {
            let cursor = match cursor {
                Cursor::Arrow => NSCursor::arrowCursor(),
                Cursor::Pointer => NSCursor::pointingHandCursor(),
            };
    
            cursor.set()    
        }
    }

    fn set_input_focus(&self, focus: bool) {
        self.view().set_input_focus(focus);
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        self.view().os_window_ptr.store(null_mut(), Ordering::Relaxed);

        if let Some(mut display_link) = self.display_link.take() {
            display_link::release(&mut display_link);
        }
    }
}

unsafe impl HasRawDisplayHandle for OsWindow {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        RawDisplayHandle::AppKit(AppKitDisplayHandle::empty())
    }
}

unsafe impl HasRawWindowHandle for OsWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        RawWindowHandle::AppKit(self.window_handle)
    }
}

unsafe extern "C" fn display_link_callback(
    _display_link: CVDisplayLinkRef,
    _in_now: *mut CVTimeStamp,
    _in_output_time: *mut CVTimeStamp,
    _flags_in: u64,
    _flags_out: *mut u64,
    display_link_context: *mut c_void,
) -> CVReturn {
    let window = unsafe { OsWindow::from_ptr(display_link_context) };
    let view = window.window_handle.ns_view as *const OsWindowView;

    let operation = NSInvocationOperation::initWithTarget_selector_object(
        NSInvocationOperation::alloc(),
        &*view,
        sel!(draw),
        None,
    ).unwrap();

    NSOperationQueue::mainQueue().addOperation(&operation);

    CVReturn::Success
}
