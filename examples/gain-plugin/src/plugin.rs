use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::rc::Rc;

use plinth_plugin::error::Error;
use plinth_plugin::{Host, HostInfo, Parameters, Plugin, ProcessorConfig};
use plinth_plugin::clap::ClapPlugin;
use plinth_plugin::vst3::Vst3Plugin;

use crate::editor::{EditorSettings, GainPluginEditor};
use crate::{parameters::GainParameters, processor::GainPluginProcessor};

mod metadata {
    // consts from Cargo.toml [package.metadata.bundle]
    plinth_derive::bundle_metadata!();
}

#[derive(Default)]
pub struct GainPlugin {
    parameters: Rc<GainParameters>,
    editor_settings: Rc<RefCell<EditorSettings>>,
}

impl Plugin for GainPlugin {
    const NAME: &'static str = metadata::NAME;
    const VENDOR: &'static str = metadata::VENDOR;
    const VERSION: &'static str = metadata::VERSION;

    type Processor = GainPluginProcessor;
    type Editor = GainPluginEditor;
    type Parameters = GainParameters;

    fn new(_host_info: HostInfo) -> Self {
        Self::default()
    }

    fn init(&mut self) {
    }

    fn with_parameters<T>(&self, mut f: impl FnMut(&Self::Parameters) -> T) -> T {
        f(&self.parameters)
    }

    fn create_processor(&self, _config: ProcessorConfig) -> Self::Processor {
        GainPluginProcessor::new((*self.parameters).clone())
    }

    fn create_editor(&self, host: Rc<dyn Host>) -> Self::Editor {
        GainPluginEditor::new(host, self.parameters.clone(), self.editor_settings.clone())
    }

    fn save_state(&self, writer: &mut impl Write) -> Result<(), Error> {
        let serialized_parameters: HashMap<_, _> = self.parameters.serialize().collect();
        let parameters_json = serde_json::to_string(&serialized_parameters)
            .map_err(|_| Error::SerializationError)?;
        write!(writer, "{parameters_json}")?;

        Ok(())
    }

    fn load_state(&mut self, reader: &mut impl Read) -> Result<(), Error> {
        let mut parameters_json = String::new();
        reader.read_to_string(&mut parameters_json)?;

        let serialized_parameters: HashMap<_, _> = serde_json::from_str(&parameters_json)
            .map_err(|_| Error::SerializationError)?;
        self.parameters.deserialize(serialized_parameters, true)?;

        Ok(())
    }
}

impl ClapPlugin for GainPlugin {
    const CLAP_ID: &'static str = metadata::CLAP_ID;
    const FEATURES: &'static [plinth_plugin::clap::Feature] = &[
        plinth_plugin::clap::Feature::AudioEffect,
        plinth_plugin::clap::Feature::Stereo,
    ];
}

impl Vst3Plugin for GainPlugin {
    const CLASS_ID: u128 = metadata::VST3_CLASS_ID;
    const SUBCATEGORIES: &'static [plinth_plugin::vst3::Subcategory] = &[
        plinth_plugin::vst3::Subcategory::Fx,
        plinth_plugin::vst3::Subcategory::Stereo,
    ];
}

#[cfg(feature = "clap")]
plinth_plugin::export_clap!(GainPlugin);
#[cfg(feature = "vst3")]
plinth_plugin::export_vst3!(GainPlugin);

#[cfg(feature = "standalone")]
impl plinth_plugin::standalone::StandalonePlugin for GainPlugin {}
