use icrate::Foundation::NSError;
use objc2::{msg_send_id, rc::Id, ClassType};

use crate::{audio_toolbox::{AUAudioUnit, AudioComponentDescription}, extension::DemoAudioUnit};

#[no_mangle]
pub extern fn create_audio_unit(description: AudioComponentDescription) -> Id<AUAudioUnit> {
    let audio_unit: Result<Id<DemoAudioUnit>, Id<NSError>> = unsafe {
        msg_send_id![
            DemoAudioUnit::alloc(),
            initWithComponentDescription:description,
            error:_,
        ]
    };

    let audio_unit = audio_unit.unwrap();

    Id::<DemoAudioUnit>::into_super(audio_unit)
}
