use objc2::{extern_class, mutability, ClassType, runtime::NSObject};

extern_class!(
    pub struct AUAudioUnit;

    unsafe impl ClassType for AUAudioUnit {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
    }
);
