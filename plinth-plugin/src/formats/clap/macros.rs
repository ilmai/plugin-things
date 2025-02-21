#[macro_export]
macro_rules! export_clap {
    ($plugin:ty) => {
        static FACTORY: ::std::sync::Mutex<Option<::plinth_plugin::clap::Factory::<$plugin>>> = ::std::sync::Mutex::new(None);

        unsafe extern "C" fn init(_plugin_path: *const ::std::ffi::c_char) -> bool {
            let mut factory = FACTORY.lock().unwrap();
        
            match factory.as_mut() {
                Some(factory) => {
                    factory.add_ref();
                },
        
                None => {
                    *factory = Some(::plinth_plugin::clap::Factory::<$plugin>::new());
                }
            }
        
            true
        }
        
        unsafe extern "C" fn deinit() {
            let mut maybe_factory = FACTORY.lock().unwrap();
        
            match maybe_factory.as_mut() {
                Some(factory) => {
                    if factory.remove_ref() == 0 {
                        *maybe_factory = None;
                    }
                },
        
                None => {
                    // TODO: Add warning about extra deinit call
                    panic!();
                },
            }
        }
        
        unsafe extern "C" fn get_factory(factory_id: *const ::std::ffi::c_char) -> *const ::std::ffi::c_void {
            if !::plinth_plugin::clap::Factory::<$plugin>::is_valid_factory_id(factory_id) {
                return ::std::ptr::null();
            }
        
            let factory = FACTORY.lock().unwrap();
            let Some(factory) = factory.as_ref() else {
                return ::std::ptr::null();
            };
        
            factory.as_raw() as _
        }
                
        #[unsafe(no_mangle)]
        #[allow(non_snake_case)]
        #[allow(non_upper_case_globals)]
        static clap_entry: ::plinth_plugin::clap::EntryPoint = ::plinth_plugin::clap::EntryPoint::new(init, deinit, get_factory);
    };
}
