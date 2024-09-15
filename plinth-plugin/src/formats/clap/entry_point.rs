use std::ffi::{c_char, c_void};

use clap_sys::{entry::clap_plugin_entry, version::CLAP_VERSION};

#[repr(transparent)]
pub struct EntryPoint {
    _raw: clap_plugin_entry,    
}

impl EntryPoint {
    pub const fn new(
        init: unsafe extern "C" fn(plugin_path: *const c_char) -> bool,
        deinit: unsafe extern "C" fn(),
        get_factory: unsafe extern "C" fn(factory_id: *const c_char) -> *const c_void,
    ) -> Self {
        Self {
            _raw: clap_plugin_entry {
                clap_version: CLAP_VERSION,
                init: Some(init),
                deinit: Some(deinit),
                get_factory: Some(get_factory),
            }
        }
    }
}
