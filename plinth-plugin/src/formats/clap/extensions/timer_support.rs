use std::marker::PhantomData;

use clap_sys::{ext::timer_support::clap_plugin_timer_support, id::clap_id, plugin::clap_plugin};

use crate::clap::{plugin_instance::PluginInstance, ClapPlugin};
use crate::editor::Editor;

#[repr(transparent)]
pub struct TimerSupport<P: ClapPlugin> {
    raw: clap_plugin_timer_support,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> TimerSupport<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_timer_support {
                on_timer: Some(Self::on_timer),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_timer_support {
        &self.raw
    }
   
    unsafe extern "C" fn on_timer(plugin: *const clap_plugin, _timer_id: clap_id) {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            if let Some(editor) = instance.editor.as_mut() {
                editor.on_frame();
            }
        })
    }
}
