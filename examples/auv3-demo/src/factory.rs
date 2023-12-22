use objc2::{msg_send_id, rc::Id, ClassType};

use crate::{audio_toolbox::AUAudioUnit, extension::DemoAudioUnit};

#[no_mangle]
pub extern fn create_audio_unit() -> Id<AUAudioUnit> {
    let audio_unit: Id<DemoAudioUnit> = unsafe { msg_send_id![DemoAudioUnit::alloc(), init] };
    Id::<DemoAudioUnit>::into_super(audio_unit)
}
