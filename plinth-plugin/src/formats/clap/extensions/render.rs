use std::marker::PhantomData;

use clap_sys::{ext::render::{clap_plugin_render, clap_plugin_render_mode, CLAP_RENDER_OFFLINE, CLAP_RENDER_REALTIME}, plugin::clap_plugin};

use crate::{clap::{plugin_instance::PluginInstance, ClapPlugin}, ProcessMode};

#[repr(transparent)]
pub struct Render<P: ClapPlugin> {
    raw: clap_plugin_render,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> Render<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_render {
                has_hard_realtime_requirement: Some(Self::has_hard_realtime_requirement),
                set: Some(Self::set),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_render {
        &self.raw
    }
   
    unsafe extern "C" fn has_hard_realtime_requirement(_plugin: *const clap_plugin) -> bool {
        false
    }

    unsafe extern "C" fn set(plugin: *const clap_plugin, mode: clap_plugin_render_mode) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.process_mode = match mode {
                CLAP_RENDER_REALTIME => ProcessMode::Realtime,
                CLAP_RENDER_OFFLINE => ProcessMode::Offline,
                _ => { return false; },
            };

            true
        })
    }
}
