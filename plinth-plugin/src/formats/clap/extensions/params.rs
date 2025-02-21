use std::{ffi::{c_char, CStr}, marker::PhantomData};

use clap_sys::{events::{clap_input_events, clap_output_events}, ext::params::{clap_param_info, clap_plugin_params, CLAP_PARAM_IS_AUTOMATABLE, CLAP_PARAM_IS_BYPASS, CLAP_PARAM_IS_MODULATABLE, CLAP_PARAM_IS_STEPPED}, id::clap_id, plugin::clap_plugin};

use crate::{clap::{event::EventIterator, parameters::{map_parameter_value_from_clap, map_parameter_value_to_clap}, plugin_instance::PluginInstance, ClapPlugin}, processor::Processor, string::copy_str_to_char8, Parameters};

#[repr(transparent)]
pub struct Params<P: ClapPlugin> {
    raw: clap_plugin_params,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> Params<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_params {
                count: Some(Self::count),
                get_info: Some(Self::get_info),
                get_value: Some(Self::get_value),
                value_to_text: Some(Self::value_to_text),
                text_to_value: Some(Self::text_to_value),
                flush: Some(Self::flush),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_params {
        &self.raw
    }

    unsafe extern "C" fn count(plugin: *const clap_plugin) -> u32 {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.plugin.as_ref().unwrap().with_parameters(|parameters| parameters.ids().len() as _)
        })
    }

    unsafe extern "C" fn get_info(plugin: *const clap_plugin, param_index: u32, param_info: *mut clap_param_info) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let Some(parameter_info) = instance.parameter_info.values().nth(param_index as usize) else {
                return false;
            };

            let clap_param_info = unsafe { &mut *param_info };
            clap_param_info.id = parameter_info.id();
            clap_param_info.flags = CLAP_PARAM_IS_AUTOMATABLE | CLAP_PARAM_IS_MODULATABLE;
            clap_param_info.min_value = 0.0;
            clap_param_info.default_value = map_parameter_value_to_clap(parameter_info, parameter_info.default_normalized_value());

            if parameter_info.is_bypass() {
                clap_param_info.flags |= CLAP_PARAM_IS_BYPASS;
            }

            let steps = parameter_info.steps();
            if steps > 0 {
                clap_param_info.flags |= CLAP_PARAM_IS_STEPPED;
                clap_param_info.max_value = steps as f64;
            } else {
                clap_param_info.max_value = 1.0;
            }

            copy_str_to_char8(parameter_info.name(), &mut clap_param_info.name);
            copy_str_to_char8(parameter_info.path(), &mut clap_param_info.module);

            true
        })
    }

    unsafe extern "C" fn get_value(plugin: *const clap_plugin, param_id: clap_id, out_value: *mut f64) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.process_events_to_plugin();

            instance.plugin.as_ref().unwrap().with_parameters(|parameters| {
                let Some(parameter) = parameters.get(param_id) else {
                    return false;
                };

                unsafe { *out_value = map_parameter_value_to_clap(parameter.info(), parameter.normalized_value()) };

                true
            })
        })
    }

    unsafe extern "C" fn value_to_text(plugin: *const clap_plugin, param_id: clap_id, value: f64, out_buffer: *mut c_char, out_buffer_capacity: u32) -> bool {
        if out_buffer.is_null() {
            return false;
        }

        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.plugin.as_ref().unwrap().with_parameters(|parameters| {
                let Some(parameter) = parameters.get(param_id) else {
                    return false;
                };

                let value = map_parameter_value_from_clap(parameter.info(), value);
                let string = parameter.normalized_to_string(value);

                let out_slice = unsafe { std::slice::from_raw_parts_mut(out_buffer, out_buffer_capacity as _) };
                copy_str_to_char8(&string, out_slice);

                true
            })
        })
    }

    unsafe extern "C" fn text_to_value(plugin: *const clap_plugin, param_id: clap_id, param_value_text: *const c_char, out_value: *mut f64) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.plugin.as_ref().unwrap().with_parameters(|parameters| {
                let Some(parameter) = parameters.get(param_id) else {
                    return false;
                };

                let string = unsafe { CStr::from_ptr(param_value_text) };
                let Ok(string) = string.to_str() else {
                    return false;
                };

                let Some(value) = parameter.string_to_normalized(string) else {
                    return false;
                };

                unsafe { *out_value = map_parameter_value_to_clap(parameter.info(), value) };
                true
            })
        })
    }

    unsafe extern "C" fn flush(plugin: *const clap_plugin, in_events: *const clap_input_events, out_events: *const clap_output_events) {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            instance.process_events_to_plugin();

            let mut audio_thread_state = instance.audio_thread_state.borrow_mut();
        
            let host_events = EventIterator::new(&instance.parameter_info, unsafe { &*in_events });    
            let editor_events = instance.parameter_event_map.iter_and_send_to_host(&instance.parameter_info, out_events);
            let all_events = host_events.chain(editor_events);

            if let Some(processor) = audio_thread_state.processor.as_mut() {
                // When we have a processor, process events directly
                processor.process_events(all_events);
                drop(audio_thread_state);
    
                // Also send them to the main thread through the queue
                instance.send_events_to_plugin(in_events);
    
                // Send a callback request so the main thread can process them
                unsafe { ((*instance.host).request_callback.unwrap())(instance.host); }
            } else {
                // When we don't have a processor, this is called from the main thread so we can process events directly
                for event in all_events {
                    instance.plugin.as_mut().unwrap().process_event(&event);
                }
            }
        })
    }
}
