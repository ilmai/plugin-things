use std::{io::{Read, Write}, rc::Rc};

use crate::{error::Error, host::HostInfo, midi_capabilities::MidiCapabilities, note_expressions::NoteExpressions, processor::ProcessorConfig, Editor, Host, Parameters, Processor};

pub trait Plugin {
    const NAME: &'static str;
    const VENDOR: &'static str;
    const VERSION: &'static str;

    const URL: Option<&'static str> = None;

    const HAS_AUX_INPUT: bool = false;
    // Enables note, midi event input ports.
    const HAS_NOTE_INPUT: bool = false;
    // Enables note, midi event output ports (currently unused).
    const HAS_NOTE_OUTPUT: bool = false;
    // Enables delivery of the specified per-note expression dimensions (VST3 note expression / CLAP note expression) when HAS_NOTE_INPUT is true.
    const NOTE_EXPRESSIONS: NoteExpressions = if Self::HAS_NOTE_INPUT { NoteExpressions::DEFAULT } else { NoteExpressions::NONE };
    // Enables specified MIDI events when HAS_NOTE_INPUT is true. Creates hidden parameter overhead for VST3, so keep disabled when unused.
    const MIDI_CAPABILITIES: MidiCapabilities = MidiCapabilities::NONE;

    type Processor: Processor;
    type Editor: Editor;
    type Parameters: Parameters;

    fn new(host_info: HostInfo) -> Self;
    fn init(&mut self);

    fn with_parameters<T>(&self, f: impl FnMut(&Self::Parameters) -> T) -> T;

    fn create_processor(&self, config: ProcessorConfig) -> Self::Processor;
    fn create_editor(&self, host: Rc<dyn Host>) -> Self::Editor;

    fn save_state(&self, writer: &mut impl Write) -> Result<(), Error>;
    fn load_state(&mut self, reader: &mut impl Read) -> Result<(), Error>;

    fn latency(&self) -> u32 {
        0
    }
}
