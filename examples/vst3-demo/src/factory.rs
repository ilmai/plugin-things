use std::{cell::OnceCell, mem::transmute, ffi::c_void};

use vst3::{Steinberg::{IPluginFactory3, IPluginFactory, IPluginFactory2, IPluginFactoryTrait, IPluginFactory2Trait, IPluginFactory3Trait, PFactoryInfo, tresult, int32, PClassInfo, FIDString, PClassInfo2, PClassInfoW, FUnknown, PFactoryInfo_::FactoryFlags_::kUnicode, kResultOk, kInvalidArgument, PClassInfo_::ClassCardinality_::kManyInstances}, ComWrapper};
use widestring::utf16str;

use crate::processor::Processor;

thread_local! {
    static FACTORY: OnceCell<ComWrapper<Factory>> = OnceCell::new();
}

#[no_mangle]
pub extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    FACTORY.with(|factory| {
        let factory = factory.get_or_init(|| ComWrapper::new(Factory));
        factory.to_com_ptr::<IPluginFactory>().unwrap().into_raw()
    })
}

pub struct Factory;

impl vst3::Class for Factory {
    type Interfaces = (IPluginFactory, IPluginFactory2, IPluginFactory3);
}

#[allow(non_snake_case)]
impl IPluginFactoryTrait for Factory {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        let info = &mut *info;
        info.vendor[..11].copy_from_slice(unsafe { transmute(b"Viiri Audio".as_slice()) });
        //info.email
        //info.url
        info.flags = kUnicode;

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
        info.cid[..15].copy_from_slice(unsafe { transmute(b"viiri-audio.com".as_slice()) });
        info.cardinality = kManyInstances;
        //info.category
        info.name[..4].copy_from_slice(unsafe { transmute(b"Demo".as_slice()) });
        //info.category
        //info.subCategories

        kResultOk
    }

    unsafe fn createInstance(&self, _cid: FIDString, _iid: FIDString, obj: *mut*mut c_void) -> tresult {
        // TODO: Check cid
        let instance = Box::new(Processor);
        *obj = Box::into_raw(instance) as *mut c_void;

        kResultOk
    }
}

#[allow(non_snake_case)]
impl IPluginFactory2Trait for Factory {
    unsafe fn getClassInfo2(&self, index: int32, info: *mut PClassInfo2) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let info = &mut *info;
        info.cid[..15].copy_from_slice(unsafe { transmute(b"viiri-audio.com".as_slice()) });
        info.cardinality = kManyInstances;
        //info.category
        info.name[..4].copy_from_slice(unsafe { transmute(b"Demo".as_slice()) });
        //info.category
        //info.subCategories
        info.version[..3].copy_from_slice(unsafe { transmute(b"0.0".as_slice()) });
        info.sdkVersion[..7].copy_from_slice(unsafe { transmute(b"VST 3.7".as_slice()) });

        kResultOk
    }
}

#[allow(non_snake_case)]
impl IPluginFactory3Trait for Factory {
    unsafe fn getClassInfoUnicode(&self, index: int32, info: *mut PClassInfoW) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let info = &mut *info;
        info.cid[..15].copy_from_slice(unsafe { transmute(b"viiri-audio.com".as_slice()) });
        info.cardinality = kManyInstances;
        //info.category
        info.name[..4].copy_from_slice(unsafe { transmute(b"Demo".as_slice()) });
        //info.category
        //info.subCategories
        info.version[..3].copy_from_slice(unsafe { transmute(utf16str!("0.0").as_slice()) });
        info.sdkVersion[..7].copy_from_slice(unsafe { transmute(utf16str!("VST 3.7").as_slice()) });

        kResultOk
    }

    unsafe fn setHostContext(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }
}
