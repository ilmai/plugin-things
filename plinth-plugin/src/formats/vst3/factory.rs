use std::{ffi::{c_void, CStr}, marker::PhantomData};

use vst3::{ComWrapper, Steinberg::{int32, kInvalidArgument, kResultOk, tresult, FIDString, FUnknown, IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3, IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo2, PClassInfoW, PClassInfo_::ClassCardinality_::kManyInstances, PFactoryInfo, PFactoryInfo_, Vst::SDKVersionString, TUID}};

use crate::string::{copy_str_to_char16, copy_str_to_char8, copy_u128_to_char8};

use super::{plugin::Vst3Plugin, component::PluginComponent};

pub struct Factory<P: Vst3Plugin> {
    _phantom_plugin: PhantomData<P>,
}

impl<P: Vst3Plugin + 'static> Factory<P> {
    // This is a bit special
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> *mut IPluginFactory {
        let factory = Self {
            _phantom_plugin: PhantomData,
        };

        ComWrapper::new(factory)
            .to_com_ptr::<IPluginFactory>()
            .unwrap()
            .into_raw() as _
    }    
}

impl<P: Vst3Plugin> vst3::Class for Factory<P> {
    type Interfaces = (IPluginFactory, IPluginFactory2, IPluginFactory3);
}

#[allow(non_snake_case)]
impl<P: Vst3Plugin + 'static> IPluginFactoryTrait for Factory<P> {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        let mut local_info: PFactoryInfo = unsafe { std::mem::zeroed() };

        local_info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as _;

        copy_str_to_char8(P::VENDOR, &mut local_info.vendor);

        if let Some(url) = P::URL {
            copy_str_to_char8(url, &mut local_info.url);
        } else {
            local_info.url.fill(0);
        }

        if let Some(email) = P::EMAIL {
            copy_str_to_char8(email, &mut local_info.email);
        } else {
            local_info.email.fill(0);
        }

        // We have to do a workaround like this for FL Studio which is giving us unaligned addresses
        unsafe { std::ptr::write_unaligned(info, local_info) };

        kResultOk
    }

    unsafe fn countClasses(&self) -> int32 {
        1
    }

    unsafe fn getClassInfo(&self, index: int32, info: *mut PClassInfo) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let mut local_info: PClassInfo = unsafe { std::mem::zeroed() };
        local_info.cardinality = kManyInstances as _;

        copy_u128_to_char8(&P::CLASS_ID, &mut local_info.cid);
        copy_str_to_char8(P::NAME, &mut local_info.name);

        copy_str_to_char8("Audio Module Class", &mut local_info.category);

        // We have to do a workaround like this for FL Studio which is giving us unaligned addresses
        unsafe { std::ptr::write_unaligned(info, local_info) };

        kResultOk
    }

    unsafe fn createInstance(&self, cid: FIDString, iid: FIDString, obj: *mut *mut c_void) -> tresult {
        if cid.is_null() {
            return kInvalidArgument;
        }

        let bytes = unsafe { std::slice::from_raw_parts(cid, 16) };
        let bytes_array: [u8; 16] = std::array::from_fn(|i| bytes[i] as u8);
        let cid = u128::from_be_bytes(bytes_array);

        if cid == P::CLASS_ID {
            let instance = ComWrapper::new(PluginComponent::<P>::new());
            let unknown = instance.as_com_ref::<FUnknown>().unwrap();
            let ptr = unknown.as_ptr();

            unsafe { ((*(*ptr).vtbl).queryInterface)(ptr, iid as *const TUID, obj) }
        } else {
            kInvalidArgument
        }
    }
}

#[allow(non_snake_case)]
impl<P: Vst3Plugin + 'static> IPluginFactory2Trait for Factory<P> {
    unsafe fn getClassInfo2(&self, index: int32, info: *mut PClassInfo2) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let mut local_info: PClassInfo2 = unsafe { std::mem::zeroed() };
        local_info.cardinality = kManyInstances as _;

        copy_u128_to_char8(&P::CLASS_ID, &mut local_info.cid);
        copy_str_to_char8(P::NAME, &mut local_info.name);
        copy_str_to_char8(P::VERSION, &mut local_info.version);

        copy_str_to_char8("Audio Module Class", &mut local_info.category);
        copy_str_to_char8(unsafe { CStr::from_ptr(SDKVersionString).to_str().unwrap() }, &mut local_info.sdkVersion);

        // We have to do a workaround like this for FL Studio which is giving us unaligned addresses
        unsafe { std::ptr::write_unaligned(info, local_info) };

        kResultOk
    }
}

#[allow(non_snake_case)]
impl<P: Vst3Plugin + 'static> IPluginFactory3Trait for Factory<P> {
    unsafe fn getClassInfoUnicode(&self, index: int32, info: *mut PClassInfoW) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let mut local_info: PClassInfoW = unsafe { std::mem::zeroed() };
        local_info.cardinality = kManyInstances as _;

        copy_u128_to_char8(&P::CLASS_ID, &mut local_info.cid);
        copy_str_to_char16(P::NAME, &mut local_info.name);
        copy_str_to_char16(P::VERSION, &mut local_info.version);

        copy_str_to_char8("Audio Module Class", &mut local_info.category);
        copy_str_to_char16(unsafe { CStr::from_ptr(SDKVersionString).to_str().unwrap() }, &mut local_info.sdkVersion);

        let subcategory_string = P::SUBCATEGORIES
            .iter()
            .map(|subcategory| subcategory.to_str())
            .collect::<Vec<_>>()
            .join("|");
        copy_str_to_char8(&subcategory_string, &mut local_info.subCategories);

        // We have to do a workaround like this for FL Studio which is giving us unaligned addresses
        unsafe { std::ptr::write_unaligned(info, local_info) };

        kResultOk
    }

    unsafe fn setHostContext(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }
}
