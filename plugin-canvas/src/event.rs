use crate::{dimensions::LogicalPosition, drag_drop::{DropData, DropOperation}, keyboard::KeyboardModifiers};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Debug)]
pub enum Event {
    Draw,

    KeyDown {
        key_code: keyboard_types::Code,
        text: Option<String>,
    },

    KeyUp {
        key_code: keyboard_types::Code,
        text: Option<String>,
    },

    KeyboardModifiers {
        modifiers: KeyboardModifiers,
    },

    MouseButtonDown {
        button: MouseButton,
        position: LogicalPosition,
    },

    MouseButtonUp {
        button: MouseButton,
        position: LogicalPosition,
    },

    MouseExited,

    MouseMoved {
        position: LogicalPosition,
    },

    MouseWheel {
        position: LogicalPosition,
        delta_x: f64,
        delta_y: f64,
    },

    DragEntered {
        position: LogicalPosition,
        data: DropData,
    },

    DragExited,

    DragMoved {
        position: LogicalPosition,
        data: DropData,
    },

    DragDropped {
        position: LogicalPosition,
        data: DropData,
    },
}

impl Event {
    pub fn with_mapped_position<F>(mut self, map: F) -> Self
    where
        F: Fn(&LogicalPosition) -> LogicalPosition,
    {
        match &mut self {
            Event::DragDropped { position, .. } => *position = map(position),
            Event::DragEntered { position, .. } => *position = map(position),
            Event::DragMoved { position, .. } => *position = map(position),
            Event::MouseButtonDown { position, .. } => *position = map(position),
            Event::MouseButtonUp { position, .. } => *position = map(position),
            Event::MouseMoved { position, .. } => *position = map(position),
            Event::MouseWheel { position, .. } => *position = map(position),
            _ => {}
        }
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventResponse {
    Handled,
    Ignored,
    DropAccepted(DropOperation),
}

pub type EventCallback = dyn Fn(Event) -> EventResponse;
