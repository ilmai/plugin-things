#[macro_export]
macro_rules! export_auv3 {
    ($plugin:ty) => {
        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_create() -> *mut ::std::ffi::c_void {
            use ::plinth_plugin::auv3::Auv3Plugin;
           
            let wrapper = Box::new(::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::new());

            // Log after creating the plugin since it will probably create loggers, if any
            log::trace!("plinth_auv3_create() from thread {:?}", std::thread::current().id());

            Box::into_raw(wrapper) as _
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_destroy(wrapper: *mut ::std::ffi::c_void) {
            log::trace!("plinth_auv3_destroy() from thread {:?}", std::thread::current().id());

            assert!(!wrapper.is_null());
            let wrapper = unsafe { Box::from_raw(wrapper as *mut ::plinth_plugin::auv3::Auv3Wrapper<$plugin>) };
            drop(wrapper);
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_activate(wrapper: *mut ::std::ffi::c_void, sample_rate: f64, max_block_size: u64) {
            log::trace!("plinth_auv3_activate() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                wrapper.activate(sample_rate, max_block_size)
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_deactivate(wrapper: *mut ::std::ffi::c_void) {
            log::trace!("plinth_auv3_deactivate() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| wrapper.deactivate());
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_tail_length(wrapper: *mut ::std::ffi::c_void) -> f64 {
            log::trace!("plinth_auv3_tail_length() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| wrapper.tail_length())
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_has_aux_bus() -> bool {
            log::trace!("plinth_auv3_has_aux_bus() from thread {:?}", std::thread::current().id());
            <$plugin>::HAS_AUX_INPUT
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_process(
            wrapper: *mut ::std::ffi::c_void,
            input: *const *const f32,
            aux: *const *const f32,
            output: *mut *mut f32,
            channels: u32,
            frames: u32,
            playing: bool,
            tempo: f64,
            position_samples: i64,
            first_event: *const ::plinth_plugin::auv3::AURenderEvent,
        ) {
            log::trace!("plinth_auv3_process() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.process(input, aux, output, channels, frames, playing, tempo, position_samples, first_event ) };
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_parameter_count(wrapper: *mut ::std::ffi::c_void) -> u64 {
            log::trace!("plinth_auv3_parameter_count() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| wrapper.parameter_count())
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_parameter_info(
            wrapper: *mut ::std::ffi::c_void,
            index: usize,
            info: *mut ::plinth_plugin::auv3::ParameterInfo
        ) {
            log::trace!("plinth_auv3_parameter_info() from thread {:?}", std::thread::current().id());

            let info = unsafe { &mut *info };

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.parameter_info(index, info); }
            })
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_group_count(wrapper: *mut ::std::ffi::c_void) -> u64 {
            log::trace!("plinth_auv3_group_count() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| wrapper.group_count())
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_group_info(
            wrapper: *mut ::std::ffi::c_void,
            index: usize,
            info: *mut ::plinth_plugin::auv3::ParameterGroupInfo
        ) {
            log::trace!("plinth_auv3_group_info() from thread {:?}", std::thread::current().id());

            let info = unsafe { &mut *info };

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.group_info(index, info); }
            });
        }
    
        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_get_parameter_value(wrapper: *mut ::std::ffi::c_void, address: u64) -> f32 {
            log::trace!("plinth_auv3_get_parameter_value() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| wrapper.parameter_value(address))
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_set_parameter_value(wrapper: *mut ::std::ffi::c_void, address: u64, value: f32) {
            log::trace!("plinth_auv3_set_parameter_value() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                wrapper.set_parameter_value(address, value)
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_parameter_normalized_to_string(
            wrapper: *mut ::std::ffi::c_void,
            address: u64,
            value: f32,
            string: *mut ::std::ffi::c_char
        ) {
            log::trace!("plinth_auv3_parameter_normalized_to_string() from thread {:?}", std::thread::current().id());

            assert!(!string.is_null());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.normalized_parameter_to_string(address, value, string); }
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_load_state(
            wrapper: *mut ::std::ffi::c_void,
            context: *mut ::std::ffi::c_void,
            read: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *mut u8, usize) -> usize,
        ) {            
            log::trace!("plinth_auv3_load_state() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.load_state(context, read); }
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_save_state(
            wrapper: *mut ::std::ffi::c_void,
            context: *mut ::std::ffi::c_void,
            write: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *const u8, usize) -> usize,
        ) {            
            log::trace!("plinth_auv3_save_state() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.save_state(context, write); }
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_editor_create(
            wrapper: *mut ::std::ffi::c_void,
            context: *mut ::std::ffi::c_void,
            start_parameter_change: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, ::plinth_plugin::ParameterId),
            change_parameter_value: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, ::plinth_plugin::ParameterId, f32),
            end_parameter_change: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, ::plinth_plugin::ParameterId),
        ) {
            log::trace!("plinth_auv3_editor_create() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.create_editor(context, start_parameter_change, change_parameter_value, end_parameter_change); }
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_editor_get_default_size(
            width: *mut f64,
            height: *mut f64
        )
        {
            use ::plinth_plugin::Editor;

            log::trace!("plinth_auv3_editor_get_default_size() from thread {:?}", std::thread::current().id());

            let size = <$plugin as ::plinth_plugin::Plugin>::Editor::DEFAULT_SIZE;

            unsafe {
                *width = size.0;
                *height = size.1;
            }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_editor_get_size(
            wrapper: *mut ::std::ffi::c_void,
            width: *mut f64,
            height: *mut f64
        )
        {
            log::trace!("plinth_auv3_editor_get_size() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let (preferred_width, preferred_height) = wrapper.window_size();

                unsafe {
                    *width = preferred_width;
                    *height = preferred_height;
                }
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_editor_set_size(wrapper: *mut ::std::ffi::c_void, width: f64, height: f64) {
            log::trace!("plinth_auv3_editor_set_size() from thread {:?}", std::thread::current().id());

            use ::plinth_plugin::Editor;

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                wrapper.set_window_size(width, height);
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_editor_open(
            wrapper: *mut ::std::ffi::c_void,
            parent: *mut ::std::ffi::c_void,
        ) {
            log::trace!("plinth_auv3_editor_open() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                unsafe { wrapper.open_editor(parent); }
            });
        }

        #[unsafe(no_mangle)]
        unsafe extern "C-unwind" fn plinth_auv3_editor_close(wrapper: *mut ::std::ffi::c_void) {
            log::trace!("plinth_auv3_editor_close() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| wrapper.close_editor());
        }
    }
}
