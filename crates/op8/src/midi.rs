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

/// The MIDI output ports the system has, by the names the player sees in SETTINGS.
#[cfg(target_os = "linux")]
pub fn list_ports() -> Vec<String> {
    linux_ports().into_iter().map(|(label, _)| label).collect()
}

/// (name the player sees, device node) of every raw-MIDI node of the sound cards: `Maschine MK3 · midiC1D0`.
#[cfg(target_os = "linux")]
fn linux_ports() -> Vec<(String, String)> {
    let cards = std::fs::read_to_string("/proc/asound/cards").unwrap_or_default();
    // " 1 [MK3            ]: USB-Audio - Maschine MK3": the number, and the long name after the dash
    let name_of = |card: &str| {
        cards
            .lines()
            .find(|l| l.split_whitespace().next() == Some(card))
            .and_then(|l| l.split(" - ").nth(1))
            .map(|n| n.trim().to_string())
    };
    let mut nodes: Vec<String> = std::fs::read_dir("/dev/snd")
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("midiC")).collect())
        .unwrap_or_default();
    nodes.sort();
    nodes
        .into_iter()
        .map(|node| {
            let card = node.trim_start_matches("midiC").split('D').next().unwrap_or("").to_string();
            let label = match name_of(&card) {
                Some(n) => format!("{n} · {node}"),
                None => node.clone(),
            };
            (label, format!("/dev/snd/{node}"))
        })
        .collect()
}

#[cfg(target_os = "linux")]
pub fn open_port(want: Option<&str>) -> anyhow::Result<Box<dyn MidiPort>> {
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
    let want = std::env::var("OP8_MIDI_PORT").ok().or_else(|| want.map(str::to_string));
    let ports = linux_ports();
    let (label, path) = match want {
        // a device node, or part of the name the player saw
        Some(w) if w.starts_with("/dev/") => (w.clone(), w),
        Some(w) => ports.into_iter().find(|(l, _)| l.contains(&w)).with_context(|| format!("no MIDI port matching {w:?}"))?,
        None => ports.into_iter().find(|(l, _)| l.contains("Maschine")).context("no Maschine sound card (choose a port in SETTINGS or set OP8_MIDI_PORT)")?,
    };
    let f = std::fs::OpenOptions::new().write(true).open(&path).with_context(|| format!("opening {path}"))?;
    Ok(Box::new(Raw(f, label)))
}

#[cfg(all(target_os = "macos", feature = "midi"))]
pub fn list_ports() -> Vec<String> {
    midir::MidiOutput::new("OP-8").map(|o| o.ports().iter().filter_map(|p| o.port_name(p).ok()).collect()).unwrap_or_default()
}

#[cfg(all(target_os = "macos", feature = "midi"))]
pub fn open_port(want: Option<&str>) -> anyhow::Result<Box<dyn MidiPort>> {
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
    let want = std::env::var("OP8_MIDI_PORT").ok().or_else(|| want.map(str::to_string)).unwrap_or_else(|| "Maschine".into());
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
pub fn list_ports() -> Vec<String> {
    Vec::new()
}

#[cfg(not(any(target_os = "linux", all(target_os = "macos", feature = "midi"))))]
pub fn open_port(_: Option<&str>) -> anyhow::Result<Box<dyn MidiPort>> {
    anyhow::bail!("no MIDI backend on this platform/feature set")
}

/// The port that is opened when the player has chosen none: the MK3's own.
pub fn open_default() -> anyhow::Result<Box<dyn MidiPort>> {
    open_port(None)
}

/// What the thread that writes to the port can be told to do while it runs.
enum Command {
    /// Use this port from now on (the old one is silenced first).
    Switch(Box<dyn MidiPort>),
    /// Play a short note on channel 1, to hear whether what is plugged in answers.
    Test,
    /// Silence everything: all notes and all sound off on the sixteen channels.
    Panic,
}

/// The way to the thread that writes to the port, for SETTINGS.
#[derive(Clone)]
pub struct Link {
    tx: std::sync::mpsc::Sender<Command>,
    name: Arc<std::sync::Mutex<String>>,
}

impl Link {
    pub fn switch(&self, port: Box<dyn MidiPort>) {
        let _ = self.tx.send(Command::Switch(port));
    }
    pub fn test(&self) {
        let _ = self.tx.send(Command::Test);
    }
    pub fn panic(&self) {
        let _ = self.tx.send(Command::Panic);
    }
    /// The port that is open.
    pub fn name(&self) -> String {
        self.name.lock().map(|n| n.clone()).unwrap_or_default()
    }
}

/// The messages that silence a port: all sound off and all notes off, on every channel.
pub fn panic_messages() -> Vec<[u8; 3]> {
    (0..16u8).flat_map(|ch| [[0xB0 | ch, 120, 0], [0xB0 | ch, 123, 0]]).collect()
}

/// Drain `rx` into `port` until `stop` is set.
pub fn spawn(rx: rtrb::Consumer<MidiMessage>, port: Box<dyn MidiPort>, stop: Arc<AtomicBool>) -> thread::JoinHandle<()> {
    spawn_linked(rx, port, stop).0
}

/// Like [`spawn`], and the [`Link`] through which SETTINGS can switch the port, play a test note and silence it.
pub fn spawn_linked(mut rx: rtrb::Consumer<MidiMessage>, port: Box<dyn MidiPort>, stop: Arc<AtomicBool>) -> (thread::JoinHandle<()>, Link) {
    let (tx, cmds) = std::sync::mpsc::channel();
    let name = Arc::new(std::sync::Mutex::new(port.name()));
    let link = Link { tx, name: name.clone() };
    let mut port = port;
    let handle = thread::Builder::new()
        .name("op8-midi".into())
        .spawn(move || {
            let mut note_off_at: Option<std::time::Instant> = None;
            while !stop.load(Ordering::Relaxed) {
                while let Ok(c) = cmds.try_recv() {
                    match c {
                        Command::Switch(next) => {
                            for m in panic_messages() {
                                let _ = port.send(&m);
                            }
                            if let Ok(mut n) = name.lock() {
                                *n = next.name();
                            }
                            port = next;
                        }
                        Command::Test => {
                            let _ = port.send(&[0x90, 60, 100]);
                            note_off_at = Some(std::time::Instant::now() + Duration::from_millis(300));
                        }
                        Command::Panic => {
                            for m in panic_messages() {
                                let _ = port.send(&m);
                            }
                        }
                    }
                }
                if note_off_at.is_some_and(|t| std::time::Instant::now() >= t) {
                    let _ = port.send(&[0x80, 60, 0]);
                    note_off_at = None;
                }
                while let Ok(m) = rx.pop() {
                    if let Err(e) = port.send(m.as_bytes()) {
                        eprintln!("midi: {e}");
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        })
        .expect("spawn midi thread");
    (handle, link)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct Log(Arc<Mutex<Vec<Vec<u8>>>>, &'static str);
    impl MidiPort for Log {
        fn send(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
            self.0.lock().unwrap().push(bytes.to_vec());
            Ok(())
        }
        fn name(&self) -> String {
            self.1.to_string()
        }
    }

    fn wait(log: &Log, want: impl Fn(&[Vec<u8>]) -> bool) -> bool {
        for _ in 0..400 {
            if want(&log.0.lock().unwrap()) {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    // F-C15: the thread that writes to the port can be told to play a test note (and let go of it), to silence the port, and to switch to
    // another one, which it silences first; and it says which port is open.
    #[test]
    fn the_port_can_be_tested_silenced_and_switched_while_it_runs() {
        let (_tx, rx) = rtrb::RingBuffer::<MidiMessage>::new(8);
        let (first, second) = (Log(Default::default(), "first"), Log(Default::default(), "second"));
        let stop = Arc::new(AtomicBool::new(false));
        let (handle, link) = spawn_linked(rx, Box::new(first.clone()), stop.clone());
        assert_eq!(link.name(), "first");
        link.test();
        assert!(wait(&first, |m| m.contains(&vec![0x90, 60, 100])), "the test note starts");
        assert!(wait(&first, |m| m.contains(&vec![0x80, 60, 0])), "and ends by itself");
        link.panic();
        assert!(wait(&first, |m| m.iter().filter(|b| b[1] == 123).count() == 16), "all notes off on the sixteen channels");
        link.switch(Box::new(second.clone()));
        assert!(wait(&first, |m| m.iter().filter(|b| b[1] == 120).count() >= 32), "the old port is silenced before it is left");
        assert!(wait(&second, |_| link.name() == "second"));
        link.test();
        assert!(wait(&second, |m| m.contains(&vec![0x90, 60, 100])), "the new port is the one that plays");
        stop.store(true, Ordering::Relaxed);
        handle.join().unwrap();
    }
}
