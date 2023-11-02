use std::any::Any;
use std::{sync::Arc, num::NonZeroU32};

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3, nih_debug_assert_eq};
use nih_plug::prelude::*;
use nih_plug_slint::plugin_component_handle::PluginComponentHandle;
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;

const DB_MIN: f32 = -80.0;
const DB_MAX: f32 = 20.0;

slint::include_modules!();

#[derive(Params)]
pub struct PluginParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

pub struct PluginComponent {
    window: PluginWindow,
    params: Arc<PluginParams>,
}

impl PluginComponent {
    fn new(params: Arc<PluginParams>) -> Self {
        Self {
            window: PluginWindow::new().unwrap(),
            params,
        }
    }

    fn drag_event_response(&self, position: &LogicalPosition) -> EventResponse {
        self.window.set_drag_x(position.x as f32);
        self.window.set_drag_y(position.y as f32);
    
        let drop_area_x = self.window.get_drop_area_x() as f64;
        let drop_area_y = self.window.get_drop_area_y() as f64;
        let drop_area_width = self.window.get_drop_area_width() as f64;
        let drop_area_height = self.window.get_drop_area_height() as f64;

        if position.x >= drop_area_x &&
            position.x <= drop_area_x + drop_area_width &&
            position.y >= drop_area_y &&
            position.y <= drop_area_y + drop_area_height
        {
            EventResponse::DropAccepted(DropOperation::Copy)
        } else {
            EventResponse::Ignored
        }
    }
}

impl PluginComponentHandle for PluginComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn window(&self) -> &slint::Window {
        self.window.window()
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        match event {
            Event::DragEntered { position, data: _ } => {
                self.window.set_dragging(true);
                self.drag_event_response(position)
            },

            Event::DragExited => {
                self.window.set_dragging(false);
                EventResponse::Handled
            },

            Event::DragMoved { position, data: _ } => {
                self.window.set_dragging(true);
                self.drag_event_response(position)
            },

            Event::DragDropped { position, data: _ } => {
                self.window.set_dragging(false);
                self.drag_event_response(position)
            },

            _ => EventResponse::Ignored,
        }
    }

    fn update_parameter(&self, _id: &str, _update_value: bool, _update_modulation: bool) {
        // TODO: Only update changed parameter
        self.update_all_parameters();
    }

    fn update_all_parameters(&self) {
        let mut gain = self.window.global::<PluginParameters>().get_gain();
        gain.value = self.params.gain.preview_normalized(self.params.gain.value());
        self.window.global::<PluginParameters>().set_gain(gain);
    }
}

// fn update_parameter(&self, id: &str, update_value: bool, update_modulation: bool) {
//     let context = self.context.borrow();
//     let context = context.as_ref().unwrap();

//     // if let Some(param_ptr) = context.param_map.get(id) {
//     //     if let Ok(Value::Struct(mut plugin_parameter)) = context.component.get_global_property(&context.parameter_globals_name, &id) {
//     //         let value = unsafe { param_ptr.unmodulated_normalized_value() };
//     //         let modulation = unsafe { param_ptr.modulated_normalized_value() - value };

//     //         if update_value {
//     //             let display_value = unsafe { param_ptr.normalized_value_to_string(value, true) };

//     //             plugin_parameter.set_field("value".into(), Value::Number(value as f64));
//     //             plugin_parameter.set_field("display-value".into(), Value::String(display_value.into()));    
//     //             plugin_parameter.set_field("modulation".into(), Value::Number(modulation as f64));
//     //         } else if update_modulation {
//     //             plugin_parameter.set_field("modulation".into(), Value::Number(modulation as f64));
//     //         }

//     //         context.component.set_global_property(&context.parameter_globals_name, id, Value::Struct(plugin_parameter)).unwrap();
//     //     }
//     // }
// }

// fn update_all_parameters(&self) {
//     for id in self.ui_parameters.borrow().iter() {
//         self.update_parameter(id, true, true);
//     }
// }

        // Save parameter names that are used by the UI
        // let mut ui_parameters = self.ui_parameters.borrow_mut();
        // for (name, _) in context.component_definition.global_properties(&context.parameter_globals_name).unwrap() {
        //     ui_parameters.insert(name);
        // }
        // drop(ui_parameters);

        // // Set callbacks
        // let param_map = context.param_map.clone();
        // let gui_context = context.gui_context.clone();
        // context.component.set_global_callback(&context.parameter_globals_name, "start-change", move |values| {
        //     if let Value::String(name) = &values[0] {
        //         let param_ptr = param_map.get(name.as_str()).unwrap();
        //         unsafe { gui_context.raw_begin_set_parameter(param_ptr.clone()) };
        //     }

        //     Value::Void
        // }).unwrap();

        // let param_map = context.param_map.clone();
        // let gui_context = context.gui_context.clone();
        // context.component.set_global_callback(&context.parameter_globals_name, "changed", move |values| {
        //     if let (Value::String(name), Value::Number(value)) = (&values[0], &values[1]) {                
        //         let param_ptr = param_map.get(name.as_str()).unwrap();
        //         unsafe { gui_context.raw_set_parameter_normalized(param_ptr.clone(), *value as f32) };
        //     }

        //     Value::Void
        // }).unwrap();

        // let param_map = context.param_map.clone();
        // let gui_context = context.gui_context.clone();
        // context.component.set_global_callback(&context.parameter_globals_name, "end-change", move |values| {
        //     if let Value::String(name) = &values[0] {
        //         let param_ptr = param_map.get(name.as_str()).unwrap();
        //         unsafe { gui_context.raw_end_set_parameter(param_ptr.clone()) };
        //     }

        //     Value::Void
        // }).unwrap();

        // let param_map = context.param_map.clone();
        // let gui_context = context.gui_context.clone();
        // context.component.set_global_callback(&context.parameter_globals_name, "set-string", move |values| {
        //     if let (Value::String(name), Value::String(string)) = (&values[0], &values[1]) {
        //         let param_ptr = param_map.get(name.as_str()).unwrap();
        //         unsafe {
        //             if let Some(value) = param_ptr.string_to_normalized_value(string) {
        //                 gui_context.raw_begin_set_parameter(param_ptr.clone());
        //                 gui_context.raw_set_parameter_normalized(param_ptr.clone(), value);
        //                 gui_context.raw_end_set_parameter(param_ptr.clone());
        //             }
        //         }
        //     }

        //     Value::Void
        // }).unwrap();

        // // Set default values for parameters
        // if let Some(ui_plugin_parameters) = context.component_definition.global_properties(&context.parameter_globals_name) {
        //     for (name, _) in ui_plugin_parameters {
        //         if let Some(param_ptr) = context.param_map.get(&name) {
        //             let default_value = unsafe { param_ptr.default_normalized_value() };

        //             if let Ok(Value::Struct(mut plugin_parameter)) = context.component.get_global_property(&context.parameter_globals_name, &name) {
        //                 plugin_parameter.set_field("default-value".into(), Value::Number(default_value as f64));
        //                 context.component.set_global_property(&context.parameter_globals_name, &name, Value::Struct(plugin_parameter)).unwrap();
        //             }
        //         }
        //     }
        // }


pub struct DemoPlugin {
    params: Arc<PluginParams>,
}

impl Default for DemoPlugin {
    fn default() -> Self {
        let params = Arc::new(PluginParams {
            gain: FloatParam::new(
                "Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(DB_MIN),
                    max: db_to_gain(DB_MAX),
                    factor: FloatRange::gain_skew_factor(DB_MIN, DB_MAX),
                })
            .with_unit("dB")
            .with_value_to_string(nih_plug::formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(nih_plug::formatters::s2v_f32_gain_to_db()),
        });

        Self {
            params,
        }
    }
}

impl Plugin for DemoPlugin {
    type BackgroundTask = ();
    type SysExMessage = ();

    const NAME: &'static str = "Demo";
    const VENDOR: &'static str = "nih_plug_slint";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = "0.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            ..AudioIOLayout::const_default()
        }
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    const HARD_REALTIME_ONLY: bool = false;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>
    ) -> bool
    {
        true
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let window_attributes = WindowAttributes::new(
            LogicalSize::new(800.0, 600.0),
            0.75,
        );

        let editor = SlintEditor::new(
            window_attributes,
            {
                let params = self.params.clone();
                move || PluginComponent::new(params.clone())
            },
        );

        Some(Box::new(editor))
    }

    fn process(
        &mut self,
        buffer: &mut Buffer<'_>,
        _aux: &mut AuxiliaryBuffers<'_>,
        _context: &mut impl ProcessContext<Self>
    ) -> ProcessStatus
    {
        for channel in buffer.as_slice() {
            for sample in channel.iter_mut() {
                *sample *= self.params.gain.value();
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for DemoPlugin {
    const CLAP_ID: &'static str = "demo";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [nih_plug::prelude::ClapFeature] = &[];
}

impl Vst3Plugin for DemoPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"DemoDemoDemoDemo";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

nih_export_clap!(DemoPlugin);
nih_export_vst3!(DemoPlugin);
