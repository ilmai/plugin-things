use std::rc::Rc;

use plinth_plugin::{FloatParameter, Parameter, Parameters};
use plugin_canvas_slint::{plugin_canvas::{event::EventResponse, Event, Window}, view::PluginView};

use crate::parameters::{GainParameter, GainParameters};

slint::include_modules!();

pub struct GainPluginView {
    plugin_window: PluginWindow,
    parameters: Rc<GainParameters>,
}

impl GainPluginView {
    pub fn new(_window: Rc<Window>, parameters: Rc<GainParameters>) -> Self {
        let plugin_window = PluginWindow::new().unwrap();

        Self {
            plugin_window,
            parameters,
        }
    }
}

impl PluginView for GainPluginView {
    fn window(&self) -> &slint::Window {
        self.plugin_window.window()
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        #[expect(clippy::single_match)]
        match event {
            Event::Draw => {
                let gain_parameter = self.parameters.typed::<FloatParameter>(GainParameter::Gain).unwrap();

                self.plugin_window.set_gain(UiParameter {
                    default_value: gain_parameter.default_value() as _,
                    display_value: gain_parameter.to_string().into(),
                    id: gain_parameter.info().id() as _,
                    normalized_value: gain_parameter.normalized_value() as _,
                });
            }

            _ => {}
        }

        EventResponse::Ignored
    }
}
