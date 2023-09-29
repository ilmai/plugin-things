use crate::dimensions::LogicalPosition;

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
    }
}

pub type EventCallback = dyn Fn(Event) + Send;
