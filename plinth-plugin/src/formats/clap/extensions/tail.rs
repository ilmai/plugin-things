use std::marker::PhantomData;

use clap_sys::{ext::tail::clap_plugin_tail, plugin::clap_plugin};

use crate::clap::{plugin_instance::PluginInstance, ClapPlugin};

#[repr(transparent)]
pub struct Tail<P: ClapPlugin> {
    raw: clap_plugin_tail,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> Tail<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_tail {
                get: Some(Self::get),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_tail {
        &self.raw
    }

    unsafe extern "C" fn get(plugin: *const clap_plugin) -> u32 {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let audio_thread_state = instance.audio_thread_state.borrow();
            audio_thread_state.tail as _
        })
    }
}
