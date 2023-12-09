use std::ffi::CStr;

use clack_extensions::audio_ports::{AudioPortInfoWriter, AudioPortInfoData, AudioPortFlags, AudioPortType};
use clack_plugin::prelude::*;

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
        builder.register::<clack_extensions::audio_ports::PluginAudioPorts>();
    }
}

pub struct DemoPluginAudioProcessor;

impl<'a> clack_plugin::plugin::PluginAudioProcessor<'a, (), DemoPluginMainThread> for DemoPluginAudioProcessor {
    fn activate(_host: HostAudioThreadHandle<'a>, _main_thread: &mut DemoPluginMainThread, _shared: &'a (), _audio_config: AudioConfiguration) -> Result<Self, PluginError> {
        Ok(Self)
    }

    fn process(&mut self, _process: Process, mut audio: Audio, _events: Events) -> Result<ProcessStatus, PluginError> {
        for mut port_pair in &mut audio {
            let Some(channel_pairs) = port_pair.channels()?.into_f32() else { continue; };

            for channel_pair in channel_pairs {
                match channel_pair {
                    ChannelPair::InputOnly(_) => {},
                    ChannelPair::OutputOnly(output) => output.fill(0.0),
                    ChannelPair::InputOutput(input, output) => {
                        output.copy_from_slice(input);
                    },
                    ChannelPair::InPlace(_) => {},
                }
            }
        }

        Ok(ProcessStatus::Continue)
    }
}

pub struct DemoPluginMainThread;

impl<'a> clack_plugin::plugin::PluginMainThread<'a, ()> for DemoPluginMainThread {
    fn new(_host: HostMainThreadHandle<'a>, _shared: &'a ()) -> Result<Self, PluginError> {
        Ok(Self)
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

clack_export_entry!(SinglePluginEntry<DemoPlugin>);
