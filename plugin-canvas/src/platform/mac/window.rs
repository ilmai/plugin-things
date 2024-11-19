use std::{cell::RefCell, ffi::c_void, ptr::{null_mut, NonNull}, rc::Rc, sync::atomic::{AtomicBool, Ordering}};

use core_graphics::display::CGDisplay;
use cursor_icon::CursorIcon;
use objc2::{msg_send_id, rc::{Allocated, Id}, runtime::AnyClass, sel, ClassType};
use objc2_app_kit::{NSCursor, NSPasteboardTypeFileURL, NSScreen, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{CGPoint, CGRect, CGSize, MainThreadMarker, NSArray, NSInvocationOperation, NSOperationQueue};
use raw_window_handle::{AppKitWindowHandle, HasDisplayHandle, HasWindowHandle, RawWindowHandle};

use crate::{error::Error, platform::interface::{OsWindowInterface, OsWindowHandle}, event::{EventCallback, EventResponse}, window::WindowAttributes, Event, LogicalPosition};

use super::display_link::{CVDisplayLinkRef, CVTimeStamp, CVReturn, self};
use super::view::OsWindowView;

pub struct OsWindow {
    view_class: &'static AnyClass,

    window_attributes: WindowAttributes,

    window_handle: AppKitWindowHandle,
    display_link: RefCell<Option<CVDisplayLinkRef>>,
    event_callback: Box<EventCallback>,

    cursor_hidden: AtomicBool,

    main_thread_marker: MainThreadMarker,
}

impl OsWindow {
    pub(super) fn window_attributes(&self) -> &WindowAttributes {
        &self.window_attributes
    }

    unsafe fn from_ptr<'a>(ptr: *mut c_void) -> &'a mut Self {
        assert!(!ptr.is_null());
        let window_ptr = ptr as *mut OsWindow;
        unsafe { &mut *window_ptr }
    }

    pub(super) fn send_event(&self, event: Event) -> EventResponse {
        (self.event_callback)(event)
    }

    fn view(&self) -> &OsWindowView {
        let window_view: *const OsWindowView = self.window_handle.ns_view.as_ptr() as _;
        unsafe { &*window_view }
    }
}

impl OsWindowInterface for OsWindow {
    fn open(
        parent_window_handle: RawWindowHandle,
        window_attributes: WindowAttributes,
        event_callback: Box<EventCallback>,
    ) -> Result<OsWindowHandle, Error> {
        let RawWindowHandle::AppKit(parent_window_handle) = parent_window_handle else {
            return Err(Error::PlatformError("Not an AppKit window".into()));
        };

        let view_class = OsWindowView::register_class();

        let physical_size = crate::PhysicalSize::from_logical(&window_attributes.size, window_attributes.scale);

        let view_rect = CGRect::new(
            CGPoint { x: 0.0, y: 0.0 },
            CGSize { width: physical_size.width as f64, height: physical_size.height as f64 },
        );

        let (view, window_handle) = unsafe {
            let view: Allocated<OsWindowView> = msg_send_id![view_class, alloc];
            let view: Id<OsWindowView> = msg_send_id![view, initWithFrame: view_rect];
        
            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                view_rect,
                NSTrackingAreaOptions::NSTrackingMouseEnteredAndExited |
                NSTrackingAreaOptions::NSTrackingMouseMoved |
                NSTrackingAreaOptions::NSTrackingActiveAlways |
                NSTrackingAreaOptions::NSTrackingInVisibleRect,
                Some(&view),
                None,
            );
            view.addTrackingArea(&tracking_area);

            let dragged_types = NSArray::arrayWithObject(NSPasteboardTypeFileURL);
            view.registerForDraggedTypes(&dragged_types);

            let parent_view: &mut NSView = &mut *(parent_window_handle.ns_view.as_ptr() as *mut NSView);
            parent_view.addSubview(&view);
    
            let window_handle = AppKitWindowHandle::new(
                NonNull::new(view.as_ref() as *const OsWindowView as _).unwrap()
            );
    
            (view, window_handle)
        };

        let main_thread_marker = MainThreadMarker::new().unwrap();

        let window = Rc::new(Self {
            view_class,

            window_attributes,

            window_handle,
            display_link: Default::default(),
            event_callback,

            cursor_hidden: Default::default(),

            main_thread_marker,
        });

        let window_clone = window.clone();
        let window_ptr = Rc::into_raw(window);

        view.set_os_window_ptr(window_ptr as _);

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

        Ok(OsWindowHandle::new(window_clone))
    }

    fn os_scale(&self) -> f64 {
        self.view()
            .window()
            .map(|window| window.backingScaleFactor())
            .unwrap_or(1.0)
    }

    fn set_cursor(&self, cursor: Option<CursorIcon>) {
        unsafe {
            if let Some(cursor) = cursor {
                let cursor = match cursor {
                    CursorIcon::Default => NSCursor::arrowCursor(),
                    CursorIcon::ContextMenu => NSCursor::contextualMenuCursor(),
                    CursorIcon::Help => NSCursor::arrowCursor(), // TODO
                    CursorIcon::Pointer => NSCursor::pointingHandCursor(),
                    CursorIcon::Progress => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::Wait => NSCursor::arrowCursor(), // TODO
                    CursorIcon::Cell => NSCursor::crosshairCursor(),
                    CursorIcon::Crosshair => NSCursor::crosshairCursor(),
                    CursorIcon::Text => NSCursor::IBeamCursor(),
                    CursorIcon::VerticalText => NSCursor::IBeamCursorForVerticalLayout(),
                    CursorIcon::Alias => NSCursor::dragLinkCursor(),
                    CursorIcon::Copy => NSCursor::dragCopyCursor(),
                    CursorIcon::Move => NSCursor::openHandCursor(),
                    CursorIcon::NoDrop => NSCursor::operationNotAllowedCursor(),
                    CursorIcon::NotAllowed => NSCursor::operationNotAllowedCursor(),
                    CursorIcon::Grab => NSCursor::openHandCursor(),
                    CursorIcon::Grabbing => NSCursor::closedHandCursor(),
                    CursorIcon::EResize => NSCursor::resizeRightCursor(),
                    CursorIcon::NResize => NSCursor::resizeUpCursor(),
                    CursorIcon::NeResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::NwResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::SResize => NSCursor::resizeDownCursor(),
                    CursorIcon::SeResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::SwResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::WResize => NSCursor::resizeLeftCursor(),
                    CursorIcon::EwResize => NSCursor::resizeLeftRightCursor(),
                    CursorIcon::NsResize => NSCursor::resizeUpDownCursor(),
                    CursorIcon::NeswResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::NwseResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::ColResize => NSCursor::resizeLeftRightCursor(),
                    CursorIcon::RowResize => NSCursor::resizeUpDownCursor(),
                    CursorIcon::AllScroll => NSCursor::openHandCursor(),
                    CursorIcon::ZoomIn => NSCursor::arrowCursor(), // TODO
                    CursorIcon::ZoomOut => NSCursor::arrowCursor(), // TODO
                    _ => todo!(),
                };
        
                cursor.set();

                if self.cursor_hidden.swap(false, Ordering::Relaxed) {
                    NSCursor::unhide();
                }
            } else {
                if !self.cursor_hidden.swap(true, Ordering::Relaxed) {
                    NSCursor::hide();
                }
            }
        }
    }

    fn set_input_focus(&self, focus: bool) {
        self.view().set_input_focus(focus);
    }

    fn warp_mouse(&self, position: LogicalPosition) {
        let window_position = unsafe { self.view().convertPoint_toView(CGPoint::new(position.x, position.y), None) };
        let screen_position = unsafe { self.view().window().unwrap().convertPointToScreen(window_position) };
        let screen_height = NSScreen::mainScreen(self.main_thread_marker).unwrap().frame().size.height;
        let cg_point = core_graphics::geometry::CGPoint::new(screen_position.x, screen_height - screen_position.y);
        CGDisplay::warp_mouse_cursor_position(cg_point).unwrap();
    }
    
    fn poll_events(&self) -> Result<(), Error> {
        Ok(())
    }
    
    fn set_size(&self, size: crate::LogicalSize) {
        let physical_size = crate::PhysicalSize::from_logical(&size, self.window_attributes.scale);
        let view_rect = CGRect::new(
            CGPoint { x: 0.0, y: 0.0 },
            CGSize { width: physical_size.width as f64, height: physical_size.height as f64 },
        );

        unsafe {
            self.view().setFrame(view_rect);
        }
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        self.view().set_os_window_ptr(null_mut());

        if let Some(mut display_link) = self.display_link.take() {
            display_link::release(&mut display_link);
        }

        OsWindowView::unregister_class(self.view_class);
    }
}

impl HasDisplayHandle for OsWindow {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::appkit())
    }
}

impl HasWindowHandle for OsWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let raw_window_handle = RawWindowHandle::AppKit(self.window_handle);
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw_window_handle) })
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
    let view = window.window_handle.ns_view.as_ptr() as *const OsWindowView;

    let operation = NSInvocationOperation::initWithTarget_selector_object(
        NSInvocationOperation::alloc(),
        &*view,
        sel!(drawRect:),
        None,
    ).unwrap();

    NSOperationQueue::mainQueue().addOperation(&operation);

    CVReturn::Success
}
