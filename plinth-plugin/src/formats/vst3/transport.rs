use vst3::Steinberg::Vst::{ProcessContext, ProcessContext_::StatesAndFlags_::kPlaying};

use crate::Transport;

impl From<&ProcessContext> for Transport {
    fn from(context: &ProcessContext) -> Self {
        Self {
            // This cast is needed on some platforms
            #[allow(clippy::unnecessary_cast)]
            playing: context.state & kPlaying as u32 > 0,
            tempo: context.tempo,
            position_samples: context.projectTimeSamples,
        }
    }
}
