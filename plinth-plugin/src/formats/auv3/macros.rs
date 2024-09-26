#[macro_export]
macro_rules! export_auv3 {
    ($plugin:ty) => {
        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_create() -> *mut ::std::ffi::c_void {
            use ::plinth_plugin::auv3::Auv3Plugin;
           
            let wrapper = Box::new(::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::new());

            // Log after creating the plugin since it will probably create loggers, if any
            log::trace!("plinth_auv3_create() from thread {:?}", std::thread::current().id());

            Box::into_raw(wrapper) as _
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_destroy(wrapper: *mut ::std::ffi::c_void) {
            log::trace!("plinth_auv3_destroy() from thread {:?}", std::thread::current().id());

            assert!(!wrapper.is_null());
            let wrapper = Box::from_raw(wrapper as *mut ::plinth_plugin::auv3::Auv3Wrapper<$plugin>);
            drop(wrapper);
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_activate(wrapper: *mut ::std::ffi::c_void, sample_rate: f64, max_block_size: u64) {
            log::trace!("plinth_auv3_activate() from thread {:?}", std::thread::current().id());

            let processor_config = ProcessorConfig {
                sample_rate,
                min_block_size: 0,
                max_block_size: max_block_size as _,
                process_mode: ::plinth_plugin::ProcessMode::Realtime, // TODO
            };

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let plugin = wrapper.plugin.lock().unwrap();

                wrapper.sample_rate.store(sample_rate, ::std::sync::atomic::Ordering::Release);
                wrapper.processor = Some(plugin.create_processor(&processor_config));
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_deactivate(wrapper: *mut ::std::ffi::c_void) {
            log::trace!("plinth_auv3_deactivate() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                wrapper.processor = None;
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_tail_length(wrapper: *mut ::std::ffi::c_void) -> f64 {
            log::trace!("plinth_auv3_tail_length() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                wrapper.tail_length_seconds.load(::std::sync::atomic::Ordering::Acquire)
            })
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_has_aux_bus() -> bool {
            log::trace!("plinth_auv3_has_aux_bus() from thread {:?}", std::thread::current().id());
            <$plugin>::HAS_AUX_INPUT
        }

        #[no_mangle]
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

            assert_eq!(channels, 2);

            let input = if input.is_null() || plinth_core::util::ptr::any_null(input, channels as usize) {
                None
            } else {
                Some(::plinth_core::signals::ptr_signal::PtrSignal::from_pointers(channels as usize, frames as usize, input))
            };

            let mut output = if output.is_null() || plinth_core::util::ptr::any_null_mut(output, channels as usize) {
                None
            } else {
                Some(::plinth_core::signals::ptr_signal::PtrSignalMut::from_pointers(channels as usize, frames as usize, output))
            };

            let aux = if aux.is_null() || plinth_core::util::ptr::any_null(aux, channels as usize) {
                None
            } else {
                Some(::plinth_core::signals::ptr_signal::PtrSignal::from_pointers(channels as usize, frames as usize, aux))
            };

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                use ::plinth_plugin::Processor;

                let processor = wrapper.processor.as_mut().unwrap();

                let event_count = wrapper.events_to_processor_receiver.slots();
                if event_count > 0 {
                    processor.process_events(wrapper.events_to_processor_receiver.read_chunk(event_count).unwrap().into_iter());
                }

                let transport = ::plinth_plugin::Transport::new(playing, tempo, position_samples);

                let state = if let (Some(input), Some(mut output)) = (input.as_ref(), output.as_mut()) {
                    for ptr in input.pointers().iter() {
                        assert!(!ptr.is_null());
                    }
                    for ptr in output.pointers().iter() {
                        assert!(!ptr.is_null());
                    }

                    // If processing out-of-place, copy input to output
                    if ::std::iter::zip(input.pointers().iter(), output.pointers().iter())
                        .any(|(&input_ptr, &output_ptr)| input_ptr != &*output_ptr)
                    {
                        use ::plinth_core::signals::signal::SignalMut;
                        output.copy_from_signal(input);
                    }
                    
                    let state = processor.process(
                        output,
                        aux.as_ref(),
                        Some(transport),
                        &mut ::plinth_plugin::auv3::EventIterator::new(first_event, &wrapper.parameter_ids));

                        let tail_length_samples = match state {
                            ::plinth_plugin::ProcessState::Error => {
                                log::error!("Processing error");
                                0
                            },
        
                            ::plinth_plugin::ProcessState::Normal => 0,
                            ::plinth_plugin::ProcessState::Tail(tail) => tail,
                            ::plinth_plugin::ProcessState::KeepAlive => usize::MAX,
                        };
        
                        let sample_rate = wrapper.sample_rate.load(::std::sync::atomic::Ordering::Acquire);
                        let tail_length_seconds = tail_length_samples as f64 / sample_rate;
                        wrapper.tail_length_seconds.store(tail_length_seconds, ::std::sync::atomic::Ordering::Release);
                } else {
                    processor.process_events(&mut ::plinth_plugin::auv3::EventIterator::new(first_event, &wrapper.parameter_ids));
                };
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_parameter_count(wrapper: *mut ::std::ffi::c_void) -> u64 {
            log::trace!("plinth_auv3_parameter_count() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let plugin = wrapper.plugin.lock().unwrap();
                plugin.with_parameters(|parameters| {
                    parameters.ids().len() as _
                })
            })
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_parameter_info(
            wrapper: *mut ::std::ffi::c_void,
            index: usize,
            info: *mut ::plinth_plugin::auv3::ParameterInfo
        ) {
            log::trace!("plinth_auv3_parameter_info() from thread {:?}", std::thread::current().id());

            let info = unsafe { &mut *info };

            assert!(!info.name.is_null());
            assert!(!info.identifier.is_null());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let plugin = wrapper.plugin.lock().unwrap();
                plugin.with_parameters(|parameters| {
                    let Some(&id) = parameters.ids().get(index as usize) else {
                        return;
                    };
                    
                    let parameter = parameters.get(id).unwrap();

                    info.address = id as _;
                    info.steps = parameter.info().steps() as _;

                    let name_slice = std::slice::from_raw_parts_mut(info.name as _, ::plinth_plugin::auv3::PLINTH_AUV3_MAX_STRING_LENGTH);
                    let identifier_slice = std::slice::from_raw_parts_mut(info.identifier as _, ::plinth_plugin::auv3::PLINTH_AUV3_MAX_STRING_LENGTH);
                    ::plinth_plugin::string::copy_str_to_char8(parameter.info().name(), name_slice);
                    ::plinth_plugin::string::copy_str_to_char8(&format!("ID{id}"), identifier_slice);

                    info.parentGroupIndex = wrapper.parameter_groups
                        .iter()
                        .position(|group| group.path == parameter.info().path())
                        .map(|position| position as i64)
                        .unwrap_or(-1);
                })
            })
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_group_count(wrapper: *mut ::std::ffi::c_void) -> u64 {
            log::trace!("plinth_auv3_group_count() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                wrapper.parameter_groups.len() as u64
            })
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_group_info(
            wrapper: *mut ::std::ffi::c_void,
            index: usize,
            info: *mut ::plinth_plugin::auv3::ParameterGroupInfo
        ) {
            log::trace!("plinth_auv3_group_info() from thread {:?}", std::thread::current().id());

            let info = unsafe { &mut *info };

            assert!(!info.name.is_null());
            assert!(!info.identifier.is_null());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let group = &wrapper.parameter_groups[index];

                let name_slice = std::slice::from_raw_parts_mut(info.name as _, ::plinth_plugin::auv3::PLINTH_AUV3_MAX_STRING_LENGTH);
                let identifier_slice = std::slice::from_raw_parts_mut(info.identifier as _, ::plinth_plugin::auv3::PLINTH_AUV3_MAX_STRING_LENGTH);
                ::plinth_plugin::string::copy_str_to_char8(&group.name, name_slice);
                ::plinth_plugin::string::copy_str_to_char8(&format!("Group{}", index), identifier_slice);

                info.parentGroupIndex = group.parent.as_ref()
                    .map(|parent| wrapper.parameter_groups.iter().position(|group| group == parent).unwrap() as i64)
                    .unwrap_or(-1);
            });
        }
    
        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_get_parameter_value(wrapper: *mut ::std::ffi::c_void, address: u64) -> f32 {
            log::trace!("plinth_auv3_get_parameter_value() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let plugin = wrapper.plugin.lock().unwrap();
                plugin.with_parameters(|parameters| {
                    if let Some(parameter) = parameters.get(address as ::plinth_plugin::parameters::ParameterId) {
                        (parameter.normalized_value() * ::plinth_plugin::auv3::parameter_multiplier(parameter)) as _
                    } else {
                        0.0
                    }
                })
            })
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_set_parameter_value(wrapper: *mut ::std::ffi::c_void, address: u64, value: f32) {
            log::trace!("plinth_auv3_set_parameter_value() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let mut plugin = wrapper.plugin.lock().unwrap();
                if let Some(event) = plugin.with_parameters(|parameters| {
                    if let Some(parameter) = parameters.get(address as ::plinth_plugin::parameters::ParameterId) {
                        Some(plinth_plugin::Event::ParameterValue {
                            sample_offset: 0,
                            id: address as _,
                            value: (value as f64 / ::plinth_plugin::auv3::parameter_multiplier(parameter)),
                        })
                    } else {
                        None
                    }
                }) {
                    if !wrapper.sending_parameter_change_from_editor.load(::std::sync::atomic::Ordering::Acquire) {
                        plugin.process_event(&event);
                    }

                    wrapper.events_to_processor_sender.push(event).unwrap();
                }
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_parameter_normalized_to_string(
            wrapper: *mut ::std::ffi::c_void,
            address: u64,
            value: f32,
            string: *mut ::std::ffi::c_char
        ) {
            log::trace!("plinth_auv3_parameter_normalized_to_string() from thread {:?}", std::thread::current().id());

            assert!(!string.is_null());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let plugin = wrapper.plugin.lock().unwrap();
                plugin.with_parameters(|parameters| {
                    if let Some(parameter) = parameters.get(address as ::plinth_plugin::parameters::ParameterId) {
                        let value = value as f64 / ::plinth_plugin::auv3::parameter_multiplier(parameter);
                        let value_string = parameter.normalized_to_string(value);
                        let string_slice = std::slice::from_raw_parts_mut(string, ::plinth_plugin::auv3::PLINTH_AUV3_MAX_STRING_LENGTH);

                        ::plinth_plugin::string::copy_str_to_char8(&value_string, string_slice)
                    }
                });
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_load_state(
            wrapper: *mut ::std::ffi::c_void,
            context: *mut ::std::ffi::c_void,
            read: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *mut u8, usize) -> usize,
        ) {            
            log::trace!("plinth_auv3_load_state() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let mut plugin = wrapper.plugin.lock().unwrap();
                let mut reader = ::plinth_plugin::auv3::Auv3Reader::new(context, read);
                plugin.load_state(&mut reader).unwrap();

                // Send events to processor
                plugin.with_parameters(|parameters| {
                    for &id in parameters.ids().iter() {
                        let event = ::plinth_plugin::Event::ParameterValue {
                            sample_offset: 0,
                            id,
                            value: parameters.get(id).unwrap().normalized_value(),
                        };

                        wrapper.events_to_processor_sender.push(event).unwrap();    
                    }
                });
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_save_state(
            wrapper: *mut ::std::ffi::c_void,
            context: *mut ::std::ffi::c_void,
            write: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, *const u8, usize) -> usize,
        ) {            
            log::trace!("plinth_auv3_save_state() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                let plugin = wrapper.plugin.lock().unwrap();
                let mut writer = ::plinth_plugin::auv3::Auv3Writer::new(context, write);
                plugin.save_state(&mut writer).unwrap();
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_preferred_editor_size(width: *mut f64, height: *mut f64) {
            log::trace!("plinth_auv3_preferred_editor_size() from thread {:?}", std::thread::current().id());

            use ::plinth_plugin::Editor;

            *width = <$plugin as Plugin>::Editor::SIZE.0;
            *height = <$plugin as Plugin>::Editor::SIZE.1;
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_editor_set_scale(wrapper: *mut ::std::ffi::c_void, scale: f64) {
            log::trace!("plinth_auv3_editor_set_scale() from thread {:?}", std::thread::current().id());

            use ::plinth_plugin::Editor;

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                if let Some(editor) = wrapper.editor.as_mut() {
                    editor.set_scale(scale);
                }
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_editor_open(
            wrapper: *mut ::std::ffi::c_void,
            parent: *mut ::std::ffi::c_void,
            context: *mut ::std::ffi::c_void,
            start_parameter_change: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, ::plinth_plugin::ParameterId),
            change_parameter_value: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, ::plinth_plugin::ParameterId, f32),
            end_parameter_change: unsafe extern "C-unwind" fn(*mut ::std::ffi::c_void, ::plinth_plugin::ParameterId),
            scale: f64,
        ) {
            log::trace!("plinth_auv3_editor_open() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                assert!(wrapper.editor.is_none());

                let mut plugin = wrapper.plugin.lock().unwrap();

                let raw_window_handle = ::plinth_plugin::raw_window_handle::AppKitWindowHandle::new(
                    std::ptr::NonNull::new(parent as _).unwrap()
                );
                let parent_window_handle = RawWindowHandle::AppKit(raw_window_handle);

                let host = ::plinth_plugin::auv3::Auv3Host::new(
                    context,
                    start_parameter_change,
                    change_parameter_value,
                    end_parameter_change,
                    wrapper.sending_parameter_change_from_editor.clone(),
                );

                wrapper.editor = Some(plugin.open_editor(parent_window_handle, Rc::new(host), scale));
            });
        }

        #[no_mangle]
        unsafe extern "C-unwind" fn plinth_auv3_editor_close(wrapper: *mut ::std::ffi::c_void) {
            log::trace!("plinth_auv3_editor_close() from thread {:?}", std::thread::current().id());

            ::plinth_plugin::auv3::Auv3Wrapper::<$plugin>::with_wrapper(wrapper, |wrapper| {
                assert!(wrapper.editor.is_some());
                wrapper.editor = None;
            });
        }
    }
}
