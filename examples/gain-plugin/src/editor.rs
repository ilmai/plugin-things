use std::rc::Rc;

use plinth_plugin::{raw_window_handle::RawWindowHandle, Editor, Host};
use plugin_canvas_slint::{editor::{EditorHandle, SlintEditor}, plugin_canvas::window::WindowAttributes};

use crate::{parameters::GainParameters, view::GainPluginView};

pub struct GainPluginEditor {
    host: Rc<dyn Host>,
    editor_handle: Option<Rc<EditorHandle>>,
    parameters: Rc<GainParameters>,
}

impl GainPluginEditor {
    pub fn new(host: Rc<dyn Host>, parameters: Rc<GainParameters>) -> Self {
        Self {
            host,
            editor_handle: None,
            parameters,
        }
    }
}

impl Editor for GainPluginEditor {
    const DEFAULT_SIZE: (f64, f64) = (400.0, 300.0);

    fn open(&mut self, parent: RawWindowHandle) {
        // Drop old editor instance first
        self.close();

        let editor_handle = SlintEditor::open(
            parent,
            WindowAttributes::new(Self::DEFAULT_SIZE.into(), 1.0),
            {
                let parameters = self.parameters.clone();
                let host = self.host.clone();
                
                move |_| {
                    GainPluginView::new(parameters.clone(), host.clone())
                }
            },
        );

        self.editor_handle = Some(editor_handle);
    }

    fn close(&mut self) {
        self.editor_handle = None;
    }

    fn on_frame(&mut self) {
        if let Some(editor_handle) = self.editor_handle.as_ref() {
            editor_handle.on_frame();
        }
    }
}
