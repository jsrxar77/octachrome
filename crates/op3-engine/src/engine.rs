use crate::{
    fx::{FxRack, PunchFx},
    status::EngineStatus,
};
use op3_cartridge::{InstrumentCartridge, MAX_BLOCK, SAMPLE_RATE};
use op3_seq::{BlockEvents, Command, Project, Sequencer, TRACKS};
use std::sync::{atomic::Ordering::Relaxed, Arc, Mutex};
use std::time::Instant;

/// A cartridge shared between the audio thread (`process`) and the UI thread
/// (`draw_ui`, `info`). The audio thread holds the lock for one block; the UI
/// only ever `try_lock`s, so it can never stall audio for long.
pub type SharedCartridge = Arc<Mutex<Box<dyn InstrumentCartridge>>>;

pub const PARAMS_PER_SLOT: usize = 8;

/// Messages from the control thread to the audio thread.
pub enum EngineCommand {
    Seq(Command),
    Play,
    Stop,
    SetParam {
        track: u8,
        index: u8,
        value: f32,
    },
    SetVolume {
        track: u8,
        volume: f32,
    },
    /// Engage/release a Punch-in FX with intensity 0..1.
    Punch(Option<PunchFx>, f32),
    PunchIntensity(f32),
    /// Smart Strip: -1..0 low-pass sweep, 0 off, 0..1 high-pass sweep.
    MasterFilter(f32),
    /// Put a cartridge in a slot. The old one comes back through the
    /// handle's garbage queue so it is never freed on the audio thread.
    LoadCartridge {
        track: u8,
        cartridge: Option<SharedCartridge>,
    },
}

/// Lives on the audio thread.
pub struct Engine {
    seq: Sequencer,
    slots: [Option<SharedCartridge>; TRACKS],
    rx: rtrb::Consumer<EngineCommand>,
    garbage: rtrb::Producer<SharedCartridge>,
    status: Arc<EngineStatus>,
    events: BlockEvents,
    track_l: Box<[f32]>,
    track_r: Box<[f32]>,
    volumes: [f32; TRACKS],
    base_params: [[f32; PARAMS_PER_SLOT]; TRACKS],
    fx: FxRack,
    peaks: [f32; TRACKS],
    note_counts: [u32; TRACKS],
}

/// Lives on the control/UI thread.
pub struct EngineHandle {
    tx: rtrb::Producer<EngineCommand>,
    garbage: rtrb::Consumer<SharedCartridge>,
    /// Mirror of the engine's project, for drawing.
    pub project: Project,
    pub status: Arc<EngineStatus>,
    pub slots: [Option<SharedCartridge>; TRACKS],
    params: [[f32; PARAMS_PER_SLOT]; TRACKS],
    playing: bool,
}

impl Engine {
    pub fn new(
        project: Project,
        slots: [Option<SharedCartridge>; TRACKS],
    ) -> (Engine, EngineHandle) {
        let (tx, rx) = rtrb::RingBuffer::new(1024);
        let (gtx, grx) = rtrb::RingBuffer::new(64);
        let status = Arc::new(EngineStatus::default());
        let mut params = [[0.5; PARAMS_PER_SLOT]; TRACKS];
        for (t, slot) in slots.iter().enumerate() {
            if let Some(c) = slot {
                params[t] = initial_params(&**c.lock().unwrap());
            }
        }
        let engine = Engine {
            seq: Sequencer::new(project, SAMPLE_RATE),
            slots: slots.clone(),
            rx,
            garbage: gtx,
            status: status.clone(),
            events: BlockEvents::default(),
            track_l: vec![0.0; MAX_BLOCK].into_boxed_slice(),
            track_r: vec![0.0; MAX_BLOCK].into_boxed_slice(),
            volumes: [0.8; TRACKS],
            base_params: params,
            fx: FxRack::new(),
            peaks: [0.0; TRACKS],
            note_counts: [0; TRACKS],
        };
        let handle = EngineHandle {
            tx,
            garbage: grx,
            project,
            status,
            slots,
            params,
            playing: false,
        };
        (engine, handle)
    }

    pub fn sequencer(&self) -> &Sequencer {
        &self.seq
    }

    fn handle_commands(&mut self) {
        while let Ok(cmd) = self.rx.pop() {
            match cmd {
                EngineCommand::Seq(c) => self.seq.apply(&c),
                EngineCommand::Play => self.seq.play(),
                EngineCommand::Stop => {
                    self.seq.stop();
                    self.fx.engage(None, 0.0);
                    self.seq.set_repeat(None);
                }
                EngineCommand::SetParam {
                    track,
                    index,
                    value,
                } => {
                    if let Some(p) = self
                        .base_params
                        .get_mut(track as usize)
                        .and_then(|p| p.get_mut(index as usize))
                    {
                        *p = value;
                    }
                    if let Some(Some(c)) = self.slots.get(track as usize) {
                        if let Ok(mut c) = c.lock() {
                            c.set_parameter(index as usize, value);
                        }
                    }
                }
                EngineCommand::SetVolume { track, volume } => {
                    if let Some(v) = self.volumes.get_mut(track as usize) {
                        *v = volume.clamp(0.0, 1.0);
                    }
                }
                EngineCommand::Punch(fx, intensity) => match fx {
                    Some(PunchFx::Repeat(ticks)) => {
                        self.seq.set_repeat(Some(ticks));
                        self.fx.engage(None, 0.0);
                    }
                    other => {
                        self.seq.set_repeat(None);
                        self.fx.engage(other, intensity);
                    }
                },
                EngineCommand::PunchIntensity(v) => self.fx.set_intensity(v),
                EngineCommand::MasterFilter(v) => self.fx.strip = v.clamp(-1.0, 1.0),
                EngineCommand::LoadCartridge { track, cartridge } => {
                    if let Some(slot) = self.slots.get_mut(track as usize) {
                        if let Some(c) = &cartridge {
                            if let Ok(c) = c.lock() {
                                self.base_params[track as usize] = initial_params(&**c);
                            }
                        }
                        if let Some(old) = std::mem::replace(slot, cartridge) {
                            // If the garbage queue is full the Arc is leaked
                            // rather than freed here; it holds at most a few KB.
                            if let Err(rtrb::PushError::Full(old)) = self.garbage.push(old) {
                                std::mem::forget(old);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Render any number of frames into two mono slices (which may be
    /// interleaved by the caller). Never allocates.
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let started = Instant::now();
        let total = out_l.len().min(out_r.len());
        self.handle_commands();
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_BLOCK);
            self.render_block(&mut out_l[done..done + n], &mut out_r[done..done + n]);
            done += n;
        }
        self.publish(started, total);
    }

    fn render_block(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len();
        self.seq.process(n, &mut self.events);
        out_l.fill(0.0);
        out_r.fill(0.0);

        for t in 0..TRACKS {
            let Some(slot) = &self.slots[t] else { continue };
            let Ok(mut c) = slot.lock() else { continue };
            // Parameter locks take effect at the block start (≤ 2.7 ms early
            // at 128 frames). Sample-accurate locks are a TODO.
            for p in self.events.params.iter().filter(|p| p.track as usize == t) {
                let v = p
                    .value
                    .unwrap_or(self.base_params[t][p.index as usize % PARAMS_PER_SLOT]);
                c.set_parameter(p.index as usize, v);
            }
            let evs = self.events.notes[t].as_slice();
            self.note_counts[t] = self.note_counts[t].wrapping_add(
                evs.iter()
                    .filter(|e| e.kind == op3_cartridge::NoteKind::On)
                    .count() as u32,
            );
            let (l, r) = (&mut self.track_l[..n], &mut self.track_r[..n]);
            c.process(l, r, evs);
            drop(c);
            let vol = self.volumes[t];
            let mut peak = self.peaks[t] * 0.9;
            for i in 0..n {
                let (a, b) = (l[i] * vol, r[i] * vol);
                out_l[i] += a;
                out_r[i] += b;
                peak = peak.max(a.abs()).max(b.abs());
            }
            self.peaks[t] = peak;
        }

        let spt = self.seq.samples_per_tick() * 24.0;
        self.fx.process(out_l, out_r, spt);
        for s in out_l.iter_mut().chain(out_r.iter_mut()) {
            *s = soft_clip(*s);
        }
    }

    fn publish(&self, started: Instant, frames: usize) {
        let s = &self.status;
        s.playing.store(self.seq.is_playing(), Relaxed);
        for t in 0..TRACKS {
            s.playhead[t].store(self.seq.playhead(t).step, Relaxed);
            s.note_count[t].store(self.note_counts[t], Relaxed);
            s.peak[t].store(self.peaks[t].to_bits(), Relaxed);
        }
        let chord = self.seq.current_chord();
        s.chord_root.store(chord.root, Relaxed);
        s.chord_mask.store(chord.mask as u32, Relaxed);
        s.chord_slot
            .store(self.seq.current_chord_slot() as u8, Relaxed);
        s.dropped_events.store(self.seq.dropped_events(), Relaxed);
        if frames > 0 {
            let budget = frames as f32 / SAMPLE_RATE;
            s.load.store(
                (started.elapsed().as_secs_f32() / budget).to_bits(),
                Relaxed,
            );
        }
    }
}

fn initial_params(c: &dyn InstrumentCartridge) -> [f32; PARAMS_PER_SLOT] {
    let mut v = [0.5; PARAMS_PER_SLOT];
    let secondary = c.secondary_parameters();
    for (i, slot) in v.iter_mut().enumerate() {
        let default = if i < 4 {
            c.parameters()[i].default
        } else {
            secondary.map_or(0.5, |s| s[i - 4].default)
        };
        *slot = c.parameter_value(i).unwrap_or(default);
    }
    v
}

#[inline]
fn soft_clip(x: f32) -> f32 {
    if x.abs() < 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.2 * ((x.abs() - 0.8) / 0.2).tanh())
    }
}

impl EngineHandle {
    /// Send to the audio thread, mirroring project edits locally.
    /// Returns false when the queue is full (the edit is dropped on both
    /// sides so they stay in sync).
    pub fn send(&mut self, cmd: EngineCommand) -> bool {
        let mirror = match &cmd {
            EngineCommand::Seq(c) => Some(*c),
            _ => None,
        };
        let param = match &cmd {
            EngineCommand::SetParam {
                track,
                index,
                value,
            } => Some((*track, *index, *value)),
            _ => None,
        };
        let play = match &cmd {
            EngineCommand::Play => Some(true),
            EngineCommand::Stop => Some(false),
            _ => None,
        };
        if self.tx.push(cmd).is_err() {
            return false;
        }
        if let Some(c) = mirror {
            self.project.apply(&c);
        }
        if let Some((t, i, v)) = param {
            if let Some(p) = self
                .params
                .get_mut(t as usize)
                .and_then(|p| p.get_mut(i as usize))
            {
                *p = v;
            }
        }
        if let Some(p) = play {
            self.playing = p;
        }
        true
    }

    pub fn seq(&mut self, c: Command) -> bool {
        self.send(EngineCommand::Seq(c))
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn toggle_play(&mut self) {
        let cmd = if self.playing {
            EngineCommand::Stop
        } else {
            EngineCommand::Play
        };
        self.send(cmd);
    }

    /// Knob value the host last set (or the cartridge's own default).
    pub fn param(&self, track: usize, index: usize) -> f32 {
        self.params[track][index % PARAMS_PER_SLOT]
    }

    /// Nudge a parameter by `delta` (normalised) and send it.
    pub fn nudge_param(&mut self, track: usize, index: usize, delta: f32) {
        let v = (self.param(track, index) + delta).clamp(0.0, 1.0);
        self.send(EngineCommand::SetParam {
            track: track as u8,
            index: index as u8,
            value: v,
        });
    }

    pub fn load_cartridge(&mut self, track: usize, c: Box<dyn InstrumentCartridge>) {
        let shared: SharedCartridge = Arc::new(Mutex::new(c));
        self.params[track] = initial_params(&**shared.lock().unwrap());
        self.slots[track] = Some(shared.clone());
        self.send(EngineCommand::LoadCartridge {
            track: track as u8,
            cartridge: Some(shared),
        });
    }

    /// Drop cartridges the audio thread has finished with. Call regularly.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op3_cartridges::{FmBass, KickSyn};
    use op3_seq::Step;

    fn shared(c: impl InstrumentCartridge + 'static) -> Option<SharedCartridge> {
        Some(Arc::new(Mutex::new(Box::new(c))))
    }

    fn rig() -> (Engine, EngineHandle) {
        let mut slots: [Option<SharedCartridge>; TRACKS] = Default::default();
        slots[0] = shared(KickSyn::new());
        slots[1] = shared(FmBass::new());
        Engine::new(Project::default(), slots)
    }

    fn render(e: &mut Engine, frames: usize) -> Vec<f32> {
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        for (cl, cr) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            e.render(cl, cr);
        }
        l
    }

    #[test]
    fn silent_until_steps_and_play() {
        let (mut e, mut h) = rig();
        assert!(render(&mut e, 4800).iter().all(|&s| s == 0.0));
        h.seq(Command::SetStep {
            track: 0,
            step: 0,
            value: Step::on(36),
        });
        h.send(EngineCommand::Play);
        let out = render(&mut e, 4800);
        assert!(out.iter().any(|&s| s.abs() > 0.1));
        assert!(e.status.playing.load(Relaxed));
        assert!(e.status.note_count[0].load(Relaxed) >= 1);
    }

    #[test]
    fn mirror_matches_engine_project() {
        let (mut e, mut h) = rig();
        h.seq(Command::ToggleStep { track: 3, step: 7 });
        h.seq(Command::SetLength {
            track: 3,
            length: 7,
        });
        h.seq(Command::SetBpm(91.0));
        render(&mut e, 128);
        assert_eq!(h.project, e.sequencer().project);
    }

    #[test]
    fn same_input_same_output() {
        let go = || {
            let (mut e, mut h) = rig();
            for s in [0, 4, 8, 12] {
                h.seq(Command::SetStep {
                    track: 0,
                    step: s,
                    value: Step::on(36),
                });
            }
            h.seq(Command::SetStep {
                track: 1,
                step: 2,
                value: Step::on(40),
            });
            h.seq(Command::SetBrainLink {
                track: 1,
                linked: true,
            });
            h.send(EngineCommand::Play);
            render(&mut e, 48_000)
        };
        assert_eq!(go(), go());
    }

    #[test]
    fn kill_fx_and_strip_filter() {
        let (mut e, mut h) = rig();
        h.seq(Command::SetStep {
            track: 0,
            step: 0,
            value: Step::on(36),
        });
        h.send(EngineCommand::Play);
        h.send(EngineCommand::Punch(Some(PunchFx::Kill), 1.0));
        assert!(render(&mut e, 4800).iter().all(|&s| s == 0.0));
        h.send(EngineCommand::Punch(None, 0.0));
        h.send(EngineCommand::MasterFilter(-0.5));
        h.send(EngineCommand::Stop);
        h.send(EngineCommand::Play);
        assert!(render(&mut e, 4800).iter().any(|&s| s.abs() > 0.05));
    }

    #[test]
    fn swapping_cartridges_returns_the_old_one() {
        let (mut e, mut h) = rig();
        h.load_cartridge(0, Box::new(FmBass::new()));
        render(&mut e, 64);
        let n = std::iter::from_fn(|| h.garbage.pop().ok()).count();
        assert_eq!(n, 1);
        assert_eq!(
            e.slots[0].as_ref().unwrap().lock().unwrap().info().name,
            "FmBass"
        );
    }

    #[test]
    fn params_track_knob_moves() {
        let (_e, mut h) = rig();
        let before = h.param(0, 1);
        h.nudge_param(0, 1, 0.1);
        assert!((h.param(0, 1) - before - 0.1).abs() < 1e-6);
    }
}
