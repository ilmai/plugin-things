use std::{cell::RefCell, rc::Rc};

use plinth_plugin::{raw_window_handle::RawWindowHandle, Editor, Host};
use plugin_canvas_slint::{editor::{EditorHandle, SlintEditor}, plugin_canvas::window::WindowAttributes};

use crate::{parameters::GainParameters, view::GainPluginView};

pub struct EditorSettings {
    pub scale: f64,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
        }
    }
}

pub struct GainPluginEditor {
    host: Rc<dyn Host>,
    editor_handle: Option<Rc<EditorHandle>>,
    parameters: Rc<GainParameters>,
    settings: Rc<RefCell<EditorSettings>>,
}

impl GainPluginEditor {
    pub fn new(host: Rc<dyn Host>, parameters: Rc<GainParameters>, settings: Rc<RefCell<EditorSettings>>) -> Self {
        Self {
            host,
            editor_handle: None,
            parameters,
            settings,
        }
    }
}

impl Editor for GainPluginEditor {
    const DEFAULT_SIZE: (f64, f64) = (400.0, 300.0);

    fn window_size(&self) -> (f64, f64) {
        let scale = self.settings.borrow().scale;

        (Self::DEFAULT_SIZE.0 * scale, Self::DEFAULT_SIZE.1 * scale)
    }

    fn set_scale(&self, scale: f64) {
        self.settings.borrow_mut().scale = scale;

        let size = self.window_size();

        if let Some(editor_handle) = self.editor_handle.as_ref() {
            editor_handle.set_window_size(size.0, size.1);
            editor_handle.set_scale(scale);
        }
    }

    fn open(&mut self, parent: RawWindowHandle) {
        // Drop old editor instance first
        self.close();

        let scale = self.settings.borrow().scale;

        let editor_handle = SlintEditor::open(
            parent,
            WindowAttributes::new(Self::DEFAULT_SIZE.into(), scale),
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

    fn on_frame(&self) {
        if let Some(editor_handle) = self.editor_handle.as_ref() {
            editor_handle.on_frame();
        }
    }
}
