//! MidiOutBridge: drives external hardware (the MK3's DIN MIDI out, or any
//! other MIDI port) from a sequencer track.
//!
//! It produces no audio. Notes become MIDI messages tagged with their sample
//! position and pushed into a lock-free queue; the host's MIDI thread drains
//! it and writes to the port (see `op3::midi`).

use crate::ui;
use op3_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565,
};
use op3_gfx::{text, Align, FontSize};
use std::sync::Mutex;

/// A short MIDI message with its timing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiMessage {
    pub bytes: [u8; 3],
    pub len: u8,
    /// Absolute sample time (blocks processed × block size + offset).
    pub sample_time: u64,
}

impl MidiMessage {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

const CHANNELS: [&str; 16] = [
    "CH 1", "CH 2", "CH 3", "CH 4", "CH 5", "CH 6", "CH 7", "CH 8", "CH 9", "CH 10", "CH 11",
    "CH 12", "CH 13", "CH 14", "CH 15", "CH 16",
];
const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new("CHANNEL", 0.0, ParamUnit::Choice(&CHANNELS)),
    ParamDescriptor::new(
        "TRANSP",
        0.5,
        ParamUnit::Semitones {
            min: -24.0,
            max: 24.0,
        },
    ),
    ParamDescriptor::new("VEL", 1.0, ParamUnit::Percent),
    ParamDescriptor::new("CC74", 0.5, ParamUnit::Percent),
];
const STATE_VERSION: u8 = 1;
const RECENT: usize = 8;

pub struct MidiOutBridge {
    values: [f32; 4],
    queue: Mutex<rtrb::Producer<MidiMessage>>,
    clock: u64,
    /// Notes currently held on the external synth (so a channel change or a
    /// panic can release them).
    held: [u128; 16],
    last_cc74: Option<u8>,
    recent: [u8; RECENT],
    recent_len: usize,
    sent: u64,
    dropped: u64,
}

impl MidiOutBridge {
    /// Returns the cartridge and the consumer end the MIDI thread drains.
    pub fn new(_port_hint: usize) -> (Self, rtrb::Consumer<MidiMessage>) {
        let (p, c) = rtrb::RingBuffer::new(1024);
        let mut values = [0.0; 4];
        for (i, d) in PARAMS.iter().enumerate() {
            values[i] = d.default;
        }
        let bridge = Self {
            values,
            queue: Mutex::new(p),
            clock: 0,
            held: [0; 16],
            last_cc74: None,
            recent: [0; RECENT],
            recent_len: 0,
            sent: 0,
            dropped: 0,
        };
        (bridge, c)
    }

    fn channel(&self) -> u8 {
        choice_index(self.values[0], 16) as u8
    }

    fn push(&mut self, bytes: [u8; 3], len: u8, offset: u32) {
        let msg = MidiMessage {
            bytes,
            len,
            sample_time: self.clock + offset as u64,
        };
        // Only the audio thread ever locks this, so it never contends; the
        // Mutex exists to make the producer `Sync` as the trait requires.
        let ok = match self.queue.try_lock() {
            Ok(mut q) => q.push(msg).is_ok(),
            Err(_) => false,
        };
        if ok {
            self.sent += 1;
        } else {
            self.dropped += 1;
        }
    }

    fn release_all(&mut self, offset: u32) {
        for ch in 0..16u8 {
            let mut h = self.held[ch as usize];
            while h != 0 {
                let n = h.trailing_zeros() as u8;
                h &= h - 1;
                self.push([0x80 | ch, n, 0], 3, offset);
            }
            self.held[ch as usize] = 0;
        }
    }
}

impl InstrumentCartridge for MidiOutBridge {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("MidiOutBridge", "MIDI")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        output_l.fill(0.0);
        output_r.fill(0.0);
        let ch = self.channel();
        let cc = (self.values[3] * 127.0).round() as u8;
        if self.last_cc74 != Some(cc) {
            self.last_cc74 = Some(cc);
            self.push([0xB0 | ch, 74, cc], 3, 0);
        }
        let transpose = PARAMS[1].denormalize(self.values[1]).round() as i32;
        for e in events {
            let note = (e.note as i32 + transpose).clamp(0, 127) as u8;
            match e.kind {
                NoteKind::On => {
                    let v = ((e.velocity * self.values[2]) * 127.0)
                        .round()
                        .clamp(1.0, 127.0) as u8;
                    self.push([0x90 | ch, note, v], 3, e.sample_offset);
                    self.held[ch as usize] |= 1 << note;
                    if self.recent_len == RECENT {
                        self.recent.copy_within(1.., 0);
                        self.recent_len -= 1;
                    }
                    self.recent[self.recent_len] = note;
                    self.recent_len += 1;
                }
                NoteKind::Off => {
                    // Release on whatever channel the note was started on.
                    for c in 0..16u8 {
                        if self.held[c as usize] & (1 << note) != 0 {
                            self.held[c as usize] &= !(1 << note);
                            self.push([0x80 | c, note, 0], 3, e.sample_offset);
                        }
                    }
                }
            }
        }
        self.clock += output_l.len() as u64;
    }

    fn parameters(&self) -> &[ParamDescriptor; 4] {
        &PARAMS
    }
    fn set_parameter(&mut self, index: usize, normalized_value: f32) {
        if let Some(v) = self.values.get_mut(index) {
            *v = normalized_value.clamp(0.0, 1.0);
        }
    }
    fn parameter_value(&self, index: usize) -> Option<f32> {
        self.values.get(index).copied()
    }
    fn all_notes_off(&mut self) {
        self.release_all(0);
    }

    fn draw_ui(&self, frame: &mut FrameBuffer480x272, accent_color: Rgb565) {
        let y = ui::AREA_Y;
        let dim = Rgb565::WHITE.scale(0.6);
        text(
            frame,
            "DIN OUT",
            16,
            y + 4,
            FontSize::Small,
            dim,
            Align::Left,
        );
        text(
            frame,
            CHANNELS[self.channel() as usize],
            16,
            y + 24,
            FontSize::Large,
            accent_color,
            Align::Left,
        );
        text(
            frame,
            &format!("SENT {}", self.sent),
            16,
            y + 60,
            FontSize::Small,
            Rgb565::WHITE,
            Align::Left,
        );
        if self.dropped > 0 {
            text(
                frame,
                &format!("DROPPED {}", self.dropped),
                16,
                y + 74,
                FontSize::Small,
                Rgb565::from_hex(0xff3030),
                Align::Left,
            );
        }
        // Recent notes as a little piano-roll strip.
        text(
            frame,
            "RECENT",
            248,
            y + 4,
            FontSize::Small,
            dim,
            Align::Left,
        );
        op3_gfx::rect_outline(frame, 248, y + 20, 216, 160, 1, Rgb565::WHITE.scale(0.25));
        for (i, &n) in self.recent[..self.recent_len].iter().enumerate() {
            let x = 256 + i as i32 * 26;
            let h = (n as i32 * 150) / 127;
            frame.fill_rect(x, y + 176 - h, 18, 2, accent_color);
            text(
                frame,
                note_name(n),
                x + 9,
                y + 182 - h - 14,
                FontSize::Small,
                Rgb565::WHITE,
                Align::Center,
            );
        }
    }

    fn save_state(&self) -> Vec<u8> {
        state::encode_f32s(STATE_VERSION, &self.values)
    }
    fn load_state(&mut self, data: &[u8]) {
        state::decode_f32s(STATE_VERSION, data, &mut self.values);
    }
}

fn note_name(n: u8) -> &'static str {
    [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ][(n % 12) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_become_midi_with_timing() {
        let (mut b, mut rx) = MidiOutBridge::new(0);
        b.set_parameter(0, 2.5 / 16.0); // channel 3
        let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
        b.process(&mut l, &mut r, &[]);
        b.process(
            &mut l,
            &mut r,
            &[NoteEvent::on(5, 60, 1.0), NoteEvent::off(40, 60)],
        );
        let msgs: Vec<MidiMessage> = std::iter::from_fn(|| rx.pop().ok()).collect();
        assert_eq!(msgs[0].as_bytes(), &[0xB2, 74, 64]); // CC74 initial value
        assert_eq!(msgs[1].as_bytes(), &[0x92, 60, 127]);
        assert_eq!(msgs[1].sample_time, 64 + 5);
        assert_eq!(msgs[2].as_bytes(), &[0x82, 60, 0]);
        assert_eq!(msgs[2].sample_time, 64 + 40);
    }

    #[test]
    fn note_off_follows_the_channel_it_started_on() {
        let (mut b, mut rx) = MidiOutBridge::new(0);
        let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
        b.process(&mut l, &mut r, &[NoteEvent::on(0, 60, 1.0)]);
        b.set_parameter(0, 1.0); // channel 16
        b.process(&mut l, &mut r, &[NoteEvent::off(0, 60)]);
        let msgs: Vec<MidiMessage> = std::iter::from_fn(|| rx.pop().ok()).collect();
        assert!(msgs.iter().any(|m| m.as_bytes() == [0x80, 60, 0]));
    }
}
