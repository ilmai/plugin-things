use std::{io::{Read, Write}, rc::Rc};

use crate::{processor::ProcessorConfig, Editor, Event, Host, Parameters, Processor};

pub trait Plugin: Default {
    const NAME: &'static str;
    const VENDOR: &'static str;
    const VERSION: &'static str;
    
    const URL: Option<&'static str> = None;

    const HAS_AUX_INPUT: bool = false;
    const HAS_NOTE_INPUT: bool = false;
    const HAS_NOTE_OUTPUT: bool = false;

    type Processor: Processor;
    type Editor: Editor;
    type Parameters: Parameters;

    fn with_parameters<T>(&self, f: impl FnMut(&Self::Parameters) -> T) -> T;
    fn process_event(&mut self, event: &Event);

    fn create_processor(&mut self, config: &ProcessorConfig) -> Self::Processor;
    fn create_editor(&mut self, host: Rc<dyn Host>) -> Self::Editor;

    fn save_state(&self, writer: &mut impl Write) -> std::io::Result<()>;
    fn load_state(&mut self, reader: &mut impl Read) -> std::io::Result<()>;

    fn latency(&self) -> u32 {
        0
    }
}
