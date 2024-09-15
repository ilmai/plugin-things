use std::marker::PhantomData;

use clap_sys::{ext::audio_ports::{clap_audio_port_info, clap_plugin_audio_ports, CLAP_AUDIO_PORT_IS_MAIN, CLAP_AUDIO_PORT_REQUIRES_COMMON_SAMPLE_SIZE, CLAP_PORT_STEREO}, id::CLAP_INVALID_ID, plugin::clap_plugin};

use crate::{clap::ClapPlugin, string::copy_str_to_char8};

#[repr(C)]
pub struct AudioPorts<P: ClapPlugin> {
    raw: clap_plugin_audio_ports,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> AudioPorts<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_audio_ports {
                count: Some(Self::count),
                get: Some(Self::get),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> &clap_plugin_audio_ports {
        &self.raw
    }

    // Number of ports, for either input or output
    // [main-thread]
    unsafe extern "C" fn count(_plugin: *const clap_plugin, is_input: bool) -> u32 {
        if is_input && P::HAS_AUX_INPUT {
            2
        } else {
            1
        }
    }

    // Get info about an audio port.
    // Returns true on success and stores the result into info.
    // [main-thread]
    unsafe extern "C" fn get(
        _plugin: *const clap_plugin,
        index: u32,
        is_input: bool,
        info: *mut clap_audio_port_info,
    ) -> bool
    {
        let info = unsafe { &mut *info };

        info.id = index;
        info.channel_count = 2;
        info.port_type = CLAP_PORT_STEREO.as_ptr();

        if index == 0 {
            info.flags = CLAP_AUDIO_PORT_IS_MAIN | CLAP_AUDIO_PORT_REQUIRES_COMMON_SAMPLE_SIZE;
            info.in_place_pair = 0;

            copy_str_to_char8("Main", &mut info.name);
        } else {
            assert!(index == 1 && is_input && P::HAS_AUX_INPUT);
            info.flags = 0;
            info.in_place_pair = CLAP_INVALID_ID;

            copy_str_to_char8("Aux", &mut info.name);
        }

        true
    }
}
