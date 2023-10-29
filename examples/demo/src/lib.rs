use std::{sync::Arc, num::NonZeroU32, path::PathBuf};

use futures::executor::block_on;
use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3, nih_debug_assert_eq};
use nih_plug::prelude::*;
use nih_plug_slint::Scale;
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;
use slint_interpreter::{ComponentCompiler, Value, ComponentInstance};

const DB_MIN: f32 = -80.0;
const DB_MAX: f32 = 20.0;

#[derive(Params)]
pub struct PluginParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

pub struct DemoPlugin {
    params: Arc<PluginParams>,
}

impl DemoPlugin {
    fn drag_event_response(component: &ComponentInstance, position: &LogicalPosition) -> EventResponse {
        component.set_property("drag-x", Value::Number(position.x.into())).unwrap();
        component.set_property("drag-y", Value::Number(position.y.into())).unwrap();

        if let (
            Value::Number(drag_x),
            Value::Number(drag_y),
            Value::Number(drop_area_x),
            Value::Number(drop_area_y),
            Value::Number(drop_area_width),
            Value::Number(drop_area_height),
        ) = (
            component.get_property("drag-x").unwrap(),
            component.get_property("drag-y").unwrap(),
            component.get_property("drop-area-x").unwrap(),
            component.get_property("drop-area-y").unwrap(),
            component.get_property("drop-area-width").unwrap(),
            component.get_property("drop-area-height").unwrap(),
        ) {
            if drag_x >= drop_area_x &&
                drag_x <= drop_area_x + drop_area_width &&
                drag_y >= drop_area_y &&
                drag_y <= drop_area_y + drop_area_height
            {
                EventResponse::DropAccepted(DropOperation::Copy)
            } else {
                EventResponse::Ignored
            }
        } else {
            EventResponse::Ignored
        }
    }
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
            Scale::from(0.75),
        );

        let editor = SlintEditor::new(
            window_attributes,
            &self.params,
            "PluginParameters",
            || {
                let mut compiler = ComponentCompiler::new();
                compiler.set_include_paths(vec![
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../nih_plug_slint/components/"),
                ]);

                let Some(definition) = block_on(compiler.build_from_source(
                    include_str!("../main.slint").into(),
                    "../main.slint".into())
                ) else {
                    panic!("{:?}", compiler.diagnostics());
                };

                definition.create().unwrap()
            },
            |component, event| {
                match event {
                    Event::DragEntered { position, data: _ } => {
                        component.set_property("dragging", Value::Bool(true)).unwrap();
                        Self::drag_event_response(component, position)
                    },

                    Event::DragExited => {
                        component.set_property("dragging", Value::Bool(false)).unwrap();
                        EventResponse::Handled
                    },

                    Event::DragMoved { position, data: _ } => {
                        component.set_property("dragging", Value::Bool(true)).unwrap();
                        Self::drag_event_response(component, position)
                    },

                    Event::DragDropped { position, data: _ } => {
                        component.set_property("dragging", Value::Bool(false)).unwrap();
                        Self::drag_event_response(component, position)
                    },

                    _ => EventResponse::Ignored,
                }
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
