use std::{sync::atomic::{Ordering, AtomicU8, AtomicUsize}, path::PathBuf};

use icrate::{AppKit::{NSView, NSEvent, NSResponder, NSTextInputClient, NSEventModifierFlagShift, NSEventModifierFlagCommand, NSEventModifierFlagControl, NSEventModifierFlagOption, NSDraggingDestination, NSDraggingInfo, NSDragOperation, NSDragOperationNone, NSDragOperationCopy, NSDragOperationMove, NSPasteboardTypeFileURL, NSDragOperationLink}, Foundation::{NSRect, NSArray, NSRange, NSRangePointer, NSPoint, NSAttributedStringKey, NSAttributedString, CGPoint, CGSize, NSURL}};
use objc2::{declare_class, mutability, ClassType,  msg_send, declare::IvarEncode, runtime::{AnyObject, Sel, NSObject, NSObjectProtocol, ProtocolObject}, ffi::NSUInteger, rc::Id};

use crate::{Event, MouseButton, LogicalPosition, event::EventResponse, drag_drop::{DropData, DropOperation}};

use super::{types::AtomicVoidPtr, window::OsWindow};

declare_class! {
    pub(super) struct OsWindowView {
        pub(super) os_window_ptr: IvarEncode<AtomicVoidPtr, "_os_window_ptr">,
        // AtomicBool isn't Encode, so let's use AtomicU8 instead
        input_focus: IvarEncode<AtomicU8, "_input_focus">,
        modifier_flags: IvarEncode<AtomicUsize, "_modifier_flags">,
    }
    
    mod ivars;

    unsafe impl ClassType for OsWindowView {
        #[inherits(NSResponder, NSObject)]
        type Super = NSView;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "plugin-canvas-WindowView";
    }

    unsafe impl OsWindowView {
        #[method(initWithFrame:)]
        fn init_with_frame(&self, rect: NSRect) -> Option<&Self> {
            unsafe { msg_send![super(self), initWithFrame: rect] }
        }

        #[method(acceptsFirstMouse:)]
        fn accepts_first_mouse(&self, _event: *const NSEvent) -> bool {
            true
        }

        #[method(acceptsFirstResponder)]
        fn accepts_first_responser(&self) -> bool {
            true
        }

        #[method(isFlipped)]
        fn is_flipped(&self) -> bool {
            true
        }

        #[method(keyDown:)]
        fn key_down(&self, event: *const NSEvent) {
            unsafe {
                let events = NSArray::arrayWithObject(&*event);
                self.interpretKeyEvents(&events);
            }

            let mut text = self.key_event_text(event);
            if text == "\r" {
                text = "\u{000a}".to_string();
            }

            self.os_window().send_event(
                Event::KeyDown {
                    text,
                }
            );

            if !self.has_input_focus() {
                unsafe { msg_send![super(self), keyDown: event] }
            }
        }

        #[method(keyUp:)]
        fn key_up(&self, event: *const NSEvent) {
            self.os_window().send_event(
                Event::KeyUp {
                    text: self.key_event_text(event),
                }
            );

            if !self.has_input_focus() {
                unsafe { msg_send![super(self), keyUp: event] }
            }
        }

        #[method(flagsChanged:)]
        fn flags_changed(&self, event: *const NSEvent) {
            self.handle_modifier_event(event);
        }

        #[method(mouseMoved:)]
        fn mouse_moved(&self, event: *const NSEvent) {
            self.handle_mouse_move_event(event);
        }

        #[method(mouseDragged:)]
        fn mouse_dragged(&self, event: *const NSEvent) {
            self.handle_mouse_move_event(event);
        }

        #[method(rightMouseDragged:)]
        fn right_mouse_dragged(&self, event: *const NSEvent) {
            self.handle_mouse_move_event(event);
        }

        #[method(otherMouseDragged:)]
        fn other_mouse_dragged(&self, event: *const NSEvent) {
            self.handle_mouse_move_event(event);
        }

        #[method(mouseDown:)]
        fn mouse_down(&self, event: *const NSEvent) {
            self.handle_mouse_button_down_event(event);
        }

        #[method(mouseUp:)]
        fn mouse_up(&self, event: *const NSEvent) {
            self.handle_mouse_button_up_event(event);
        }

        #[method(rightMouseDown:)]
        fn right_mouse_down(&self, event: *const NSEvent) {
            self.handle_mouse_button_down_event(event);
        }

        #[method(rightMouseUp:)]
        fn right_mouse_up(&self, event: *const NSEvent) {
            self.handle_mouse_button_up_event(event);
        }

        #[method(otherMouseDown:)]
        fn other_mouse_down(&self, event: *const NSEvent) {
            self.handle_mouse_button_down_event(event);
        }

        #[method(otherMouseUp:)]
        fn other_mouse_up(&self, event: *const NSEvent) {
            self.handle_mouse_button_up_event(event);
        }

        #[method(mouseExited:)]
        fn mouse_exited(&self, _event: *const NSEvent) {
            self.os_window().send_event(Event::MouseExited);
        }

        #[method(scrollWheel:)]
        fn scroll_wheel(&self, event: *const NSEvent) {
            assert!(!event.is_null());
            let x: f64 = unsafe { (*event).deltaX() };
            let y: f64 = unsafe { (*event).deltaY() };

            self.os_window().send_event(
                Event::MouseWheel {
                    position: self.mouse_event_position(event),
                    delta_x: x,
                    delta_y: y,
                }
            );
        }

        #[method(draw)]
        fn draw(&self) {
            // Window might have closed while the operation calling this function
            // was queued
            if !self.os_window_ptr.load(Ordering::Relaxed).is_null() {
                self.os_window().send_event(Event::Draw);
            }
        }
    }

    unsafe impl NSTextInputClient for OsWindowView {
        #[method(insertText:replacementRange:)]
        unsafe fn insert_text_replacement_range(
            &self,
            _string: &AnyObject,
            _replacement_range: NSRange,
        ) {
        }        

        #[method(doCommandBySelector:)]
        unsafe fn do_command_by_selector(&self, _selector: Sel) {
        }

        #[method(setMarkedText:selectedRange:replacementRange:)]
        unsafe fn set_marked_text_selected_range_replacement_range(
            &self,
            _string: &AnyObject,
            _selected_range: NSRange,
            _replacement_range: NSRange,
        ) {
        }

        #[method(unmarkText)]
        unsafe fn unmark_text(&self) {            
        }

        #[method(selectedRange)]
        unsafe fn selected_range(&self) -> NSRange {
            NSRange::new(0, 0)
        }

        #[method(markedRange)]
        unsafe fn marked_range(&self) -> NSRange {
            NSRange::new(0, 0)
        }

        #[method(hasMarkedText)]
        unsafe fn has_marked_text(&self) -> bool {
            false
        }

        #[method_id(attributedSubstringForProposedRange:actualRange:)]
        unsafe fn attributed_substring_for_proposed_range_actual_range(
            &self,
            _range: NSRange,
            _actual_range: NSRangePointer,
        ) -> Option<Id<NSAttributedString>> {
            None
        }

        #[method_id(validAttributesForMarkedText)]
        unsafe fn valid_attributes_for_marked_text(&self) -> Id<NSArray<NSAttributedStringKey>> {
            NSArray::new()
        }

        #[method(firstRectForCharacterRange:actualRange:)]
        unsafe fn first_rect_for_character_range_actual_range(
            &self,
            _range: NSRange,
            _actual_range: NSRangePointer,
        ) -> NSRect {
            NSRect::new(
                CGPoint::new(0.0, 0.0),
                CGSize::new(0.0, 0.0),
            )
        }

        #[method(characterIndexForPoint:)]
        unsafe fn character_index_for_point(&self, _point: NSPoint) -> NSUInteger {
            0
        }
    }

    unsafe impl NSDraggingDestination for OsWindowView {
        #[method(wantsPeriodicDraggingUpdates)]
        unsafe fn wants_periodic_dragging_updates(&self) -> bool {
            false
        }

        #[method(draggingEntered:)]
        unsafe fn dragging_entered(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            let response = self.os_window().send_event(Event::DragEntered {
                position: self.drag_event_position(sender),
                data: self.drag_event_data(sender),
            });

            self.convert_drag_operation(response)
        }

        #[method(draggingUpdated:)]
        unsafe fn dragging_updated(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            let response = self.os_window().send_event(Event::DragMoved {
                position: self.drag_event_position(sender),
                data: self.drag_event_data(sender),
            });

            self.convert_drag_operation(response)
        }

        #[method(draggingExited:)]
        unsafe fn dragging_exited(&self, _sender: &ProtocolObject<dyn NSDraggingInfo>) {
            self.os_window().send_event(Event::DragExited);
        }

        #[method(prepareForDragOperation:)]
        unsafe fn prepare_for_drag_operation(&self, _sender: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            true
        }

        #[method(performDragOperation:)]
        unsafe fn perform_drag_operation(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            let response = self.os_window().send_event(Event::DragDropped {
                position: self.drag_event_position(sender),
                data: self.drag_event_data(sender),
            });

            self.convert_drag_operation(response) != NSDragOperationNone
        }
    }
}

unsafe impl NSObjectProtocol for OsWindowView {}

impl OsWindowView {
    pub(crate) fn os_window(&self) -> &mut OsWindow {
        let window_ptr = self.os_window_ptr.load(Ordering::Relaxed) as *mut OsWindow;
        assert!(!window_ptr.is_null());
        unsafe { &mut *window_ptr }
    }

    pub(crate) fn has_input_focus(&self) -> bool {
        self.input_focus.load(Ordering::Relaxed) != 0
    }

    pub(crate) fn set_input_focus(&self, focus: bool) {
        let focus = if focus { 1 } else { 0 };
        self.input_focus.store(focus, Ordering::Relaxed);
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
        let old_flags = self.modifier_flags.load(Ordering::Relaxed);
        let event_flags = unsafe { (*event).modifierFlags() };
        self.modifier_flags.store(event_flags, Ordering::Relaxed);

        for (modifier, text) in [
            (NSEventModifierFlagCommand, "\u{0017}"),
            (NSEventModifierFlagControl, "\u{0011}"),
            (NSEventModifierFlagOption, "\u{0012}"),
            (NSEventModifierFlagShift, "\u{0010}"),
        ] {
            let was_down = old_flags & modifier > 0;
            let is_down = event_flags & modifier > 0;

            if !was_down && is_down {
                self.os_window().send_event(Event::KeyDown { text: text.to_string() });
            } else if was_down && !is_down {
                self.os_window().send_event(Event::KeyUp { text: text.to_string() });
            }
        }
    }
    
    fn handle_mouse_move_event(&self, event: *const NSEvent) {
        self.os_window().send_event(
            Event::MouseMoved {
                position: self.mouse_event_position(event)
            },
        );
    }

    fn handle_mouse_button_down_event(&self, event: *const NSEvent) {
        if let Some(button) = self.mouse_event_button(event) {
            self.os_window().send_event(
                Event::MouseButtonDown {
                    button,
                    position: self.mouse_event_position(event)
                },
            );
        };
    }

    fn handle_mouse_button_up_event(&self, event: *const NSEvent) {
        if let Some(button) = self.mouse_event_button(event) {
            self.os_window().send_event(
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

    fn window_point_to_position(&self, point_in_window: CGPoint) -> LogicalPosition {
        let local_position = unsafe { self.convertPoint_fromView(point_in_window, None) };
        let user_scale = self.os_window().window_attributes().user_scale;

        LogicalPosition {
            x: local_position.x / user_scale,
            y: local_position.y / user_scale,
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
                    if let Some(url) = item.stringForType(&NSPasteboardTypeFileURL)
                        .and_then(|url| NSURL::URLWithString(&url))
                        .and_then(|url| url.path())
                        .and_then(|url| Some(PathBuf::from(url.to_string())))
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
                DropOperation::None => NSDragOperationNone,
                DropOperation::Copy => NSDragOperationCopy,
                DropOperation::Move => NSDragOperationMove,
                DropOperation::Link => NSDragOperationLink,
            }
        } else {
            NSDragOperationNone
        }
    }
}
