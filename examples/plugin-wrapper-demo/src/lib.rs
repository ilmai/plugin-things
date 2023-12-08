use std::ffi::CStr;

use clack_plugin::{clack_export_entry, entry::SinglePluginEntry, plugin::descriptor::StaticPluginDescriptor, prelude::PluginDescriptor};
use plugin_wrapper::ClapPlugin;

pub struct DemoPlugin {

}

impl ClapPlugin for DemoPlugin {

}

impl clack_plugin::plugin::Plugin for DemoPlugin {
    type AudioProcessor<'a> = ();
    type Shared<'a> = ();
    type MainThread<'a> = ();

    fn get_descriptor() -> Box<dyn PluginDescriptor> {
        Box::new(StaticPluginDescriptor {
            id: CStr::from_bytes_with_nul(b"com.viiri-audio.plugin-things.demo\0").unwrap(),
            name: CStr::from_bytes_with_nul(b"plugin-wrapper demo\0").unwrap(),
            // vendor: todo!(),
            // url: todo!(),
            // manual_url: todo!(),
            // support_url: todo!(),
            // version: todo!(),
            // description: todo!(),
            // features: todo!(),

            ..Default::default()
        })
    }
}

clack_export_entry!(SinglePluginEntry<DemoPlugin>);
