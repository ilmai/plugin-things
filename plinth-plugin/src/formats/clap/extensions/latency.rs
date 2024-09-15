use std::marker::PhantomData;

use clap_sys::{ext::latency::clap_plugin_latency, plugin::clap_plugin};

use crate::clap::{plugin_instance::PluginInstance, ClapPlugin};

#[repr(transparent)]
pub struct Latency<P: ClapPlugin> {
    raw: clap_plugin_latency,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> Latency<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_latency {
                get: Some(Self::get),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_latency {
        &self.raw
    }
   
    unsafe extern "C" fn get(plugin: *const clap_plugin) -> u32 {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.plugin.as_ref().unwrap().latency() as _
        })
    }
}
