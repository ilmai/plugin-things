use std::any::Any;
use std::collections::HashMap;
use std::{sync::Arc, num::NonZeroU32};

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3, nih_debug_assert_eq};
use nih_plug::prelude::*;
use nih_plug_slint::plugin_component_handle::PluginComponentHandle;
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;
use slint::SharedString;

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
    param_map: HashMap<SharedString, ParamPtr>,
}

impl PluginComponent {
    fn new(params: Arc<PluginParams>, gui_context: Arc<dyn GuiContext>) -> Self {
        let window = PluginWindow::new().unwrap();

        let param_map: HashMap<SharedString, _> = params.param_map().iter()
            .map(|(name, param_ptr, _)| {
                (name.clone().into(), *param_ptr)
            })
            .collect();

        window.on_start_change({
            let gui_context = gui_context.clone();
            let param_map = param_map.clone();

            move |parameter_id| {
                let param_ptr = param_map.get(&parameter_id).unwrap();
                unsafe { gui_context.raw_begin_set_parameter(*param_ptr) };
            }
        });

        window.on_changed({
            let gui_context = gui_context.clone();
            let param_map = param_map.clone();

            move |parameter_id, value| {
                let param_ptr = param_map.get(&parameter_id).unwrap();
                unsafe { gui_context.raw_set_parameter_normalized(*param_ptr, value) };
            }
        });

        window.on_end_change({
            let gui_context = gui_context.clone();
            let param_map = param_map.clone();

            move |parameter_id| {
                let param_ptr = param_map.get(&parameter_id).unwrap();
                unsafe { gui_context.raw_end_set_parameter(*param_ptr) };
            }
        });

        window.on_set_string({
            let gui_context = gui_context.clone();
            let param_map = param_map.clone();

            move |parameter_id, string| {
                let param_ptr = param_map.get(&parameter_id).unwrap();
                unsafe {
                    if let Some(value) = param_ptr.string_to_normalized_value(&string) {
                        gui_context.raw_begin_set_parameter(*param_ptr);
                        gui_context.raw_set_parameter_normalized(*param_ptr, value);
                        gui_context.raw_end_set_parameter(*param_ptr);
                    }    
                }
            }
        });

        Self {
            window,
            param_map,
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

    fn convert_parameter(&self, id: &str) -> PluginParameter {
        let param_ptr = self.param_map.get(id.into()).unwrap();

        let value = unsafe { param_ptr.unmodulated_normalized_value() };
        let default_value = unsafe { param_ptr.default_normalized_value() };
        let display_value = unsafe { param_ptr.normalized_value_to_string(value, true) };
        let modulated_value = unsafe { param_ptr.modulated_normalized_value() };

        PluginParameter {
            default_value,
            display_value: display_value.into(),
            modulated_value,
            value,
        }
    }

    fn set_parameter(&self, id: &str, parameter: PluginParameter) {
        match id {
            "gain" => self.window.set_gain(parameter),
            _ => unimplemented!(),
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
                self.drag_event_response(position)
            },

            Event::DragDropped { position, data: _ } => {
                self.window.set_dragging(false);
                self.drag_event_response(position)
            },

            _ => EventResponse::Ignored,
        }
    }

    fn update_parameter(&self, id: &str, _update_value: bool, _update_modulation: bool) {
        let parameter = self.convert_parameter(id);
        self.set_parameter(id, parameter);
    }

    fn update_all_parameters(&self) {
        for id in self.param_map.keys() {
            self.update_parameter(id, true, true);
        }
    }
}

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
                move |gui_context| PluginComponent::new(params.clone(), gui_context)
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
