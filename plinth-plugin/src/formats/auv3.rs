#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod au_render_event;
mod event;
mod host;
mod macros;
mod parameters;
mod plugin;
mod reader;
mod util;
mod wrapper;
mod writer;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub use au_render_event::AURenderEvent;
pub use event::EventIterator;
pub use host::Auv3Host;
pub use plugin::Auv3Plugin;
pub use reader::Auv3Reader;
pub use util::parameter_multiplier;
pub use wrapper::Auv3Wrapper;
pub use writer::Auv3Writer;
