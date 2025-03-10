use std::collections::HashMap;
use std::io::{Read, Result, Write};
use std::rc::Rc;

use plinth_plugin::{export_clap, export_vst3, Event, Host, Parameters, Plugin, ProcessorConfig};
use plinth_plugin::clap::ClapPlugin;
use plinth_plugin::vst3::Vst3Plugin;

use crate::editor::GainPluginEditor;
use crate::{parameters::GainParameters, processor::GainPluginProcessor};

#[derive(Default)]
struct GainPlugin {
    parameters: Rc<GainParameters>,
}

impl Plugin for GainPlugin {
    const NAME: &'static str = "Gain Example";
    const VENDOR: &'static str = "Viiri Audio";
    const VERSION: &'static str = "0.1";

    type Processor = GainPluginProcessor;
    type Editor = GainPluginEditor;
    type Parameters = GainParameters;

    fn with_parameters<T>(&self, mut f: impl FnMut(&Self::Parameters) -> T) -> T {
        f(&self.parameters)
    }

    fn process_event(&mut self, event: &Event) {
        self.parameters.process_event(event);
    }

    fn create_processor(&mut self, _config: &ProcessorConfig) -> Self::Processor {
        GainPluginProcessor::new((*self.parameters).clone())
    }

    fn create_editor(&mut self, host: Rc<dyn Host>) -> Self::Editor {
        GainPluginEditor::new(host, self.parameters.clone())
    }

    fn save_state(&self, writer: &mut impl Write) -> Result<()> {        
        let serialized_parameters: HashMap<_, _> = self.parameters.serialize().collect();
        let parameters_json = serde_json::to_string(&serialized_parameters)?;
        write!(writer, "{parameters_json}")
    }

    fn load_state(&mut self, reader: &mut impl Read) -> Result<()> {
        let mut parameters_json = String::new();
        reader.read_to_string(&mut parameters_json)?;

        let serialized_parameters: HashMap<_, _> = serde_json::from_str(&parameters_json)?;
        self.parameters.deserialize(serialized_parameters);

        Ok(())
    }
}

impl ClapPlugin for GainPlugin {
    const CLAP_ID: &'static str = "viiri-audio.gain-example";
    const FEATURES: &'static [plinth_plugin::clap::Feature] = &[
        plinth_plugin::clap::Feature::AudioEffect,
        plinth_plugin::clap::Feature::Stereo,
    ];
}

impl Vst3Plugin for GainPlugin {
    const CLASS_ID: u128 = 0xE84410DB1788DC81;
    const SUBCATEGORIES: &'static [plinth_plugin::vst3::Subcategory] = &[
        plinth_plugin::vst3::Subcategory::Fx,
        plinth_plugin::vst3::Subcategory::Stereo,
    ];
}

export_clap!(GainPlugin);
export_vst3!(GainPlugin);
