use std::{ffi::{c_void, CString}, ops::{Deref, DerefMut}, path::PathBuf, rc::Weak, str::FromStr, sync::atomic::{AtomicPtr, AtomicU8, AtomicUsize, Ordering}};

use objc2::{declare::ClassBuilder, ffi::objc_disposeClassPair, msg_send, runtime::{AnyClass, Bool}, sel, ClassType, Encode, Encoding, Message, RefEncode};
use objc2::runtime::{Sel, ProtocolObject};
use objc2_app_kit::{NSDragOperation, NSDraggingInfo, NSEvent, NSEventModifierFlags, NSPasteboardTypeFileURL, NSView};
use objc2_foundation::{NSPoint, NSRect, NSURL};
use uuid::Uuid;

use crate::{Event, MouseButton, LogicalPosition, event::EventResponse, drag_drop::{DropData, DropOperation}};

use super::window::OsWindow;

pub struct OsWindowView {
    superclass: NSView,
}

struct Context {
    os_window_ptr: AtomicPtr<OsWindow>,
    input_focus: AtomicU8,
    modifier_flags: AtomicUsize,
}

unsafe impl Encode for Context {
    const ENCODING: Encoding = Encoding::Struct(
        "Encode",
        &[
            AtomicPtr::<c_void>::ENCODING,
            AtomicU8::ENCODING,
            AtomicUsize::ENCODING,
        ]
    );
}

unsafe impl RefEncode for OsWindowView {
    const ENCODING_REF: Encoding = NSView::ENCODING_REF;
}

unsafe impl Message for OsWindowView {}

impl OsWindowView {
    pub(crate) fn register_class() -> &'static AnyClass {
        let class_name = format!("plugin-canvas-OsWindowView-{}", Uuid::new_v4().simple());

        let mut builder = ClassBuilder::new(&CString::from_str(&class_name).unwrap(), NSView::class())
            .unwrap_or_else(|| panic!("Class failed to register: {class_name}"));

        builder.add_ivar::<Context>(c"_context");

        unsafe {
            // NSView
            builder.add_method(sel!(initWithFrame:), Self::init_with_frame as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(acceptsFirstMouse:), Self::accepts_first_mouse as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(acceptsFirstResponder), Self::accepts_first_responder as unsafe extern "C" fn(_, _) -> _);
            builder.add_method(sel!(isFlipped), Self::is_flipped as unsafe extern "C" fn(_, _) -> _);
            builder.add_method(sel!(keyDown:), Self::key_down as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(keyUp:), Self::key_up as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(flagsChanged:), Self::flags_changed as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(mouseMoved:), Self::mouse_moved as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(mouseDragged:), Self::mouse_dragged as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(rightMouseDragged:), Self::right_mouse_dragged as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(otherMouseDragged:), Self::other_mouse_dragged as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(mouseDown:), Self::mouse_down as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(mouseUp:), Self::mouse_up as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(rightMouseDown:), Self::right_mouse_down as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(rightMouseUp:), Self::right_mouse_up as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(otherMouseDown:), Self::other_mouse_down as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(otherMouseUp:), Self::other_mouse_up as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(mouseExited:), Self::mouse_exited as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(scrollWheel:), Self::scroll_wheel as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(drawRect:), Self::draw_rect as unsafe extern "C" fn(_, _, _) -> _);

            // NSDraggingDestination
            builder.add_method(sel!(wantsPeriodicDraggingUpdates), Self::wants_periodic_dragging_updates as unsafe extern "C" fn(_, _) -> _);
            builder.add_method(sel!(draggingEntered:), Self::dragging_entered as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(draggingUpdated:), Self::dragging_updated as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(draggingExited:), Self::dragging_exited as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(prepareForDragOperation:), Self::prepare_for_drag_operation as unsafe extern "C" fn(_, _, _) -> _);
            builder.add_method(sel!(performDragOperation:), Self::perform_drag_operation as unsafe extern "C" fn(_, _, _) -> _);
        }

        builder.register()
    }

    pub(crate) fn set_os_window_ptr(&self, ptr: *mut OsWindow) {
        self.with_context(|context| context.os_window_ptr.store(ptr, Ordering::Release));
    }

    pub(crate) fn with_os_window<T>(&self, f: impl FnOnce(&OsWindow) -> T) -> Option<T> {
        self.with_context(|context| {
            let os_window_ptr = context.os_window_ptr.load(Ordering::Acquire);
            if !os_window_ptr.is_null() {
                let os_window_weak = unsafe { Weak::from_raw(os_window_ptr) };

                let result = os_window_weak.upgrade().map(|os_window| f(&os_window));

                // Leak weak reference so it isn't dropped
                let _ = os_window_weak.into_raw();

                result
            } else {
                None
            }
        })
    }

    fn with_context<T>(&self, f: impl FnOnce(&Context) -> T) -> T {
        let ivar = self.class().instance_variable(c"_context").unwrap();
        let context: &Context = unsafe { ivar.load(self) };
        f(context)
    }

    pub(crate) fn has_input_focus(&self) -> bool {
        self.with_context(|context| context.input_focus.load(Ordering::Relaxed) != 0)
    }

    pub(crate) fn set_input_focus(&self, focus: bool) {
        let focus = if focus { 1 } else { 0 };
        self.with_context(|context| context.input_focus.store(focus, Ordering::Relaxed));
    }

    pub(super) fn send_event(&self, event: Event) -> EventResponse {
        match self.with_os_window(move |os_window| os_window.send_event(event)) {
            Some(response) => response,
            None => EventResponse::Ignored,
        }
    }

    fn key_event_text(&self, event: *const NSEvent) -> String {
        assert!(!event.is_null());

        let characters = unsafe {
            match (*event).characters() {
                Some(characters) => characters.to_string(),
                None => "".to_string(),
            }
        };

        // Do some manual mapping to get Backspace and Delete working correctly
        // Is there a more "proper" solution for this?
        match characters.as_str() {
            "\u{7f}" => "\u{8}".to_string(),
            "\u{f728}" => "\u{7f}".to_string(),
            _ => characters
        }
    }

    fn handle_modifier_event(&self, event: *const NSEvent) {
        let (old_flags, event_flags) = self.with_context(|context| {
            let old_flags = context.modifier_flags.load(Ordering::Relaxed);
            let event_flags = unsafe { (*event).modifierFlags() };
            context.modifier_flags.store(event_flags.bits(), Ordering::Relaxed);

            (old_flags, event_flags)
        });

        for (modifier, text) in [
            (NSEventModifierFlags::Command, "\u{0017}"),
            (NSEventModifierFlags::Control, "\u{0011}"),
            (NSEventModifierFlags::Option, "\u{0012}"),
            (NSEventModifierFlags::Shift, "\u{0010}"),
        ] {
            let was_down = old_flags & modifier.bits() > 0;
            let is_down = !(event_flags & modifier).is_empty();

            if !was_down && is_down {
                self.send_event(Event::KeyDown { text: text.to_string() });
            } else if was_down && !is_down {
                self.send_event(Event::KeyUp { text: text.to_string() });
            }
        }
    }
    
    fn handle_mouse_move_event(&self, event: *const NSEvent) {
        self.send_event(
            Event::MouseMoved {
                position: self.mouse_event_position(event)
            },
        );
    }

    fn handle_mouse_button_down_event(&self, event: *const NSEvent) {
        if let Some(button) = self.mouse_event_button(event) {
            self.send_event(
                Event::MouseButtonDown {
                    button,
                    position: self.mouse_event_position(event)
                },
            );
        };
    }

    fn handle_mouse_button_up_event(&self, event: *const NSEvent) {
        if let Some(button) = self.mouse_event_button(event) {
            self.send_event(
                Event::MouseButtonUp {
                    button,
                    position: self.mouse_event_position(event)
                },
            );
        };
    }

    fn mouse_event_button(&self, event: *const NSEvent) -> Option<MouseButton> {
        let button_number = unsafe { (*event).buttonNumber() };

        match button_number {
            0 => Some(MouseButton::Left),
            1 => Some(MouseButton::Right),
            2 => Some(MouseButton::Middle),
            _ => None,
        }
    }

    fn mouse_event_position(&self, event: *const NSEvent) -> LogicalPosition {
        assert!(!event.is_null());
        let point = unsafe { (*event).locationInWindow() };

        self.window_point_to_position(point)
    }

    fn window_point_to_position(&self, point_in_window: NSPoint) -> LogicalPosition {
        let local_position = self.convertPoint_fromView(point_in_window, None);

        LogicalPosition {
            x: local_position.x,
            y: local_position.y,
        }
    }

    fn drag_event_position(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> LogicalPosition {
        let point = unsafe { sender.draggingLocation() };
        self.window_point_to_position(point)
    }

    fn drag_event_data(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> DropData {
        let paths = unsafe {
            let pasteboard = sender.draggingPasteboard();
            let mut paths = Vec::new();

            if let Some(items) = pasteboard.pasteboardItems() {
                for i in 0..items.count() {
                    let item = items.objectAtIndex(i);
                    if let Some(url) = item.stringForType(NSPasteboardTypeFileURL)
                        .and_then(|url| NSURL::URLWithString(&url))
                        .and_then(|url| url.path())
                        .map(|url| PathBuf::from(url.to_string()))
                    {
                        paths.push(url);
                    }
                }
            }

            paths
        };

        if paths.is_empty() {
            DropData::None
        } else {
            DropData::Files(paths)
        }
    }

    fn convert_drag_operation(&self, response: EventResponse) -> NSDragOperation {
        if let EventResponse::DropAccepted(operation) = response {
            match operation {
                DropOperation::None => NSDragOperation::None,
                DropOperation::Copy => NSDragOperation::Copy,
                DropOperation::Move => NSDragOperation::Move,
                DropOperation::Link => NSDragOperation::Link,
            }
        } else {
            NSDragOperation::None
        }
    }

    // NSView
    unsafe extern "C" fn init_with_frame(&self, _cmd: Sel, rect: NSRect) -> Option<&Self> {
        unsafe { msg_send![super(self, NSView::class()), initWithFrame: rect] }
    }

    unsafe extern "C" fn accepts_first_mouse(&self, _cmd: Sel, _event: *const NSEvent) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn accepts_first_responder(&self, _cmd: Sel) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn is_flipped(&self, _cmd: Sel) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn key_down(&self, _cmd: Sel, event: *const NSEvent) {
        let mut text = self.key_event_text(event);
        if text == "\r" {
            text = "\u{000a}".to_string();
        }

        self.send_event(
            Event::KeyDown {
                text,
            }
        );

        if !self.has_input_focus() {
            unsafe { msg_send![super(self, NSView::class()), keyDown: event] }
        }
    }

    unsafe extern "C" fn key_up(&self, _cmd: Sel, event: *const NSEvent) {
        self.send_event(
            Event::KeyUp {
                text: self.key_event_text(event),
            }
        );

        if !self.has_input_focus() {
            unsafe { msg_send![super(self, NSView::class()), keyUp: event] }
        }
    }

    unsafe extern "C" fn flags_changed(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_modifier_event(event);
    }

    unsafe extern "C" fn mouse_moved(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_move_event(event);
    }

    unsafe extern "C" fn mouse_dragged(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_move_event(event);
    }

    unsafe extern "C" fn right_mouse_dragged(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_move_event(event);
    }

    unsafe extern "C" fn other_mouse_dragged(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_move_event(event);
    }

    unsafe extern "C" fn mouse_down(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_button_down_event(event);
    }

    unsafe extern "C" fn mouse_up(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_button_up_event(event);
    }

    unsafe extern "C" fn right_mouse_down(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_button_down_event(event);
    }

    unsafe extern "C" fn right_mouse_up(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_button_up_event(event);
    }

    unsafe extern "C" fn other_mouse_down(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_button_down_event(event);
    }

    unsafe extern "C" fn other_mouse_up(&self, _cmd: Sel, event: *const NSEvent) {
        self.handle_mouse_button_up_event(event);
    }

    unsafe extern "C" fn mouse_exited(&self, _cmd: Sel, _event: *const NSEvent) {
        self.send_event(Event::MouseExited);
    }

    unsafe extern "C" fn scroll_wheel(&self, _cmd: Sel, event: *const NSEvent) {
        assert!(!event.is_null());
        let x: f64 = unsafe { (*event).deltaX() };
        let y: f64 = unsafe { (*event).deltaY() };

        self.send_event(
            Event::MouseWheel {
                position: self.mouse_event_position(event),
                delta_x: x,
                delta_y: y,
            }
        );
    }

    unsafe extern "C" fn draw_rect(&self, _cmd: Sel, _rect: NSRect) {
        self.send_event(Event::Draw);
    }

    // NSDraggingDestination
    unsafe extern "C" fn wants_periodic_dragging_updates(&self, _cmd: Sel) -> Bool {
        Bool::NO
    }

    unsafe extern "C" fn dragging_entered(&self, _cmd: Sel, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
        let response = self.send_event(Event::DragEntered {
            position: self.drag_event_position(sender),
            data: self.drag_event_data(sender),
        });

        self.convert_drag_operation(response)
    }

    unsafe extern "C" fn dragging_updated(&self, _cmd: Sel, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
        let response = self.send_event(Event::DragMoved {
            position: self.drag_event_position(sender),
            data: self.drag_event_data(sender),
        });

        self.convert_drag_operation(response)
    }

    unsafe extern "C" fn dragging_exited(&self, _cmd: Sel, _sender: &ProtocolObject<dyn NSDraggingInfo>) {
        self.send_event(Event::DragExited);
    }

    unsafe extern "C" fn prepare_for_drag_operation(&self, _cmd: Sel, _sender: &ProtocolObject<dyn NSDraggingInfo>) -> Bool {
        Bool::YES
    }

    unsafe extern "C" fn perform_drag_operation(&self, _cmd: Sel, sender: &ProtocolObject<dyn NSDraggingInfo>) -> Bool {
        let response = self.send_event(Event::DragDropped {
            position: self.drag_event_position(sender),
            data: self.drag_event_data(sender),
        });

        if self.convert_drag_operation(response) != NSDragOperation::None {
            Bool::YES
        } else {
            Bool::NO
        }
    }
}

impl Deref for OsWindowView {
    type Target = NSView;

    fn deref(&self) -> &Self::Target {
        &self.superclass
    }
}

impl DerefMut for OsWindowView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.superclass
    }
}
