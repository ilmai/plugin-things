use std::mem::ManuallyDrop;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum AURenderEventType {
	AURenderEventParameter		= 1,
	AURenderEventParameterRamp	= 2,
	AURenderEventMIDI			= 8,
	AURenderEventMIDISysEx		= 9,
	AURenderEventMIDIEventList  = 10
}

#[repr(C)]
pub struct AURenderEventHeader {
    pub next: *mut AURenderEvent,
    pub event_sample_time: i64,
    pub event_type: AURenderEventType,
    _reserved: u8,
}

#[repr(C)]
pub struct AUParameterEvent {
    pub next: *mut AURenderEvent,
    pub event_sample_time: i64,
    pub event_type: AURenderEventType,
    pub _reserved: [u8; 3],
    pub ramp_duration_sample_frames: u32,
    pub parameter_address: u64,
    pub value: f32,
}

#[repr(C)]
pub struct AUMIDIEvent {
    pub next: *mut AURenderEvent,
    pub event_sample_time: i64,
    pub event_type: AURenderEventType,
    _reserved: u8,
    pub length: u16,
    pub cable: u8,
    pub data: [u8; 3],
}

#[repr(C)]
pub struct AUMIDIEventList {
    pub next: *mut AURenderEvent,
    pub event_sample_time: i64,
    pub event_type: AURenderEventType,
    _reserved: u8,
    pub cable: u8,
    pub event_list: MIDIEventList,
}

#[repr(C)]
pub union AURenderEvent {
	pub header: ManuallyDrop<AURenderEventHeader>,
    pub parameter: ManuallyDrop<AUParameterEvent>,
    pub midi: ManuallyDrop<AUMIDIEvent>,
    pub midi_events_list: ManuallyDrop<AUMIDIEventList>,
}

#[repr(i32)]
#[allow(dead_code)]
pub enum MIDIProtocolId {
    Protocol1_0 = 1,
    Protocol2_0 = 2,
}

#[repr(C)]
pub struct MIDIEventPacket {
    pub time_stamp: u64,
    pub word_count: u32,
    pub words: [u32; 64],
}

#[repr(C)]
pub struct MIDIEventList {
    pub protocol: MIDIProtocolId,
    pub num_packets: u32,
    pub packet: [MIDIEventPacket; 1],
}
