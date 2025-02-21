#[macro_export]
macro_rules! export_vst3 {
    ($plugin:ty) => {
        #[unsafe(no_mangle)]
        pub extern "system" fn GetPluginFactory() -> *mut ::std::ffi::c_void {
            ::plinth_plugin::vst3::Factory::<$plugin>::new() as _
        }
        
        #[cfg(target_os="windows")]
        #[unsafe(no_mangle)]
        pub extern "system" fn InitDll() -> bool {
            true
        }
        
        #[cfg(target_os="windows")]
        #[unsafe(no_mangle)]
        pub extern "system" fn ExitDll() -> bool {
            true
        }
        
        #[cfg(target_os="macos")]
        #[unsafe(no_mangle)]
        pub extern "system" fn bundleEntry(_bundle: *mut std::ffi::c_void) -> bool {
            true
        }
        
        #[cfg(target_os="macos")]
        #[unsafe(no_mangle)]
        pub extern "system" fn bundleExit() -> bool {
            true
        }
        
        #[cfg(target_os="linux")]
        #[unsafe(no_mangle)]
        pub extern "system" fn ModuleEntry(shared_library_handle: *mut std::ffi::c_void) -> bool {
            true
        }
        
        #[cfg(target_os="linux")]
        #[unsafe(no_mangle)]
        pub extern "system" fn ModuleExit() -> bool {
            true
        }                
    };
}
