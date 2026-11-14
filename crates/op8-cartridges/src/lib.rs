//! The native cartridges. Each one depends only on the
//! `op8-cartridge` contract — none of them know the sequencer or the MK3
//! exist.

pub mod drums;
pub mod dsp;
pub mod fm_bass;
pub mod kick_syn;
pub mod macro_osc;
pub mod midi_out;
pub mod resonator;
pub mod ui;

pub use drums::Drums;
pub use fm_bass::FmBass;
pub use kick_syn::KickSyn;
pub use macro_osc::MacroOsc;
pub use midi_out::{MidiMessage, MidiOutBridge};
pub use resonator::Resonator;

use op8_cartridge::InstrumentCartridge;

/// Every built-in cartridge, by name, for the slot browser.
pub fn builtin_names() -> &'static [&'static str] {
    &["drums", "KickSyn", "FmBass", "resonator", "macro"]
}

/// Construct a built-in audio cartridge by name. (MidiOutBridge is created
/// separately because it hands back the queue the MIDI thread drains.)
pub fn create(name: &str) -> Option<Box<dyn InstrumentCartridge>> {
    match name {
        "KickSyn" => Some(Box::new(KickSyn::new())),
        "FmBass" => Some(Box::new(FmBass::new())),
        "macro" => Some(Box::new(MacroOsc::new())),
        "drums" => Some(Box::new(Drums::new())),
        "resonator" => Some(Box::new(Resonator::new())),
        _ => None,
    }
}

#[cfg(test)]
mod contract_tests {
    //! Checks every cartridge must pass.
    use super::*;
    use op8_cartridge::{FrameBuffer480x272, NoteEvent, Rgb565, MAX_BLOCK};

    fn all() -> Vec<Box<dyn InstrumentCartridge>> {
        let mut v: Vec<Box<dyn InstrumentCartridge>> =
            builtin_names().iter().map(|n| create(n).unwrap()).collect();
        v.push(Box::new(MidiOutBridge::new(0).0));
        v
    }

    #[test]
    fn outputs_are_finite_and_bounded() {
        for mut c in all() {
            let (mut l, mut r) = (vec![9.0f32; MAX_BLOCK], vec![9.0f32; MAX_BLOCK]);
            for p in 0..4 {
                c.set_parameter(p, 1.0);
            }
            c.process(
                &mut l,
                &mut r,
                &[NoteEvent::on(0, 36, 1.0), NoteEvent::on(10, 48, 1.0)],
            );
            for _ in 0..100 {
                c.process(&mut l, &mut r, &[]);
                assert!(
                    l.iter().chain(&r).all(|s| s.is_finite() && s.abs() <= 1.5),
                    "{}",
                    c.info().name
                );
            }
        }
    }

    #[test]
    fn silence_without_notes_and_after_release() {
        for mut c in all() {
            let (mut l, mut r) = (vec![0.0f32; 512], vec![0.0f32; 512]);
            c.process(&mut l, &mut r, &[]);
            assert!(l.iter().all(|&s| s == 0.0), "{} not silent", c.info().name);
            c.process(
                &mut l,
                &mut r,
                &[NoteEvent::on(0, 40, 1.0), NoteEvent::off(100, 40)],
            );
            for _ in 0..1000 {
                c.process(&mut l, &mut r, &[]);
            }
            assert!(
                l.iter().all(|&s| s.abs() < 1e-3),
                "{} rings forever",
                c.info().name
            );
        }
    }

    #[test]
    fn state_round_trips() {
        for mut c in all() {
            c.set_parameter(0, 0.123);
            c.set_parameter(2, 0.877);
            let blob = c.save_state();
            let mut d = create(&c.info().name).unwrap_or_else(|| Box::new(MidiOutBridge::new(0).0));
            d.load_state(&blob);
            assert_eq!(d.save_state(), blob, "{}", c.info().name);
            d.load_state(&[0xff, 1, 2]); // garbage must not panic
        }
    }

    #[test]
    fn draws_something() {
        for c in all() {
            let mut fb = FrameBuffer480x272::new();
            c.draw_ui(&mut fb, Rgb565::from_hex(0xff5500));
            assert!(fb.pixels().iter().any(|&p| p != 0), "{}", c.info().name);
        }
    }
}
