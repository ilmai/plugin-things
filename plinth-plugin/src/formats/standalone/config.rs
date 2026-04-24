use cpal::traits::{DeviceTrait, HostTrait};
use midir::MidiInput;

/// Available audio backends for [`StandalonePlugin`].
#[derive(Debug, Default, Clone, Copy)]
pub enum AudioDeviceDriver {
    #[default]
    Default,
    #[cfg(target_os = "windows")]
    Asio,
    #[cfg(target_os = "windows")]
    Wasapi,
    #[cfg(target_os = "linux")]
    Alsa,
    #[cfg(target_os = "macos")]
    CoreAudio,
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    Jack,
}

impl AudioDeviceDriver {
    fn open(self) -> Result<cpal::Host, Box<dyn std::error::Error>> {
        match self {
            AudioDeviceDriver::Default => Ok(cpal::default_host()),
            #[cfg(target_os = "windows")]
            AudioDeviceDriver::Asio => Ok(cpal::host_from_id(cpal::HostId::Asio)?),
            #[cfg(target_os = "windows")]
            AudioDeviceDriver::Wasapi => Ok(cpal::host_from_id(cpal::HostId::Wasapi)?),
            #[cfg(target_os = "linux")]
            AudioDeviceDriver::Alsa => Ok(cpal::host_from_id(cpal::HostId::Alsa)?),
            #[cfg(target_os = "macos")]
            AudioDeviceDriver::CoreAudio => Ok(cpal::host_from_id(cpal::HostId::CoreAudio)?),
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            AudioDeviceDriver::Jack => Ok(cpal::host_from_id(cpal::HostId::Jack)?),
        }
    }
}

/// Audio output configuration for [`StandalonePlugin`].
#[derive(Debug, Default)]
pub struct AudioOutputConfig {
    /// Audio host/driver to use. Defaults to `cpal::default_host`.
    pub driver: AudioDeviceDriver,
    /// Id of the output device to open. `None` selects the driver's default device.
    pub device_id: Option<cpal::DeviceId>,
    /// Desired sample rate in Hz. `None` uses the device's default rate.
    pub sample_rate: Option<u32>,
    /// Audio buffer size in frames. `None` uses the device's default buffer size.
    pub buffer_size: Option<u32>,
}

impl AudioOutputConfig {
    const PREFERRED_SAMPLE_RATE: cpal::SampleRate = 44100;
    const PREFERRED_CHANNELS: cpal::ChannelCount = 2;
    const PREFERRED_SAMPLE_FORMAT: cpal::SampleFormat = cpal::SampleFormat::F32;

    /// Returns all audio drivers available on this platform.
    ///
    /// Always includes [`AudioDeviceDriver::Default`], followed by any named drivers that are
    /// currently available (e.g. ASIO, WASAPI on Windows; ALSA, JACK on Linux).
    pub fn available_drivers() -> Vec<AudioDeviceDriver> {
        let hosts = cpal::available_hosts();
        let mut drivers = vec![AudioDeviceDriver::Default];
        #[cfg(target_os = "windows")]
        if hosts.contains(&cpal::HostId::Asio) {
            drivers.push(AudioDeviceDriver::Asio);
        }
        #[cfg(target_os = "windows")]
        if hosts.contains(&cpal::HostId::Wasapi) {
            drivers.push(AudioDeviceDriver::Wasapi);
        }
        #[cfg(target_os = "linux")]
        if hosts.contains(&cpal::HostId::Alsa) {
            drivers.push(AudioDeviceDriver::Alsa);
        }
        #[cfg(target_os = "macos")]
        if hosts.contains(&cpal::HostId::CoreAudio) {
            drivers.push(AudioDeviceDriver::CoreAudio);
        }
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        if hosts.contains(&cpal::HostId::Jack) {
            drivers.push(AudioDeviceDriver::Jack);
        }
        drivers
    }

    /// Returns `(id, name)`s of all output devices available for the given driver.
    pub fn available_devices(
        driver: AudioDeviceDriver,
    ) -> Result<Vec<(cpal::DeviceId, String)>, Box<dyn std::error::Error>> {
        let host = driver.open()?;
        let mut devices = Vec::new();
        for device in host.output_devices()? {
            match (device.id(), device.description()) {
                (Ok(id), Ok(description)) => {
                    devices.push((id, description.to_string()));
                }
                (Ok(id), Err(_)) => {
                    devices.push((id.clone(), id.to_string()));
                }
                (Err(err), _) => {
                    log::warn!("Failed to query audio device id {err}")
                }
            }
        }
        Ok(devices)
    }

    pub fn open_host(&self) -> Result<cpal::Host, Box<dyn std::error::Error>> {
        self.driver.open()
    }

    pub fn open_device(
        &self,
        host: &mut cpal::Host,
    ) -> Result<cpal::Device, Box<dyn std::error::Error>> {
        if let Some(device_id) = &self.device_id {
            log::info!("Opening CPAL output device '{}'...", device_id);
            host.output_devices()?
                .find(|d| d.id().ok().as_ref() == Some(device_id))
                .ok_or_else(|| "Specified audio device not found".into())
        } else {
            log::info!("Opening CPAL default output device...");
            host.default_output_device()
                .ok_or_else(|| "No audio output device available".into())
        }
    }

    pub fn select_stream_config(
        &self,
        device: &cpal::Device,
    ) -> Result<cpal::SupportedStreamConfig, Box<dyn std::error::Error>> {
        let target_rate = self.sample_rate.unwrap_or(Self::PREFERRED_SAMPLE_RATE);
        let mut configs = device.supported_output_configs()?.collect::<Vec<_>>();
        configs.sort_by(|a, b| b.cmp_default_heuristics(a));
        let supports_rate = |s: &cpal::SupportedStreamConfigRange| {
            (s.min_sample_rate()..=s.max_sample_rate()).contains(&target_rate)
        };
        let best_match = configs
            .iter()
            .find(|s| {
                supports_rate(s)
                    && s.channels() == Self::PREFERRED_CHANNELS
                    && s.sample_format() == Self::PREFERRED_SAMPLE_FORMAT
            })
            .or_else(|| {
                configs
                    .iter()
                    .find(|s| supports_rate(s) && s.channels() == Self::PREFERRED_CHANNELS)
            })
            .or_else(|| configs.iter().find(|s| supports_rate(s)));
        match best_match {
            Some(s) => Ok(s.with_sample_rate(target_rate)),
            None => {
                log::warn!("No matching audio device config found, using device default");
                Ok(device.default_output_config()?)
            }
        }
    }
}

/// MIDI input configuration for [`StandalonePlugin`].
#[derive(Debug, Default)]
pub struct MidiInputConfig {
    /// Names of MIDI input ports to connect to. `None` connects to all available ports.
    pub port_names: Option<Vec<String>>,
}

impl MidiInputConfig {
    /// Returns the names of all currently available MIDI input ports.
    pub fn available_ports() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let midi_in = MidiInput::new("plinth-standalone")?;
        let ports = midi_in.ports();
        let names = ports
            .iter()
            .filter_map(|p| midi_in.port_name(p).ok())
            .collect();
        Ok(names)
    }
}
