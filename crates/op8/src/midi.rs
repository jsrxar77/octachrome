//! MIDI output for the MidiOutBridge cartridge.
//!
//! The bridge queues messages on the audio thread; [`spawn`] drains that
//! queue on its own thread and writes them to a port:
//!
//! * Linux: the ALSA raw-MIDI node of the MK3's DIN jacks
//!   (`/dev/snd/midiC<card>D0`), or `OP8_MIDI_PORT=/dev/snd/midiC1D0`.
//! * macOS: a CoreMIDI port via `midir` whose name contains "Maschine", or
//!   `OP8_MIDI_PORT=<name substring>`.
//!
//! Messages go out as soon as they are drained (≤ 1 ms + one audio block
//! late). Scheduling against `sample_time` is a TODO for tight external sync.

use op8_cartridges::MidiMessage;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub trait MidiPort: Send {
    fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()>;
    fn name(&self) -> String;
}

/// Swallows everything (no port found, or the simulator).
pub struct NullPort;
impl MidiPort for NullPort {
    fn send(&mut self, _: &[u8]) -> anyhow::Result<()> {
        Ok(())
    }
    fn name(&self) -> String {
        "none".into()
    }
}

#[cfg(target_os = "linux")]
pub fn open_default() -> anyhow::Result<Box<dyn MidiPort>> {
    use anyhow::Context;
    use std::io::Write;
    struct Raw(std::fs::File, String);
    impl MidiPort for Raw {
        fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
            self.0.write_all(bytes)?;
            Ok(())
        }
        fn name(&self) -> String {
            self.1.clone()
        }
    }
    let path = match std::env::var("OP8_MIDI_PORT") {
        Ok(p) => p,
        Err(_) => {
            let cards = std::fs::read_to_string("/proc/asound/cards")
                .context("reading /proc/asound/cards")?;
            let card = cards
                .lines()
                .find(|l| l.contains("Maschine"))
                .and_then(|l| l.split_whitespace().next())
                .context("no Maschine sound card (set OP8_MIDI_PORT)")?;
            format!("/dev/snd/midiC{card}D0")
        }
    };
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .with_context(|| format!("opening {path}"))?;
    Ok(Box::new(Raw(f, path)))
}

#[cfg(all(target_os = "macos", feature = "midi"))]
pub fn open_default() -> anyhow::Result<Box<dyn MidiPort>> {
    use anyhow::{anyhow, Context};
    struct Core(midir::MidiOutputConnection, String);
    impl MidiPort for Core {
        fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
            self.0.send(bytes).map_err(|e| anyhow!("{e}"))
        }
        fn name(&self) -> String {
            self.1.clone()
        }
    }
    let want = std::env::var("OP8_MIDI_PORT").unwrap_or_else(|_| "Maschine".into());
    let out = midir::MidiOutput::new("OP-8").context("CoreMIDI")?;
    let port = out
        .ports()
        .into_iter()
        .find(|p| out.port_name(p).map_or(false, |n| n.contains(&want)))
        .with_context(|| format!("no MIDI output matching {want:?}"))?;
    let name = out.port_name(&port).unwrap_or_default();
    let conn = out.connect(&port, "op8-out").map_err(|e| anyhow!("{e}"))?;
    Ok(Box::new(Core(conn, name)))
}

#[cfg(not(any(target_os = "linux", all(target_os = "macos", feature = "midi"))))]
pub fn open_default() -> anyhow::Result<Box<dyn MidiPort>> {
    anyhow::bail!("no MIDI backend on this platform/feature set")
}

/// Drain `rx` into `port` until `stop` is set.
pub fn spawn(
    mut rx: rtrb::Consumer<MidiMessage>,
    mut port: Box<dyn MidiPort>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("op8-midi".into())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                while let Ok(m) = rx.pop() {
                    if let Err(e) = port.send(m.as_bytes()) {
                        eprintln!("midi: {e}");
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        })
        .expect("spawn midi thread")
}
