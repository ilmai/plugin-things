use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSignature {
    pub numerator: u32,
    pub denominator: u32,
}

pub struct Transport {
    pub(crate) playing: bool,
    pub(crate) recording: bool,
    pub(crate) tempo: Option<f64>,
    pub(crate) time_signature: Option<TimeSignature>,
    pub(crate) position_samples: Option<i64>,
    pub(crate) position_beats: Option<f64>,
    pub(crate) bar_start_beats: Option<f64>,
    pub(crate) loop_active: bool,
    pub(crate) loop_range_beats: Option<Range<f64>>,
}

impl Transport {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playing: bool,
        recording: bool,
        tempo: Option<f64>,
        time_signature: Option<TimeSignature>,
        position_samples: Option<i64>,
        position_beats: Option<f64>,
        bar_start_beats: Option<f64>,
        loop_active: bool,
        loop_range_beats: Option<Range<f64>>,
    ) -> Self {
        Self {
            playing,
            recording,
            tempo,
            time_signature,
            position_samples,
            position_beats,
            bar_start_beats,
            loop_active,
            loop_range_beats,
        }
    }

    /// True when the host's transport is playing.
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// True when the host is recording.
    pub fn recording(&self) -> bool {
        self.recording
    }

    /// Tempo in beats per minute, if provided by the host.
    pub fn tempo(&self) -> Option<f64> {
        self.tempo
    }

    /// Time signature, if provided by the host.
    pub fn time_signature(&self) -> Option<TimeSignature> {
        self.time_signature
    }

    /// Song position in samples, if provided by the host.
    pub fn position_samples(&self) -> Option<i64> {
        self.position_samples
    }

    /// Song position in beats (quarter notes), if provided by the host.
    pub fn position_beats(&self) -> Option<f64> {
        self.position_beats
    }

    /// Song position of the current bar's start in beats (quarter notes), if provided by the host.
    pub fn bar_start_beats(&self) -> Option<f64> {
        self.bar_start_beats
    }

    /// True when the host's loop (cycle) is enabled.
    pub fn loop_active(&self) -> bool {
        self.loop_active
    }

    /// Loop start and end in beats (quarter notes), if provided by the host.
    pub fn loop_range_beats(&self) -> Option<Range<f64>> {
        self.loop_range_beats.clone()
    }
}
