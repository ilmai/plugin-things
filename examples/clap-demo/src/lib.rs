use core::fmt::Write;
use std::{ffi::CStr, sync::Arc, iter::zip};

use clack_extensions::{audio_ports::{AudioPortInfoWriter, AudioPortInfoData, AudioPortFlags, AudioPortType}, params::{implementation::{ParamInfoWriter, ParamDisplayWriter}, info::{ParamInfoData, ParamInfoFlags}}};
use clack_plugin::{prelude::*, utils::Cookie, events::{event_types::ParamValueEvent, Event}};
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
}

impl<'a> clack_plugin::plugin::PluginMainThread<'a, ()> for DemoPluginMainThread {
    fn new(_host: HostMainThreadHandle<'a>, _shared: &'a ()) -> Result<Self, PluginError> {
        Ok(Self {
            parameters: Default::default(),
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
        _input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    )
    {
    }
}

clack_export_entry!(SinglePluginEntry<DemoPlugin>);
