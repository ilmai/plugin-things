use std::sync::mpsc::Sender;

use midir::{MidiInput, MidiInputConnection};

use crate::Event;

pub fn connect_midi_inputs(sender: Sender<Event>) -> Vec<MidiInputConnection<()>> {
    let midi_in = match MidiInput::new("plinth-standalone") {
        Ok(midi_in) => midi_in,
        Err(err) => {
            log::warn!("Failed to create MIDI input: {err}");
            return vec![];
        }
    };

    let ports = midi_in.ports();
    let mut connections = Vec::with_capacity(ports.len());

    for port in &ports {
        let port_name = midi_in.port_name(port).unwrap_or_else(|_| port.id());
        let midi_in = match MidiInput::new("plinth-standalone") {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Failed to create MIDI input for port '{port_name}': {e}");
                continue;
            }
        };

        let sender = sender.clone();
        match midi_in.connect(
            port,
            "plinth-midi-input",
            move |_timestamp, data, _| {
                if let Some(event) = parse_midi(data) {
                    let _ = sender.send(event);
                }
            },
            (),
        ) {
            Ok(conn) => connections.push(conn),
            Err(e) => log::warn!("Failed to connect to MIDI input port '{port_name}': {e}"),
        }
    }

    connections
}

fn parse_midi(data: &[u8]) -> Option<Event> {
    if data.len() < 2 {
        return None;
    }

    let status = data[0] & 0xF0;
    let channel = (data[0] & 0x0F) as i16;
    let key = data[1] as i16;
    let velocity = if data.len() >= 3 {
        data[2] as f64 / 127.0
    } else {
        0.0
    };

    match status {
        0x90 if data.len() >= 3 && data[2] > 0 => Some(Event::NoteOn {
            sample_offset: 0,
            channel,
            key,
            note: -1,
            velocity,
        }),
        0x80 | 0x90 => Some(Event::NoteOff {
            sample_offset: 0,
            channel,
            key,
            note: -1,
            velocity,
        }),
        0xE0 if data.len() >= 3 => {
            let lsb = data[1] as i16;
            let msb = data[2] as i16;
            let bend = (msb << 7 | lsb) - 8192;
            let semitones = bend as f64 / 8192.0 * 2.0;
            Some(Event::PitchBend {
                sample_offset: 0,
                channel,
                key: -1,
                note: -1,
                semitones,
            })
        }
        _ => None,
    }
}
