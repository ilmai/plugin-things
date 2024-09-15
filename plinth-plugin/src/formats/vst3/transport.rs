use vst3::Steinberg::Vst::{ProcessContext, ProcessContext_::StatesAndFlags_::kPlaying};

use crate::Transport;

impl From<&ProcessContext> for Transport {
    fn from(context: &ProcessContext) -> Self {
        Self {
            playing: context.state & kPlaying as u32 > 0,
            tempo: context.tempo,
            position_samples: context.projectTimeSamples,
        }
    }
}
