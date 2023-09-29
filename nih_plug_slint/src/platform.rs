use std::rc::Rc;

use i_slint_core::{platform::{Platform, PlatformError}, window::WindowAdapter};

use crate::window_adapter::PluginCanvasWindowAdapter;

pub struct PluginCanvasPlatform;

impl Platform for PluginCanvasPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        PluginCanvasWindowAdapter::new()
    }
} 
