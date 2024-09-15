use clap_sys::{events::{clap_event_transport, CLAP_TRANSPORT_IS_PLAYING}, fixedpoint::CLAP_SECTIME_FACTOR};

use crate::Transport;

pub fn convert_transport(transport: &clap_event_transport, sample_rate: f64) -> Transport {
    let position_seconds = transport.song_pos_seconds as f64 / CLAP_SECTIME_FACTOR as f64;

    Transport {
        playing: transport.flags & CLAP_TRANSPORT_IS_PLAYING > 0,
        tempo: transport.tempo,
        position_samples: f64::round(position_seconds * sample_rate) as _,
    }
}
