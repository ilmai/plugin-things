use std::{rc::Rc, collections::VecDeque, sync::{Arc, Mutex}};

use i_slint_core::{platform::{Platform, PlatformError}, window::WindowAdapter};
use slint::{EventLoopError, platform::EventLoopProxy};

use crate::window_adapter::PluginCanvasWindowAdapter;

pub(crate) type CallbackQueue = Arc<Mutex<VecDeque<Box<dyn FnOnce() + Send>>>>;

struct PluginCanvasEventLoopProxy {
    queue: CallbackQueue,
}

impl EventLoopProxy for PluginCanvasEventLoopProxy {
    fn quit_event_loop(&self) -> Result<(), EventLoopError> {
        Ok(())
    }

    fn invoke_from_event_loop(&self, event: Box<dyn FnOnce() + Send>) -> Result<(), EventLoopError> {
        self.queue.lock().unwrap().push_back(event);
        Ok(())
    }
}

pub struct PluginCanvasPlatform {
    callback_queue: CallbackQueue,
}

impl PluginCanvasPlatform {
    pub fn new() -> Self {
        Self { callback_queue: Arc::new(Mutex::new(VecDeque::new())) }
    }
}

impl Platform for PluginCanvasPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        PluginCanvasWindowAdapter::new(self.callback_queue.clone())
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(PluginCanvasEventLoopProxy {
            queue: self.callback_queue.clone(),
        }))
    }
}
