use vst3::Steinberg::{Vst::{IAudioProcessor, IComponent, IAudioProcessorTrait, IComponentTrait, SpeakerArrangement, BusDirection, ProcessSetup, ProcessData, IoMode, MediaType, BusInfo, RoutingInfo}, int32, tresult, uint32, TBool, IPluginBaseTrait, TUID, IBStream, FUnknown};

pub struct Processor;

impl vst3::Class for Processor {
    type Interfaces = (IAudioProcessor, IComponent);
}

#[allow(non_snake_case)]
impl IAudioProcessorTrait for Processor {
    unsafe fn setBusArrangements(&self, inputs: *mut SpeakerArrangement, numIns: int32, outputs: *mut SpeakerArrangement, numOuts: int32) -> tresult {
        todo!()
    }

    unsafe fn getBusArrangement(&self, dir: BusDirection, index: int32, arr: *mut SpeakerArrangement) -> tresult {
        todo!()
    }

    unsafe fn canProcessSampleSize(&self, symbolicSampleSize: int32) -> tresult {
        todo!()
    }

    unsafe fn getLatencySamples(&self) -> uint32 {
        todo!()
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        todo!()
    }

    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        todo!()
    }

    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        todo!()
    }

    unsafe fn getTailSamples(&self) -> uint32 {
        todo!()
    }
}

#[allow(non_snake_case)]
impl IComponentTrait for Processor {
    unsafe fn getControllerClassId(&self, classId: *mut TUID) -> tresult {
        todo!()
    }

    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        todo!()
    }

    unsafe fn getBusCount(&self, r#type: MediaType, dir: BusDirection) -> int32 {
        todo!()
    }

    unsafe fn getBusInfo(&self, r#type: MediaType, dir: BusDirection, index: int32, bus: *mut BusInfo) -> tresult {
        todo!()
    }

    unsafe fn getRoutingInfo(&self, inInfo: *mut RoutingInfo, outInfo: *mut RoutingInfo) -> tresult {
        todo!()
    }

    unsafe fn activateBus(&self, r#type: MediaType, dir: BusDirection, index: int32, state: TBool) -> tresult {
        todo!()
    }

    unsafe fn setActive(&self, state: TBool) -> tresult {
        todo!()
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        todo!()
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        todo!()
    }
}

impl IPluginBaseTrait for Processor {
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        todo!()
    }

    unsafe fn terminate(&self,) -> tresult {
        todo!()
    }
}
