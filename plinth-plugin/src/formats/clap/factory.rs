use std::{ffi::{c_char, CStr}, marker::PhantomData, ptr::null};

use clap_sys::{factory::plugin_factory::clap_plugin_factory, host::clap_host, plugin::{clap_plugin, clap_plugin_descriptor}};

use super::{descriptor::Descriptor, plugin::ClapPlugin, plugin_instance::PluginInstance};

#[repr(C)]
pub struct Factory<P: ClapPlugin> {
    raw: clap_plugin_factory,
    count: usize,

    descriptor: Descriptor,
    
    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> Factory<P> {
    pub fn new() -> Self {
        Self {
            raw: clap_plugin_factory {
                get_plugin_count: Some(Self::get_plugin_count),
                get_plugin_descriptor: Some(Self::get_plugin_descriptor),
                create_plugin: Some(Self::create_plugin),
            },
            count: 1,

            descriptor: Descriptor::new::<P>(),

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_factory {
        &self.raw
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn add_ref(&mut self) -> usize {
        self.count += 1;
        self.count
    }

    pub fn remove_ref(&mut self) -> usize {
        assert!(self.count > 0);
        self.count -= 1;
        self.count
    }

    pub unsafe fn is_valid_factory_id(factory_id: *const c_char) -> bool {
        if factory_id.is_null() {
            return false;
        }
    
        if unsafe { CStr::from_ptr(factory_id) } != ::clap_sys::factory::plugin_factory::CLAP_PLUGIN_FACTORY_ID {
            return false;
        }
    
        true
    }

    unsafe extern "C" fn get_plugin_count(_factory: *const clap_plugin_factory) -> u32 {
        1
    }
    
    unsafe extern "C" fn get_plugin_descriptor(
        factory: *const clap_plugin_factory,
        index: u32,
    ) -> *const clap_plugin_descriptor
    {
        let factory = unsafe { &*(factory as *const Self) };

        if index == 0 {
            factory.descriptor.as_raw()
        } else {
            null()
        }
    }
    
    unsafe extern "C" fn create_plugin(
        factory: *const clap_plugin_factory,
        host: *const clap_host,
        plugin_id: *const c_char,
    ) -> *const clap_plugin
    {
        let factory = &*(factory as *const Self);

        if plugin_id.is_null() {
            return null();
        }
        if CStr::from_ptr(plugin_id) != factory.descriptor.id() {
            return null();
        }

        let instance = Box::new(PluginInstance::<P>::new(&factory.descriptor, host));
        Box::into_raw(instance) as _
    }
}

unsafe impl<P: ClapPlugin> Send for Factory<P> {}
