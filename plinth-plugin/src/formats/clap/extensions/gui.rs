use std::{ffi::{c_char, CStr}, marker::PhantomData, rc::Rc};

use clap_sys::{ext::gui::{clap_gui_resize_hints, clap_plugin_gui, clap_window}, plugin::clap_plugin};

use crate::{clap::{host::ClapHost, plugin_instance::PluginInstance, ClapPlugin}, Editor};

#[repr(transparent)]
pub struct Gui<P: ClapPlugin> {
    raw: clap_plugin_gui,

    _phantom_plugin: PhantomData<P>,
}

impl<P: ClapPlugin> Gui<P> {
    pub const fn new() -> Self {
        Self {
            raw: clap_plugin_gui {
                is_api_supported: Some(Self::is_api_supported),
                get_preferred_api: Some(Self::get_preferred_api),
                create: Some(Self::create),
                destroy: Some(Self::destroy),
                set_scale: Some(Self::set_scale),
                get_size: Some(Self::get_size),
                can_resize: Some(Self::can_resize),
                get_resize_hints: Some(Self::get_resize_hints),
                adjust_size: Some(Self::adjust_size),
                set_size: Some(Self::set_size),
                set_parent: Some(Self::set_parent),
                set_transient: Some(Self::set_transient),
                suggest_title: Some(Self::suggest_title),
                show: Some(Self::show),
                hide: Some(Self::hide),
            },

            _phantom_plugin: PhantomData,
        }
    }

    pub fn as_raw(&self) -> *const clap_plugin_gui {
        &self.raw
    }

    #[cfg(target_os="linux")]
    unsafe extern "C" fn is_api_supported(_plugin: *const clap_plugin, api: *const c_char, is_floating: bool) -> bool {
        if is_floating {
            return false;
        }

        unsafe { CStr::from_ptr(api) == clap_sys::ext::gui::CLAP_WINDOW_API_X11 }
    }

    #[cfg(target_os="macos")]
    unsafe extern "C" fn is_api_supported(_plugin: *const clap_plugin, api: *const c_char, is_floating: bool) -> bool {
        if is_floating {
            return false;
        }

        unsafe { CStr::from_ptr(api) == clap_sys::ext::gui::CLAP_WINDOW_API_COCOA }
    }

    #[cfg(target_os="windows")]
    unsafe extern "C" fn is_api_supported(_plugin: *const clap_plugin, api: *const c_char, is_floating: bool) -> bool {
        if is_floating {
            return false;
        }

        unsafe { CStr::from_ptr(api) == clap_sys::ext::gui::CLAP_WINDOW_API_WIN32 }
    }

    unsafe extern "C" fn get_preferred_api(_plugin: *const clap_plugin, _api: *mut *const c_char, _is_floating: *mut bool) -> bool {
        false
    }

    unsafe extern "C" fn create(plugin: *const clap_plugin, _api: *const c_char, is_floating: bool) -> bool {
        if is_floating {
            return false;
        }

        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let host = Rc::new(ClapHost::new(
                instance.host,
                instance.host_ext_params,
                instance.host_ext_state,
                instance.parameter_event_map.clone(),
            ));

            instance.editor = Some(instance.plugin.as_mut().unwrap().create_editor(host));

            #[cfg(target_os="linux")]
            if !instance.host_ext_timer_support.is_null() {
                let mut timer_id = 0;
                unsafe { ((*instance.host_ext_timer_support).register_timer.unwrap())(instance.host, crate::editor::FRAME_TIMER_MILLISECONDS as u32, &mut timer_id) };
                instance.timer_id = Some(timer_id);
            }
        });

        true
    }

    unsafe extern "C" fn destroy(plugin: *const clap_plugin) {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            #[cfg(target_os="linux")]
            if let Some(timer_id) = instance.timer_id {
                if !instance.host_ext_timer_support.is_null() {
                    unsafe { ((*instance.host_ext_timer_support).unregister_timer.unwrap())(instance.host, timer_id) };
                }
            }

            instance.editor = None;
        });
    }

    unsafe extern "C" fn set_scale(plugin: *const clap_plugin, scale: f64) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let Some(editor) = instance.editor.as_mut() else {
                return false;
            };

            editor.set_scale(scale);
            true
        })
    }

    unsafe extern "C" fn get_size(plugin: *const clap_plugin, width: *mut u32, height: *mut u32) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let Some(editor) = instance.editor.as_ref() else {
                return false;
            };

            let editor_size = editor.window_size();
            
            unsafe {
                (*width) = editor_size.0 as u32;
                (*height) = editor_size.1 as u32;
            }

            true
        });

        true
    }

    unsafe extern "C" fn can_resize(plugin: *const clap_plugin) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let Some(editor) = instance.editor.as_ref() else {
                return false;
            };

            editor.can_resize()
        })
    }
    
    unsafe extern "C" fn get_resize_hints(_plugin: *const clap_plugin, _hints: *mut clap_gui_resize_hints) -> bool {
        false
    }

    unsafe extern "C" fn adjust_size(plugin: *const clap_plugin, width: *mut u32, height: *mut u32) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let Some(editor) = instance.editor.as_ref() else {
                return false;
            };

            let requested_size = unsafe { (*width as _, *height as _) };
            let Some(new_size) = editor.check_window_size(requested_size) else {
                return false;
            };

            unsafe {
                *width = new_size.0 as _;
                *height = new_size.1 as _;    
            }

            true
        })
    }

    unsafe extern "C" fn set_size(plugin: *const clap_plugin, width: u32, height: u32) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let Some(editor) = instance.editor.as_mut() else {
                return false;
            };

            editor.set_window_size(width as _, height as _);

            true
        })
    }

    unsafe extern "C" fn set_parent(plugin: *const clap_plugin, window: *const clap_window) -> bool {
        PluginInstance::with_plugin_instance(plugin, |instance: &mut PluginInstance<P>| {
            let parent = crate::window_handle::from_ptr(unsafe { (*window).specific.ptr });

            let Some(editor) = instance.editor.as_mut() else {
                return false;
            };

            editor.open(parent);
            
            true
        })
    }

    unsafe extern "C" fn set_transient(_plugin: *const clap_plugin, _window: *const clap_window) -> bool {
        false
    }
    
    unsafe extern "C" fn suggest_title(_plugin: *const clap_plugin, _title: *const c_char) {
    }

    unsafe extern "C" fn show(_plugin: *const clap_plugin) -> bool {
        // TODO
        true
    }

    unsafe extern "C" fn hide(_plugin: *const clap_plugin) -> bool {
        // TODO
        true
    }
}
