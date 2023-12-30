use std::ffi::{c_uint, c_void};

use icrate::block2::Block;
use objc2::{extern_class, mutability, ClassType, runtime::NSObject, Encode, Encoding, RefEncode};

#[repr(isize)]
pub enum AUAudioUnitBusType {
    Input = 1,
    Output = 2,
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AURenderEventType {
    Parameter = 1,
    ParameterRamp = 2,
    MIDI = 8,
    MIDISysEx = 9,
}

unsafe impl Encode for AURenderEventType {
    const ENCODING: Encoding = Encoding::UChar;
}

#[repr(u32)]
pub enum AudioTimeStampFlags {
    NothingValid = 0,
    SampleTimeValid = 1 << 0,
    HostTimeValid = 1 << 1,
    RateScalarValid = 1 << 2,
    WordClockTimeValid = 1 << 3,
    SMPTETimeValid = 1 << 4,
}

unsafe impl Encode for AudioTimeStampFlags {
    const ENCODING: Encoding = Encoding::ULong;
}

#[repr(u32)]
pub enum AudioUnitRenderActionFlags {
    PreRender = 1 << 2,
    PostRender = 1 << 3,
    OutputIsSilence = 1 << 4,
    Preflight = 1 << 5,
    Render = 1 << 6,
    Complete = 1 << 7,
    PostRenderError = 1 << 8,
    DoNotCheckRenderArgs = 1 << 9,
}

unsafe impl Encode for AudioUnitRenderActionFlags {
    const ENCODING: Encoding = Encoding::ULong;
}

unsafe impl RefEncode for AudioUnitRenderActionFlags {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[repr(u32)]
pub enum SMPTETimeFlags {
    Unknown = 0,
    Valid = 1 << 0,
    Running = 1 << 1,
}

unsafe impl Encode for SMPTETimeFlags {
    const ENCODING: Encoding = Encoding::ULong;
}

#[repr(u32)]
pub enum SMPTETimeType {
    Type24 = 0,
    Type25 = 1,
    Type30Drop = 2,
    Type30 = 3,
    Type2997 = 4,
    Type2997Drop = 5,
    Type60 = 6,
    Type5994 = 7,
    Type60Drop = 8,
    Type5994Drop = 9,
    Type50 = 10,
    Type2398 = 11,
}

unsafe impl Encode for SMPTETimeType {
    const ENCODING: Encoding = Encoding::ULong;
}

pub type AUAudioFrameCount = u32;
pub type AUAudioUnitStatus = c_uint;

#[repr(C)]
pub struct AudioBuffer {
    number_channels: u32,
    data_byte_size: u32,
    data: *mut c_void,
}

unsafe impl Encode for AudioBuffer {
    const ENCODING: Encoding = Encoding::Struct(
        "AudioBuffer",
        &[
            Encoding::ULong,
            Encoding::ULong,
            Encoding::Pointer(&Encoding::Void),
        ]
    );
}

unsafe impl RefEncode for AudioBuffer {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[repr(C)]
pub struct AudioBufferList {
    number_buffers: u32,
    buffers: *mut AudioBuffer,
}

unsafe impl Encode for AudioBufferList {
    const ENCODING: Encoding = Encoding::Struct(
        "AudioBufferList",
        &[
            Encoding::ULong,
            AudioBuffer::ENCODING_REF,
        ]
    );
}

unsafe impl RefEncode for AudioBufferList {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AURenderEventHeader {
    next: *const AURenderEvent,
    event_sample_time: i64,
    event_type: AURenderEventType,
    reserved: u8,
}

unsafe impl Encode for AURenderEventHeader {
    const ENCODING: Encoding = Encoding::Struct(
        "AURenderEventHeader",
        &[
            Encoding::Pointer(&Encoding::Void),
            Encoding::LongLong,
            AURenderEventType::ENCODING,
            Encoding::UChar,
        ],
    );
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AUParameterEvent {
    next: *const AURenderEvent,
    event_sample_time: i64,
    event_type: AURenderEventType,
    reserved: [u8; 3],
    ramp_duration_sample_frames: u32,
    parameter_address: u64,
    value: f32,
}

unsafe impl Encode for AUParameterEvent {
    const ENCODING: Encoding = Encoding::Struct(
        "AUParameterEvent",
        &[
            Encoding::Pointer(&Encoding::Void),
            Encoding::LongLong,
            AURenderEventType::ENCODING,
            Encoding::Array(3, &Encoding::UChar),
            Encoding::ULong,
            Encoding::ULongLong,
            Encoding::Float,
        ],
    );
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AUMIDIEvent {
    next: *const AURenderEvent,
    event_sample_time: i64,
    event_type: AURenderEventType,
    reserved: u8,
    length: u16,
    cable: u8,
    data: [u8; 3],
}

unsafe impl Encode for AUMIDIEvent {
    const ENCODING: Encoding = Encoding::Struct(
        "AUMIDIEvent",
        &[
            Encoding::Pointer(&Encoding::Void),
            Encoding::LongLong,
            AURenderEventType::ENCODING,
            Encoding::UChar,
            Encoding::UShort,
            Encoding::UChar,
            Encoding::Array(3, &Encoding::UChar),
        ],
    );
}

#[repr(C)]
pub union AURenderEvent {
    header: AURenderEventHeader,
    parameter: AUParameterEvent,
    midi: AUMIDIEvent,
}

unsafe impl Encode for AURenderEvent {
    const ENCODING: Encoding = Encoding::Union(
        "AURenderEvent", 
        &[
            AURenderEventHeader::ENCODING,
            AUParameterEvent::ENCODING,
            AUMIDIEvent::ENCODING,
        ]
    );
}

unsafe impl RefEncode for AURenderEvent {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[repr(C)]
pub struct AudioTimeStamp {
    sample_time: f64,
    host_time: u64,
    rate_scalar: f64,
    word_clock_time: u64,
    smpte_time: SMPTETime,
    flags: AudioTimeStampFlags,
    _reserved: u32,
}

unsafe impl Encode for AudioTimeStamp {
    const ENCODING: Encoding = Encoding::Struct(
        "AudioTimeStamp",
        &[
            Encoding::Double,
            Encoding::ULongLong,
            Encoding::Double,
            Encoding::ULongLong,
            SMPTETime::ENCODING,
            AudioTimeStampFlags::ENCODING,
            Encoding::ULong,
        ],
    );
}

unsafe impl RefEncode for AudioTimeStamp {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[repr(transparent)]
pub struct AUInternalRenderBlock(pub Block<(
    *const AudioUnitRenderActionFlags,
    *const AudioTimeStamp,
    AUAudioFrameCount,
    isize,
    *mut AudioBufferList,
    *const AURenderEvent,
    AURenderPullInputBlock,
), AUAudioUnitStatus>);

unsafe impl Encode for AUInternalRenderBlock {
    const ENCODING: Encoding = Encoding::Block;
}

#[repr(transparent)]
pub struct AURenderPullInputBlock(Block<(
), AUAudioUnitStatus>);

unsafe impl Encode for AURenderPullInputBlock {
    const ENCODING: Encoding = Encoding::Block;
}

// typedef AUAudioUnitStatus (^AURenderPullInputBlock)(AudioUnitRenderActionFlags *actionFlags, const AudioTimeStamp *timestamp, AUAudioFrameCount frameCount, NSInteger inputBusNumber, AudioBufferList *inputData);

#[repr(C)]
pub struct SMPTETime {
    subframes: i16,
    subframe_divisor: i16,
    counter: u32,
    time_type: SMPTETimeType,
    flags: SMPTETimeFlags,
    hours: i16,
    minutes: i16,
    seconds: i16,
    frames: i16,
}

unsafe impl Encode for SMPTETime {
    const ENCODING: Encoding = Encoding::Struct(
        "SMPTETime",
        &[
            Encoding::Short,
            Encoding::Short,
            Encoding::ULong,
            SMPTETimeType::ENCODING,
            SMPTETimeFlags::ENCODING,
            Encoding::Short,
            Encoding::Short,
            Encoding::Short,
            Encoding::Short,
        ]
    );
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
