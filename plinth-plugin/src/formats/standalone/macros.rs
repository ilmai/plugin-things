/// Generates a `main` entry point that runs the given plugin as a standalone application.
///
/// By default audio and MIDI use default configurations. To customise them call [`run_standalone`] directly:
///
/// ```rust,ignore
/// fn main() {
///     // Enumerate available audio devices for the default driver
///     let audio_devices = AudioOutputConfig::available_devices(AudioDeviceDriver::Default)
///         .expect("Failed to enumerate audio devices");
///     // Enumerate available MIDI input ports
///     let midi_ports = MidiInputConfig::available_ports()
///         .expect("Failed to enumerate MIDI ports");
///
///     let audio_config = AudioOutputConfig {
///         driver: AudioDeviceDriver::Default,
///         device_id: audio_devices.first().map(|(id, _)| id.clone()),
///         sample_rate: Some(48000),
///         buffer_size: Some(512),
///     };
///     let midi_config = MidiInputConfig {
///         port_names: Some(vec!["My MIDI Keyboard".to_string()]),
///     };
///
///     run_standalone::<MyPlugin>(audio_config, midi_config);
/// }
/// ```
#[macro_export]
macro_rules! export_standalone {
    ($plugin:ty) => {
        fn main() {
            ::plinth_plugin::standalone::run_standalone::<$plugin>(
                Default::default(),
                Default::default(),
            );
        }
    };
}
