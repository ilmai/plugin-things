use objc2::{declare_class, mutability, ClassType};

use crate::audio_toolbox::AUAudioUnit;

declare_class!(
    pub struct DemoAudioUnit {
    }

    unsafe impl ClassType for DemoAudioUnit {
        type Super = AUAudioUnit;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "DemoExtension";
    }
);
