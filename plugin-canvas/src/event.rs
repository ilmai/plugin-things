use crate::{dimensions::LogicalPosition, drag_drop::{DropData, DropOperation}};

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug)]
pub enum Event {
    Draw,

    KeyDown {
        text: String,
    },

    KeyUp {
        text: String,
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
    }
}

pub enum EventResponse {
    Handled,
    Ignored,
    DragAccepted(DropOperation),
}

pub type EventCallback = dyn Fn(Event) -> EventResponse + Send;
