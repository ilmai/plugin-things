use vst3::Steinberg::{Vst::{IAudioProcessor, IComponent, IAudioProcessorTrait, IComponentTrait, SpeakerArrangement, BusDirection, ProcessSetup, ProcessData, IoMode, MediaType, BusInfo, RoutingInfo, SpeakerArr, SymbolicSampleSizes_::kSample32, MediaTypes_, BusTypes_, BusInfo_::BusFlags_}, int32, tresult, uint32, TBool, IPluginBaseTrait, TUID, IBStream, FUnknown, kResultOk, kResultFalse, kInvalidArgument};

pub struct Processor;

impl vst3::Class for Processor {
    type Interfaces = (IAudioProcessor, IComponent);
}

#[allow(non_snake_case)]
impl IAudioProcessorTrait for Processor {
    unsafe fn setBusArrangements(&self, _inputs: *mut SpeakerArrangement, _numIns: int32, _outputs: *mut SpeakerArrangement, _numOuts: int32) -> tresult {
        kResultFalse
    }

    unsafe fn getBusArrangement(&self, _dir: BusDirection, index: int32, arr: *mut SpeakerArrangement) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        unsafe { *arr = SpeakerArr::kStereo; }
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, symbolicSampleSize: int32) -> tresult {
        if symbolicSampleSize == kSample32 as _ {
            kResultOk
        } else {
            kResultFalse
        }
    }

    unsafe fn getLatencySamples(&self) -> uint32 {
        0
    }

    unsafe fn setupProcessing(&self, _setup: *mut ProcessSetup) -> tresult {
        kResultOk
    }

    unsafe fn setProcessing(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn process(&self, _data: *mut ProcessData) -> tresult {
        kResultOk
    }

    unsafe fn getTailSamples(&self) -> uint32 {
        0
    }
}

#[allow(non_snake_case)]
impl IComponentTrait for Processor {
    unsafe fn getControllerClassId(&self, _classId: *mut TUID) -> tresult {
        kResultFalse
    }

    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    unsafe fn getBusCount(&self, _type: MediaType, _dir: BusDirection) -> int32 {
        1
    }

    unsafe fn getBusInfo(&self, _type: MediaType, dir: BusDirection, index: int32, bus: *mut BusInfo) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }

        let bus = unsafe { &mut *bus };
        bus.mediaType = MediaTypes_::kAudio as _;
        bus.direction = dir;
        bus.channelCount = 2;
        bus.busType = BusTypes_::kMain as _;
        bus.flags = BusFlags_::kDefaultActive as _;

        kResultOk
    }

    unsafe fn getRoutingInfo(&self, _inInfo: *mut RoutingInfo, _outInfo: *mut RoutingInfo) -> tresult {
        kResultFalse
    }

    unsafe fn activateBus(&self, _type: MediaType, _dir: BusDirection, _index: int32, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setActive(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }
}

impl IPluginBaseTrait for Processor {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}
