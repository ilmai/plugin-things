use std::ffi::c_uint;

use objc2::{extern_class, mutability, ClassType, runtime::NSObject, Encode, Encoding};

#[repr(isize)]
pub enum AUAudioUnitBusType {
    Input = 1,
    Output = 2,
}

extern_class!(
    pub struct AUAudioUnit;

    unsafe impl ClassType for AUAudioUnit {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    pub struct AUAudioUnitBus;

    unsafe impl ClassType for AUAudioUnitBus {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    pub struct AUAudioUnitBusArray;

    unsafe impl ClassType for AUAudioUnitBusArray {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

extern_class!(
    pub struct AVAudioFormat;

    unsafe impl ClassType for AVAudioFormat {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);

#[repr(C)]
#[derive(Debug)]
pub struct AudioComponentDescription {
    pub component_type: c_uint,
    pub component_sub_type: c_uint,
    pub component_manufacturer: c_uint,
    pub component_flags: u32,
    pub component_flags_mask: u32,
}

unsafe impl Encode for AudioComponentDescription {
    const ENCODING: Encoding = Encoding::Struct(
        "AudioComponentDescription",
        &[
            Encoding::UInt,
            Encoding::UInt,
            Encoding::UInt,
            Encoding::UInt,
            Encoding::UInt,
        ]
    );
}
