pub struct Transport {
    pub(crate) playing: bool,
    pub(crate) tempo: f64,
    pub(crate) position_samples: i64,
}

impl Transport {
    pub fn new(playing: bool, tempo: f64, position_samples: i64) -> Self {
        Self {
            playing,
            tempo,
            position_samples,
        }
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn tempo(&self) -> f64 {
        self.tempo
    }

    pub fn position_samples(&self) -> i64 {
        self.position_samples
    }
}
