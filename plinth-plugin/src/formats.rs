use std::fmt::Display;

#[cfg(target_os="macos")]
pub mod auv3;
pub mod clap;
#[cfg(feature = "standalone")]
pub mod standalone;
pub mod vst3;

#[derive(Clone, Copy, Debug)]
pub enum PluginFormat {
    Auv3,
    Clap,
    #[cfg(feature = "standalone")]
    Standalone,
    Vst3,
}

impl Display for PluginFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginFormat::Auv3 => f.write_str("AUv3"),
            PluginFormat::Clap => f.write_str("CLAP"),
            #[cfg(feature = "standalone")]
            PluginFormat::Standalone => f.write_str("Standalone"),
            PluginFormat::Vst3 => f.write_str("VST3"),
        }
    }
}
