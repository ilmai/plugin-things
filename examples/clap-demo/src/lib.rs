use core::fmt::Write;
use std::{ffi::CStr, sync::Arc, iter::zip};

use clack_extensions::{audio_ports::{AudioPortInfoWriter, AudioPortInfoData, AudioPortFlags, AudioPortType}, params::{implementation::{ParamInfoWriter, ParamDisplayWriter}, info::{ParamInfoData, ParamInfoFlags}}, gui::{GuiApiType, GuiConfiguration, GuiResizeHints, GuiSize}};
use clack_plugin::{prelude::*, utils::Cookie, events::{event_types::ParamValueEvent, Event}};
use plugin_canvas::{window::WindowAttributes, LogicalSize, event::EventResponse};
use portable_atomic::{AtomicF64, Ordering};

pub struct DemoPlugin {
}

impl clack_plugin::plugin::Plugin for DemoPlugin {
    type AudioProcessor<'a> = DemoPluginAudioProcessor;
    type Shared<'a> = ();
    type MainThread<'a> = DemoPluginMainThread;

    fn get_descriptor() -> Box<dyn PluginDescriptor> {
        Box::new(StaticPluginDescriptor {
            id: CStr::from_bytes_with_nul(b"com.viiri-audio.plugin-things.clap-demo\0").unwrap(),
            name: CStr::from_bytes_with_nul(b"CLAP demo\0").unwrap(),

            ..Default::default()
        })
    }

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: &Self::Shared<'_>) {
        builder
            .register::<clack_extensions::audio_ports::PluginAudioPorts>()
            .register::<clack_extensions::gui::PluginGui>()
            .register::<clack_extensions::params::PluginParams>();
    }
}

pub struct DemoPluginParameters {
    gain: AtomicF64,
}

impl Default for DemoPluginParameters {
    fn default() -> Self {
        Self {
            gain: Default::default()
        }
    }
}

pub struct DemoPluginAudioProcessor {
    parameters: Arc<DemoPluginParameters>,
}

impl DemoPluginAudioProcessor {
    fn new(parameters: Arc<DemoPluginParameters>) -> Self {
        Self {
            parameters,
        }
    }

    fn process_event(&mut self, event: &UnknownEvent) {
        match event.header().type_id() {
            ParamValueEvent::TYPE_ID => {
                let param_value_event = event.as_event::<ParamValueEvent>().unwrap();
                assert_eq!(param_value_event.param_id(), 0);
                self.parameters.gain.store(param_value_event.value(), Ordering::Relaxed);
            },

            _ => {},
        }
    }
}


impl<'a> clack_plugin::plugin::PluginAudioProcessor<'a, (), DemoPluginMainThread> for DemoPluginAudioProcessor {
    fn activate(_host: HostAudioThreadHandle<'a>, main_thread: &mut DemoPluginMainThread, _shared: &'a (), _audio_config: AudioConfiguration) -> Result<Self, PluginError> {
        Ok(Self::new(main_thread.parameters.clone()))
    }

    fn process(&mut self, _process: Process, mut audio: Audio, events: Events) -> Result<ProcessStatus, PluginError> {
        for event in events.input {
            self.process_event(event);
        }

        let gain = 10.0_f64.powf(self.parameters.gain.load(Ordering::Relaxed) / 20.0) as f32;

        for mut port_pair in &mut audio {
            let Some(channel_pairs) = port_pair.channels()?.into_f32() else { continue; };

            for channel_pair in channel_pairs {
                match channel_pair {
                    ChannelPair::InputOnly(_) => {},
                    ChannelPair::OutputOnly(output) => output.fill(0.0),
                    ChannelPair::InputOutput(input, output) => {
                        for (input_sample, output_sample) in zip(input, output) {
                            *output_sample = *input_sample * gain;
                        }
                    },
                    ChannelPair::InPlace(_) => {},
                }
            }
        }

        Ok(ProcessStatus::Continue)
    }
}

impl clack_extensions::params::implementation::PluginAudioProcessorParams for DemoPluginAudioProcessor {
    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    )
    {
        for event in input_parameter_changes {
            self.process_event(event);
        }
    }
}

pub struct DemoPluginMainThread {
    parameters: Arc<DemoPluginParameters>,
    window_size: LogicalSize,
    os_scale: f64,
    user_scale: f64,
}

impl DemoPluginMainThread {
    fn process_event(&mut self, event: &UnknownEvent) {
        match event.header().type_id() {
            ParamValueEvent::TYPE_ID => {
                let param_value_event = event.as_event::<ParamValueEvent>().unwrap();
                assert_eq!(param_value_event.param_id(), 0);
                self.parameters.gain.store(param_value_event.value(), Ordering::Relaxed);
            },

            _ => {},
        }
    }
}

impl<'a> clack_plugin::plugin::PluginMainThread<'a, ()> for DemoPluginMainThread {
    fn new(_host: HostMainThreadHandle<'a>, _shared: &'a ()) -> Result<Self, PluginError> {
        Ok(Self {
            parameters: Default::default(),
            window_size: LogicalSize::new(0.0, 0.0),
            os_scale: 1.0,
            user_scale: 1.0,
        })
    }
}

impl clack_extensions::audio_ports::PluginAudioPortsImpl for DemoPluginMainThread {
    fn count(&self, _is_input: bool) -> u32 {
        1
    }

    fn get(&self, _is_input: bool, index: u32, writer: &mut AudioPortInfoWriter) {
        assert_eq!(index, 0);

        writer.set(&AudioPortInfoData {
            id: 0,
            name: b"Main",
            channel_count: 2,
            flags: AudioPortFlags::IS_MAIN,
            port_type: Some(AudioPortType::STEREO),
            in_place_pair: Some(0),
        });
    }
}

impl clack_extensions::gui::PluginGuiImpl for DemoPluginMainThread {
    fn is_api_supported(&self, configuration: clack_extensions::gui::GuiConfiguration) -> bool {
        if configuration.is_floating {
            return false;
        }

        Some(configuration.api_type) == GuiApiType::default_for_current_platform()
    }

    fn get_preferred_api(&self) -> Option<clack_extensions::gui::GuiConfiguration> {
        GuiApiType::default_for_current_platform()
            .and_then(|api_type| {
                Some(GuiConfiguration {
                    api_type,
                    is_floating: false,
                })            
            })
    }

    fn create(&mut self, configuration: clack_extensions::gui::GuiConfiguration) -> Result<(), clack_extensions::gui::GuiError> {
        if !self.is_api_supported(configuration) {
            return Err(clack_extensions::gui::GuiError::CreateError);
        }

        // We can't create anything without a parent, so just return
        Ok(())
    }

    fn destroy(&mut self) {
        todo!()
    }

    #[cfg(target_os = "macos")]
    fn set_scale(&mut self, _scale: f64) -> Result<(), clack_extensions::gui::GuiError> {
        return Err(clack_extensions::gui::GuiError::SetScaleError);
    }

    #[cfg(not(target_os = "macos"))]
    fn set_scale(&mut self, scale: f64) -> Result<(), clack_extensions::gui::GuiError> {
        self.os_scale = scale;
        Ok(())
    }

    fn get_size(&mut self) -> Option<clack_extensions::gui::GuiSize> {
        Some(GuiSize {
            width: self.window_size.width.round() as u32,
            height: self.window_size.height.round() as u32,
        })
    }

    fn can_resize(&self) -> bool {
        // TODO
        false
    }

    /// Provide hints on the resize-ability of the GUI
    fn get_resize_hints(&self) -> Option<GuiResizeHints> {
        // TODO
        None
    }

    fn set_size(&mut self, size: clack_extensions::gui::GuiSize) -> Result<(), clack_extensions::gui::GuiError> {
        self.window_size.width = size.width as f64;
        self.window_size.height = size.height as f64;

        Ok(())
    }

    fn set_parent(&mut self, window: clack_extensions::gui::Window) -> Result<(), clack_extensions::gui::GuiError> {
        plugin_canvas::Window::open(
            window,
            WindowAttributes::new(self.window_size.clone(), self.user_scale),
            self.os_scale,
            Box::new(|event| { EventResponse::Ignored }),
            Box::new(|window| {}),
        ).map_err(|_| clack_extensions::gui::GuiError::SetParentError)?;

        Ok(())
    }

    fn set_transient(&mut self, _window: clack_extensions::gui::Window) -> Result<(), clack_extensions::gui::GuiError> {
        // Only used for floating windows which we don't support
        Ok(())
    }

    fn show(&mut self) -> Result<(), clack_extensions::gui::GuiError> {
        todo!()
    }

    fn hide(&mut self) -> Result<(), clack_extensions::gui::GuiError> {
        todo!()
    }
}

impl clack_extensions::params::implementation::PluginMainThreadParams for DemoPluginMainThread {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        assert_eq!(param_index, 0);
        info.set(&ParamInfoData {
            id: 0,
            flags: ParamInfoFlags::IS_AUTOMATABLE |
                ParamInfoFlags::IS_MODULATABLE |
                ParamInfoFlags::REQUIRES_PROCESS,
            cookie: Cookie::default(),
            name: "Gain",
            module: "",
            min_value: -80.0,
            max_value: 0.0,
            default_value: 0.0,
        });
    }

    fn get_value(&self, param_id: u32) -> Option<f64> {
        assert_eq!(param_id, 0);
        Some(self.parameters.gain.load(Ordering::Relaxed))
    }

    fn value_to_text(
        &self,
        param_id: u32,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        assert_eq!(param_id, 0);
        write!(writer, "{value:.1} dB")
    }

    fn text_to_value(&self, param_id: u32, text: &str) -> Option<f64> {        
        assert_eq!(param_id, 0);
        text.trim_end_matches(&[' ', 'd', 'D', 'b', 'B']).parse().ok()
    }

    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    )
    {
        for event in input_parameter_changes {
            self.process_event(event);
        }
    }
}

clack_export_entry!(SinglePluginEntry<DemoPlugin>);
