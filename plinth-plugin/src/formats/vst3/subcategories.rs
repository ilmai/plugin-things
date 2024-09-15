pub enum Subcategory {
    Fx,
    Instrument,

    Analyzer,
    Delay,
    Distortion,
    Drum,
    Dynamics,
    Eq,
    External,
    Filter,
    Generator,
    Mastering,
    Modulation,
    Network,
    Piano,
    PitchShift,
    Restoration,
    Reverb,
    Sampler,
    Spatial,
    Synth,
    Tools,
    UpDownMix,

    Ambisonics,
    Mono,
    Stereo,
    Surround,
}

impl Subcategory {
    pub fn to_str(&self) -> &str {
        match self {
            Subcategory::Fx => "Fx",
            Subcategory::Instrument => "Instrument",

            Subcategory::Analyzer => "Analyzer",
            Subcategory::Delay => "Delay",
            Subcategory::Distortion => "Distortion",
            Subcategory::Drum => "Drum",
            Subcategory::Dynamics => "Dynamics",
            Subcategory::Eq => "EQ",
            Subcategory::External => "External",
            Subcategory::Filter => "Filter",
            Subcategory::Generator => "Generator",
            Subcategory::Mastering => "Mastering",
            Subcategory::Modulation => "Modulation",
            Subcategory::Network => "Network",
            Subcategory::Piano => "Piano",
            Subcategory::PitchShift => "Pitch Shift",
            Subcategory::Restoration => "Restoration",
            Subcategory::Reverb => "Reverb",
            Subcategory::Sampler => "Sampler",
            Subcategory::Spatial => "Spatial",
            Subcategory::Synth => "Synth",
            Subcategory::Tools => "Tools",
            Subcategory::UpDownMix => "Up-Downmix",
 
            Subcategory::Ambisonics => "Ambisonics",
            Subcategory::Mono => "Mono",
            Subcategory::Stereo => "Stereo",
            Subcategory::Surround => "Surround",
        }
    }
}
