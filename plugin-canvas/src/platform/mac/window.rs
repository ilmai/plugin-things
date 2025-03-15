use std::{cell::RefCell, ptr::{null_mut, NonNull}, rc::Rc, sync::atomic::{AtomicBool, Ordering}};

use cursor_icon::CursorIcon;
use objc2::{msg_send, rc::{Allocated, Retained}, sel, AllocAnyThread};
use objc2_app_kit::{NSCursor, NSPasteboardTypeFileURL, NSScreen, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_core_foundation::{CGPoint, CGSize};
use objc2_core_graphics::CGWarpMouseCursorPosition;
use objc2_foundation::{MainThreadMarker, NSArray, NSDefaultRunLoopMode, NSPoint, NSRect, NSRunLoop, NSSize};
use objc2_quartz_core::CADisplayLink;
use raw_window_handle::{AppKitWindowHandle, HasDisplayHandle, HasWindowHandle, RawWindowHandle};

use crate::{platform::os_window_handle::OsWindowHandle, Event, LogicalPosition};
use crate::error::Error;
use crate::event::{EventCallback, EventResponse};
use crate::platform::interface::OsWindowInterface;
use crate::window::WindowAttributes;

use super::view::OsWindowView;

pub(crate) struct OsWindow {
    window_handle: AppKitWindowHandle,
    display_link: RefCell<Option<Retained<CADisplayLink>>>,
    event_callback: Box<EventCallback>,

    cursor_hidden: AtomicBool,

    main_thread_marker: MainThreadMarker,
}

impl OsWindow {
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

        let view_rect = NSRect::new(
            NSPoint { x: 0.0, y: 0.0 },
            NSSize { width: physical_size.width as f64, height: physical_size.height as f64 },
        );

        let (view, window_handle) = unsafe {
            let view: Allocated<OsWindowView> = msg_send![view_class, alloc];
            let view: Retained<OsWindowView> = msg_send![view, initWithFrame: view_rect];
        
            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                view_rect,
                NSTrackingAreaOptions::MouseEnteredAndExited |
                NSTrackingAreaOptions::MouseMoved |
                NSTrackingAreaOptions::ActiveAlways |
                NSTrackingAreaOptions::InVisibleRect,
                Some(&view),
                None,
            );
            view.addTrackingArea(&tracking_area);

            let dragged_types = NSArray::arrayWithObject(NSPasteboardTypeFileURL);
            view.registerForDraggedTypes(&dragged_types);

            let parent_view: &mut NSView = &mut *(parent_window_handle.ns_view.as_ptr() as *mut NSView);
            parent_view.addSubview(&view);
    
            let window_handle = AppKitWindowHandle::new(
                NonNull::new(view.as_ref() as *const NSView as _).unwrap()
            );
    
            (view, window_handle)
        };

        let main_thread_marker = MainThreadMarker::new().unwrap();

        let window = Rc::new(Self {
            window_handle,
            display_link: Default::default(),
            event_callback,

            cursor_hidden: Default::default(),

            main_thread_marker,
        });

        let display_link = unsafe { view.displayLinkWithTarget_selector(&view, sel!(drawRect:)) };

        unsafe {
            display_link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSDefaultRunLoopMode)
        };

        *window.display_link.borrow_mut() = Some(display_link);

        view.set_os_window_ptr(Rc::downgrade(&window).into_raw() as _);

        Ok(OsWindowHandle::new(window))
    }

    fn os_scale(&self) -> f64 {
        self.view()
            .window()
            .map(|window| window.backingScaleFactor())
            .unwrap_or(1.0)
    }

    fn resized(&self, size: crate::LogicalSize) {
        let cg_size = CGSize {
            width: size.width as _,
            height: size.height as _,
        };

        unsafe { self.view().setFrameSize(cg_size) };
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
                    CursorIcon::EResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::NResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::NeResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::NwResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::SResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::SeResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::SwResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::WResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::EwResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::NsResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::NeswResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::NwseResize => NSCursor::arrowCursor(), // TODO
                    CursorIcon::ColResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::RowResize => NSCursor::arrowCursor(), // TODO,
                    CursorIcon::AllScroll => NSCursor::openHandCursor(),
                    CursorIcon::ZoomIn => NSCursor::arrowCursor(), // TODO
                    CursorIcon::ZoomOut => NSCursor::arrowCursor(), // TODO
                    _ => todo!(),
                };
        
                cursor.set();

                if self.cursor_hidden.swap(false, Ordering::Relaxed) {
                    NSCursor::unhide();
                }
            } else if !self.cursor_hidden.swap(true, Ordering::Relaxed) {
                NSCursor::hide();
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
        let cg_point = CGPoint::new(screen_position.x, screen_height - screen_position.y);
        unsafe { CGWarpMouseCursorPosition(cg_point) };
    }
    
    fn poll_events(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl Drop for OsWindow {
    fn drop(&mut self) {
        if let Some(display_link) = self.display_link.borrow().as_ref() {
            unsafe {
                display_link.removeFromRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSDefaultRunLoopMode)
            };
        }

        self.view().set_os_window_ptr(null_mut());
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
