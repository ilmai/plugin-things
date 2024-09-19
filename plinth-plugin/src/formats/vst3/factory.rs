use std::{ffi::{c_void, CStr}, marker::PhantomData};

use vst3::{ComWrapper, Steinberg::{int32, kInvalidArgument, kResultOk, tresult, FIDString, FUnknown, IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactory3, IPluginFactory3Trait, IPluginFactoryTrait, PClassInfo, PClassInfo2, PClassInfoW, PClassInfo_::ClassCardinality_::kManyInstances, PFactoryInfo, PFactoryInfo_, Vst::{IComponent, SDKVersionString}, TUID}};

use crate::string::{copy_str_to_char16, copy_str_to_char8, copy_u128_to_char8};

use super::{plugin::Vst3Plugin, component::PluginComponent};

pub struct Factory<P: Vst3Plugin> {
    _phantom_plugin: PhantomData<P>,
}

impl<P: Vst3Plugin + 'static> Factory<P> {
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
        let info = &mut *info;
        info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as _;

        copy_str_to_char8(P::VENDOR, &mut info.vendor);

        if let Some(url) = P::URL {
            copy_str_to_char8(url, &mut info.url);
        } else {
            info.url.fill(0);
        }

        if let Some(email) = P::EMAIL {
            copy_str_to_char8(email, &mut info.email);
        } else {
            info.email.fill(0);
        }

        kResultOk
    }

    unsafe fn countClasses(&self) -> int32 {
        1
    }

    unsafe fn getClassInfo(&self, index: int32, info: *mut PClassInfo) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let info = &mut *info;
        info.cardinality = kManyInstances as _;

        copy_u128_to_char8(&P::CLASS_ID, &mut info.cid);
        copy_str_to_char8(&P::NAME, &mut info.name);

        copy_str_to_char8("Audio Module Class", &mut info.category);

        kResultOk
    }

    unsafe fn createInstance(&self, cid: FIDString, iid: FIDString, obj: *mut *mut c_void) -> tresult {
        if cid.is_null() {
            return kInvalidArgument;
        }

        let bytes = std::slice::from_raw_parts(cid, 16);
        let bytes_array: [u8; 16] = std::array::from_fn(|i| bytes[i] as u8);
        let cid = u128::from_be_bytes(bytes_array);

        if cid == P::CLASS_ID {
            let instance = ComWrapper::new(PluginComponent::<P>::new());
            let unknown = instance.as_com_ref::<FUnknown>().unwrap();
            let ptr = unknown.as_ptr();

            ((*(*ptr).vtbl).queryInterface)(ptr, iid as *const TUID, obj)
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

        let info = &mut *info;
        info.cardinality = kManyInstances as _;

        copy_u128_to_char8(&P::CLASS_ID, &mut info.cid);
        copy_str_to_char8(&P::NAME, &mut info.name);
        copy_str_to_char8(&P::VERSION, &mut info.version);

        copy_str_to_char8("Audio Module Class", &mut info.category);
        copy_str_to_char8(CStr::from_ptr(SDKVersionString).to_str().unwrap(), &mut info.sdkVersion);

        kResultOk
    }
}

#[allow(non_snake_case)]
impl<P: Vst3Plugin + 'static> IPluginFactory3Trait for Factory<P> {
    unsafe fn getClassInfoUnicode(&self, index: int32, info: *mut PClassInfoW) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let info = &mut *info;
        info.cardinality = kManyInstances as _;

        copy_u128_to_char8(&P::CLASS_ID, &mut info.cid);
        copy_str_to_char16(&P::NAME, &mut info.name);
        copy_str_to_char16(&P::VERSION, &mut info.version);

        copy_str_to_char8("Audio Module Class", &mut info.category);
        copy_str_to_char16(CStr::from_ptr(SDKVersionString).to_str().unwrap(), &mut info.sdkVersion);

        let subcategory_string = P::SUBCATEGORIES
            .iter()
            .map(|subcategory| subcategory.to_str())
            .collect::<Vec<_>>()
            .join("|");
        copy_str_to_char8(&subcategory_string, &mut info.subCategories);

        kResultOk
    }

    unsafe fn setHostContext(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }
}
