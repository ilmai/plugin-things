use std::marker::PhantomData;

use clap_sys::{ext::state::clap_plugin_state, plugin::clap_plugin, stream::{clap_istream, clap_ostream}};

use crate::clap::{plugin_instance::PluginInstance, stream::{InputStream, OutputStream}, ClapPlugin};

#[repr(transparent)]
pub struct State<P: ClapPlugin> {
    raw: clap_plugin_state,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> State<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_state {
                save: Some(Self::save),
                load: Some(Self::load),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_state {
        &self.raw
    }

    unsafe extern "C" fn save(plugin: *const clap_plugin, stream: *const clap_ostream) -> bool {
        let mut stream = OutputStream::new(stream);

        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.process_events_to_plugin();

            match instance.plugin.as_ref().unwrap().save_state(&mut stream) {
                Ok(_) => true,
                Err(e) => {
                    log::error!("Error saving state: {:?}", e);
                    false
                },
            }
        })
    }

    unsafe extern "C" fn load(plugin: *const clap_plugin, stream: *const clap_istream) -> bool {
        let mut stream = InputStream::new(stream);

        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.process_events_to_plugin();

            match instance.plugin.as_mut().unwrap().load_state(&mut stream) {
                Ok(_) => true,
                Err(e) => {
                    log::error!("Error loading state: {e:?}");
                    false
                }
            }
        })
    }
}
