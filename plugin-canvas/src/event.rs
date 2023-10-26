use crate::{dimensions::LogicalPosition, drag::{DragData, DragOperation}};

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
        data: DragData,
    },

    DragExited,

    DragMoved {
        position: LogicalPosition,
        data: DragData,
    },

    DragDropped {
        position: LogicalPosition,
        data: DragData,
    }
}

pub enum EventResponse {
    Handled,
    Ignored,
    DragAccepted(DragOperation),
}

pub type EventCallback = dyn Fn(Event) -> EventResponse + Send;
