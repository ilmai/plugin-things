use std::rc::Rc;

use plinth_plugin::{FloatParameter, Host, Parameter, ParameterId, Parameters};
use plugin_canvas_slint::{plugin_canvas::{event::EventResponse, Event}, view::PluginView};

use crate::parameters::{GainParameter, GainParameters};

slint::include_modules!();

pub struct GainPluginView {
    plugin_window: PluginWindow,
    parameters: Rc<GainParameters>,
}

impl GainPluginView {
    pub fn new(parameters: Rc<GainParameters>, host: Rc<dyn Host>) -> Self {
        let plugin_window = PluginWindow::new().unwrap();

        plugin_window.on_start_parameter_change({
            let host = host.clone();

            move |id| {
                host.start_parameter_change(id as _);
            }
        });

        plugin_window.on_change_parameter_value({
            let host = host.clone();

            move |id, value| {
                host.change_parameter_value(id as _, value as _);
            }
        });

        plugin_window.on_end_parameter_change({
            let host = host.clone();

            move |id| {
                host.end_parameter_change(id as _);
            }
        });

        plugin_window.on_change_parameter_string({
            let parameters = parameters.clone();
            let host = host.clone();

            move |id, string| {
                let parameter = parameters.get(id as ParameterId).unwrap();

                if let Some(normalized) = parameter.string_to_normalized(string.as_str()) {
                    host.start_parameter_change(id as _);
                    host.change_parameter_value(id as _, normalized);
                    host.end_parameter_change(id as _);
                }
            }
        });

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
                    id: gain_parameter.info().id() as _,
                    normalized_value: gain_parameter.normalized_value() as _,
                    default_normalized_value: gain_parameter.info().default_normalized_value() as _,
                    display_value: gain_parameter.to_string().into(),
                });
            }

            _ => {}
        }

        EventResponse::Ignored
    }
}
