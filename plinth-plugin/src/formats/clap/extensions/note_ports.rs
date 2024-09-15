use std::marker::PhantomData;

use clap_sys::{ext::note_ports::{clap_note_port_info, clap_plugin_note_ports, CLAP_NOTE_DIALECT_CLAP}, plugin::clap_plugin};

use crate::{clap::ClapPlugin, string::copy_str_to_char8};

#[repr(C)]
pub struct NotePorts<P: ClapPlugin> {
    raw: clap_plugin_note_ports,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> NotePorts<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_note_ports {
                count: Some(Self::count),
                get: Some(Self::get),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> &clap_plugin_note_ports {
        &self.raw
    }

    // Number of ports, for either input or output
    // [main-thread]
    unsafe extern "C" fn count(_plugin: *const clap_plugin, is_input: bool) -> u32 {
        if is_input && P::HAS_NOTE_INPUT {
            1
        } else if !is_input && P::HAS_NOTE_OUTPUT {
            1
        } else {
            0
        }
    }

    // Get info about an audio port.
    // Returns true on success and stores the result into info.
    // [main-thread]
    unsafe extern "C" fn get(
        _plugin: *const clap_plugin,
        index: u32,
        _is_input: bool,
        info: *mut clap_note_port_info,
    ) -> bool
    {
        let info = unsafe { &mut *info };

        info.id = index;
        info.supported_dialects = CLAP_NOTE_DIALECT_CLAP;
        info.preferred_dialect = CLAP_NOTE_DIALECT_CLAP;
        copy_str_to_char8("Main", &mut info.name);

        true
    }
}
