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

impl PluginWindow {
    fn drag_event_response(&self, position: &LogicalPosition) -> EventResponse {
        self.set_drag_x(position.x as f32);
        self.set_drag_y(position.y as f32);
    
        let drop_area_x = self.get_drop_area_x() as f64;
        let drop_area_y = self.get_drop_area_y() as f64;
        let drop_area_width = self.get_drop_area_width() as f64;
        let drop_area_height = self.get_drop_area_height() as f64;

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

impl PluginComponentHandle for PluginWindow {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn window(&self) -> &slint::Window {
        slint::ComponentHandle::window(self)
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        match event {
            Event::DragEntered { position, data: _ } => {
                self.set_dragging(true);
                self.drag_event_response(position)
            },

            Event::DragExited => {
                self.set_dragging(false);
                EventResponse::Handled
            },

            Event::DragMoved { position, data: _ } => {
                self.set_dragging(true);
                self.drag_event_response(position)
            },

            Event::DragDropped { position, data: _ } => {
                self.set_dragging(false);
                self.drag_event_response(position)
            },

            _ => EventResponse::Ignored,
        }
    }

    fn update_parameter(&self, _id: &str, _update_value: bool, _update_modulation: bool) {
    }

    fn update_all_parameters(&self) {
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
            || PluginWindow::new().unwrap(),
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
