use clap_sys::{events::{clap_event_transport, clap_transport_flags, CLAP_TRANSPORT_HAS_BEATS_TIMELINE, CLAP_TRANSPORT_HAS_SECONDS_TIMELINE, CLAP_TRANSPORT_HAS_TEMPO, CLAP_TRANSPORT_HAS_TIME_SIGNATURE, CLAP_TRANSPORT_IS_LOOP_ACTIVE, CLAP_TRANSPORT_IS_PLAYING, CLAP_TRANSPORT_IS_RECORDING}, fixedpoint::{clap_beattime, clap_sectime, CLAP_BEATTIME_FACTOR, CLAP_SECTIME_FACTOR}};

use crate::{TimeSignature, Transport};

pub fn convert_transport(transport: &clap_event_transport, sample_rate: f64) -> Transport {
    let has_flag = |flag: clap_transport_flags| transport.flags & flag != 0;
    let to_samples = |time: clap_sectime|
        f64::round(time as f64 / CLAP_SECTIME_FACTOR as f64 * sample_rate) as i64;
    let to_beats = |time: clap_beattime| time as f64 / CLAP_BEATTIME_FACTOR as f64;

    Transport {
        playing: has_flag(CLAP_TRANSPORT_IS_PLAYING),
        recording: has_flag(CLAP_TRANSPORT_IS_RECORDING),
        tempo: has_flag(CLAP_TRANSPORT_HAS_TEMPO).then_some(transport.tempo),
        time_signature: has_flag(CLAP_TRANSPORT_HAS_TIME_SIGNATURE).then_some(TimeSignature {
            numerator: transport.tsig_num as _,
            denominator: transport.tsig_denom as _,
        }),
        position_samples: has_flag(CLAP_TRANSPORT_HAS_SECONDS_TIMELINE).then(||
            to_samples(transport.song_pos_seconds)),
        position_beats: has_flag(CLAP_TRANSPORT_HAS_BEATS_TIMELINE).then(||
            to_beats(transport.song_pos_beats)),
        bar_start_beats: has_flag(CLAP_TRANSPORT_HAS_BEATS_TIMELINE).then(||
            to_beats(transport.bar_start)),
        loop_active: has_flag(CLAP_TRANSPORT_IS_LOOP_ACTIVE),
        loop_range_beats: has_flag(CLAP_TRANSPORT_HAS_BEATS_TIMELINE).then(||
            to_beats(transport.loop_start_beats)..to_beats(transport.loop_end_beats)),
    }
}
