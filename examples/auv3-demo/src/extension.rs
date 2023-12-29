use std::sync::RwLock;

use icrate::Foundation::{NSError, NSArray};
use objc2::{declare_class, mutability, ClassType, rc::{Id, Allocated}, DeclaredClass, msg_send_id};

use crate::audio_toolbox::{AUAudioUnit, AUAudioUnitBusArray, AudioComponentDescription, AUAudioUnitBusType, AVAudioFormat, AUAudioUnitBus};

pub struct Ivars {
    input_busses: RwLock<Option<Id<AUAudioUnitBusArray>>>,
    output_busses: RwLock<Option<Id<AUAudioUnitBusArray>>>,
}

declare_class!(
    pub struct DemoAudioUnit;

    unsafe impl ClassType for DemoAudioUnit {
        type Super = AUAudioUnit;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "DemoAudioUnit";
    }

    impl DeclaredClass for DemoAudioUnit {
        type Ivars = Ivars;
    }

    unsafe impl DemoAudioUnit {
        #[method_id(inputBusses)]
        fn __get_input_busses(&self) -> Option<Id<AUAudioUnitBusArray>> {
            self.ivars().input_busses.read().unwrap().clone()
        }

        #[method_id(outputBusses)]
        fn __get_output_busses(&self) -> Option<Id<AUAudioUnitBusArray>> {
            self.ivars().output_busses.read().unwrap().clone()
        }

        #[method_id(initWithComponentDescription:error:)]
        fn __init_with_component_description_error(this: Allocated<Self>, component_description: AudioComponentDescription, error: &mut &mut NSError) -> Option<Id<Self>> {
            let this = this.set_ivars(Ivars {
                input_busses: Default::default(),
                output_busses: Default::default(),
            });

            let this: Option<Id<Self>> = unsafe { msg_send_id![super(this), initWithComponentDescription:component_description, error:error] };

            if let Some(this) = this.as_ref() {
                unsafe {
                    let format: Id<AVAudioFormat> = msg_send_id![
                        AVAudioFormat::alloc(),
                        initStandardFormatWithSampleRate:48000.0_f64,
                        channels:2_u32,
                    ];

                    let output_bus: Result<Id<AUAudioUnitBus>, Id<NSError>> = msg_send_id![
                        AUAudioUnitBus::alloc(),
                        initWithFormat:Id::as_ptr(&format),
                        error: _
                    ];
                    let output_bus = output_bus.unwrap();

                    let output_bus_array = NSArray::from_vec(vec![output_bus]);
                    let output_busses: Id<AUAudioUnitBusArray> = msg_send_id![
                        AUAudioUnitBusArray::alloc(),
                        initWithAudioUnit:Id::as_ptr(this),
                        busType:AUAudioUnitBusType::Output as isize,
                        busses:Id::as_ptr(&output_bus_array),
                    ];

                    *this.ivars().output_busses.write().unwrap() = Some(output_busses);
                }
            }

            this
        }
    }
);
