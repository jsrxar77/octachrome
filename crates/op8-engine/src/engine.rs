use crate::{
    fx::{FxRack, PunchFx},
    status::EngineStatus,
    track_fx::TrackFxProc,
};
use op8_cartridge::{InstrumentCartridge, MAX_BLOCK, SAMPLE_RATE};
use op8_seq::{BlockEvents, Command, Project, Sequencer, TRACKS};
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
    /// Start from a tick (96 to the quarter note) instead of the top: what a song position pointer from outside asks for.
    PlayFrom(u64),
    Stop,
    SetParam {
        track: u8,
        index: u8,
        value: f32,
    },
    /// Engage/release a Punch-in FX with intensity 0..1.
    Punch(Option<PunchFx>, f32),
    PunchIntensity(f32),
    /// Smart Strip: -1..0 low-pass sweep, 0 off, 0..1 high-pass sweep.
    MasterFilter(f32),
    /// Play a note on a track's instrument right now, outside the sequence, to
    /// hear what a choice sounds like. It is released after a fraction of a second.
    Audition {
        track: u8,
        note: u8,
        velocity: f32,
    },
    /// A note played by hand on a track's instrument (a pad in PAD MODE or KEYBOARD): started on
    /// `on`, released on `!on`. Joins the sequence's notes at the next block.
    LiveNote {
        track: u8,
        note: u8,
        velocity: f32,
        on: bool,
    },
    /// The pitch wheel of a keyboard on a track's instrument: bend by `semitones` (0 is centre). An instrument with no pitch ignores it.
    PitchBend { track: u8, semitones: f32 },
    /// The master volume, 0.0..=1.0 (VOLUME + the 4D encoder).
    MasterVolume(f32),
    /// The metronome clicks on every beat while the sequencer runs (SHIFT + TAP).
    Metronome(bool),
    /// Replace the whole project (FILE). Boxed: it is large, and the queue holds many commands.
    LoadProject(Box<op8_seq::Project>),
    /// Put a cartridge in a slot. The old one comes back through the
    /// handle's garbage queue so it is never freed on the audio thread.
    LoadCartridge {
        track: u8,
        cartridge: Option<SharedCartridge>,
    },
    /// SAMPLING: start recording from a source (what was in `take` is overwritten), or stop with `None`.
    Record(Option<RecSource>),
    /// SAMPLING, armed: wait for a sound louder than the threshold (0..1) and start recording from just before it.
    RecordWhenLoud(RecSource, f32),
    /// The audio that comes in (mono, 48 kHz) for `RecSource::Input`. Boxed: the ring is large.
    SetInput(Box<rtrb::Consumer<f32>>),
    /// SAMPLING: play `start..end` of the take (in samples) to hear it; `None` stops it.
    Preview(Option<(u32, u32)>),
    /// MIDI clock out: start, 24 clocks to the quarter note and stop go to the MIDI thread, or not.
    MidiClock(bool),
    /// One click of the count-in before a recording (a higher one when `true`), whether the sequencer runs or not.
    Click(bool),
    /// SAMPLING: move the part that is being listened to while it plays (nothing if it is not playing): the end moves, and the
    /// playing position follows the start if it falls behind it.
    PreviewWindow(u32, u32),
}

/// Where a recording comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecSource {
    /// The audio interface's input (a line or a microphone).
    Input,
    /// What OP-8 itself is playing (the master).
    Resample,
}

/// The longest recording: ten seconds, like the longest sample.
pub const MAX_RECORD: usize = SAMPLE_RATE as usize * 10;
/// How much before the sound that starts an armed recording is kept: 10 ms.
const PREROLL: usize = 480;

/// How long an auditioned note is held before it is released.
const AUDITION_SAMPLES: i32 = (SAMPLE_RATE * 0.25) as i32;

/// A note being auditioned on a track.
#[derive(Clone, Copy)]
struct Audition {
    note: u8,
    velocity: f32,
    started: bool,
    /// Samples left before the release.
    left: i32,
}

/// The state of the LFO of one track.
#[derive(Clone, Copy)]
struct LfoState {
    phase: f32,
    /// The value a Random LFO holds, -1..=1.
    held: f32,
    rng: u32,
    /// The knob it moved last block, so that it can be put back when the LFO moves elsewhere or stops.
    driving: Option<usize>,
    /// What it last wrote to that knob.
    written: f32,
}

impl Default for LfoState {
    fn default() -> Self {
        Self { phase: 0.0, held: 0.0, rng: 0x1234_5679, driving: None, written: -1.0 }
    }
}

impl LfoState {
    /// Move on by `samples`; the LFO's value now, -1..=1.
    fn advance(&mut self, lfo: &op8_seq::Lfo, samples: usize) -> f32 {
        let before = self.phase;
        self.phase = (self.phase + lfo.hz() * samples as f32 / SAMPLE_RATE).fract();
        if self.phase < before {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 17;
            self.rng ^= self.rng << 5;
            self.held = (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        }
        match lfo.shape {
            op8_seq::LfoShape::Random => self.held,
            s => s.at(self.phase),
        }
    }
}

/// Lives on the audio thread.
pub struct Engine {
    track_fx: Vec<TrackFxProc>,
    lfo: [LfoState; TRACKS],
    master: f32,
    metronome: bool,
    /// The click that is sounding: samples left, its phase and its frequency.
    click_left: u32,
    click_phase: f32,
    click_hz: f32,
    last_beat: u64,
    audition: [Option<Audition>; TRACKS],
    /// Notes played by hand, waiting for the next block.
    live: [op8_seq::fixed::FixedVec<op8_cartridge::NoteEvent, 16>; TRACKS],
    seq: Sequencer,
    slots: [Option<SharedCartridge>; TRACKS],
    rx: rtrb::Consumer<EngineCommand>,
    garbage: rtrb::Producer<SharedCartridge>,
    status: Arc<EngineStatus>,
    events: BlockEvents,
    track_l: Box<[f32]>,
    track_r: Box<[f32]>,
    base_params: [[f32; PARAMS_PER_SLOT]; TRACKS],
    fx: FxRack,
    peaks: [f32; TRACKS],
    note_counts: [u32; TRACKS],
    rec: Option<RecSource>,
    /// Armed: the source to record and the level (0..1) that starts it; the last few milliseconds are kept to start just before it.
    armed: Option<(RecSource, f32)>,
    preroll: Vec<f32>,
    preroll_at: usize,
    preview: Option<(usize, usize)>,
    /// MIDI clock out: on or off, the way to the MIDI thread, and the sample time of the block.
    comp: crate::comp::Compressor,
    eq: crate::eq::Eq,
    sends: crate::sends::Sends,
    /// What the tracks send to the delay and to the reverb in this block.
    /// The sound of the track the compressor listens to (its side-chain), after its level and pan.
    side: [Box<[f32]>; 2],
    bus_a: [Box<[f32]>; 2],
    bus_b: [Box<[f32]>; 2],
    midi_clock: bool,
    sync_tx: rtrb::Producer<SyncMessage>,
    sync_time: u64,
    input: Option<rtrb::Consumer<f32>>,
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
    pub master: f32,
    pub metronome: bool,
    /// The MIDI real-time messages (start, clock, stop) for the MIDI thread to take, once.
    sync_rx: Option<rtrb::Consumer<SyncMessage>>,
}

impl EngineHandle {
    /// The end of the queue of MIDI real-time messages; the MIDI thread (or the test bench) takes it once.
    pub fn take_midi_sync(&mut self) -> Option<rtrb::Consumer<SyncMessage>> {
        self.sync_rx.take()
    }
}

/// A MIDI real-time message (0xF8 clock, 0xFA start, 0xFC stop) with the sample time it falls at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncMessage {
    pub byte: u8,
    pub sample_time: u64,
}

impl Engine {
    pub fn new(
        project: Project,
        slots: [Option<SharedCartridge>; TRACKS],
    ) -> (Engine, EngineHandle) {
        let (tx, rx) = rtrb::RingBuffer::new(1024);
        let (gtx, grx) = rtrb::RingBuffer::new(64);
        let (sync_tx, sync_rx) = rtrb::RingBuffer::new(256);
        let status = Arc::new(EngineStatus::default());
        status.take.lock().unwrap().resize(MAX_RECORD, 0.0);
        let mut params = [[0.5; PARAMS_PER_SLOT]; TRACKS];
        for (t, slot) in slots.iter().enumerate() {
            if let Some(c) = slot {
                params[t] = initial_params(&**c.lock().unwrap());
            }
        }
        let engine = Engine {
            track_fx: (0..TRACKS).map(|_| TrackFxProc::new()).collect(),
            lfo: Default::default(),
            master: 1.0,
            metronome: false,
            click_left: 0,
            click_phase: 0.0,
            click_hz: 1000.0,
            last_beat: 0,
            audition: [None; TRACKS],
            live: Default::default(),
            seq: Sequencer::new(project.clone(), SAMPLE_RATE),
            slots: slots.clone(),
            rx,
            garbage: gtx,
            status: status.clone(),
            events: BlockEvents::default(),
            track_l: vec![0.0; MAX_BLOCK].into_boxed_slice(),
            track_r: vec![0.0; MAX_BLOCK].into_boxed_slice(),
            base_params: params,
            fx: FxRack::new(),
            peaks: [0.0; TRACKS],
            note_counts: [0; TRACKS],
            rec: None,
            armed: None,
            preroll: vec![0.0; PREROLL],
            preroll_at: 0,
            preview: None,
            comp: crate::comp::Compressor::new(SAMPLE_RATE),
            eq: crate::eq::Eq::new(SAMPLE_RATE),
            sends: crate::sends::Sends::new(SAMPLE_RATE),
            side: [vec![0.0; MAX_BLOCK].into_boxed_slice(), vec![0.0; MAX_BLOCK].into_boxed_slice()],
            bus_a: [vec![0.0; MAX_BLOCK].into_boxed_slice(), vec![0.0; MAX_BLOCK].into_boxed_slice()],
            bus_b: [vec![0.0; MAX_BLOCK].into_boxed_slice(), vec![0.0; MAX_BLOCK].into_boxed_slice()],
            midi_clock: true,
            sync_tx,
            sync_time: 0,
            input: None,
        };
        let handle = EngineHandle {
            tx,
            garbage: grx,
            project,
            status,
            slots,
            params,
            playing: false,
            master: 1.0,
            metronome: false,
            sync_rx: Some(sync_rx),
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
                EngineCommand::PlayFrom(tick) => self.seq.play_from(tick),
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
                EngineCommand::LiveNote { track, note, velocity, on } => {
                    if let Some(q) = self.live.get_mut(track as usize) {
                        let ev = if on { op8_cartridge::NoteEvent::on(0, note, velocity.clamp(0.0, 1.0)) } else { op8_cartridge::NoteEvent::off(0, note) };
                        q.push(ev);
                    }
                }
                EngineCommand::PitchBend { track, semitones } => {
                    if let Some(Ok(mut c)) = self.slots.get(track as usize).and_then(|s| s.as_ref()).map(|s| s.lock()) {
                        c.set_pitch_bend(semitones);
                    }
                }
                EngineCommand::MasterVolume(v) => self.master = v.clamp(0.0, 1.0),
                EngineCommand::Metronome(on) => {
                    self.metronome = on;
                    self.click_left = 0;
                }
                EngineCommand::LoadProject(p) => {
                    self.seq.load_project(*p);
                    self.audition = [None; TRACKS];
                }
                EngineCommand::Audition { track, note, velocity } => {
                    if let Some(a) = self.audition.get_mut(track as usize) {
                        *a = Some(Audition { note, velocity: velocity.clamp(0.0, 1.0), started: false, left: AUDITION_SAMPLES });
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
                EngineCommand::Record(source) => {
                    self.rec = source;
                    self.armed = None;
                    self.status.armed.store(false, Relaxed);
                    if source.is_some() {
                        self.preview = None;
                    }
                    if source.is_some() {
                        self.status.rec_len.store(0, Relaxed);
                        self.status.rec_peak.store(0, Relaxed);
                    }
                    self.status.recording.store(source.is_some(), Relaxed);
                }
                EngineCommand::RecordWhenLoud(source, threshold) => {
                    self.rec = None;
                    self.armed = Some((source, threshold.clamp(0.0005, 1.0)));
                    self.preroll_at = 0;
                    self.preroll.iter_mut().for_each(|x| *x = 0.0);
                    self.preview = None;
                    self.status.rec_len.store(0, Relaxed);
                    self.status.rec_peak.store(0, Relaxed);
                    self.status.recording.store(false, Relaxed);
                    self.status.armed.store(true, Relaxed);
                }
                EngineCommand::SetInput(rx) => self.input = Some(*rx),
                EngineCommand::PreviewWindow(a, b) => {
                    if let Some((pos, _)) = self.preview {
                        let (a, b) = (a as usize, (b as usize).min(MAX_RECORD));
                        self.preview = (pos.max(a) < b).then_some((pos.max(a), b));
                    }
                }
                EngineCommand::MidiClock(on) => {
                    if !on && self.midi_clock && self.seq.is_playing() {
                        // what is following the clock is told to stop, since no more clocks will come
                        let _ = self.sync_tx.push(SyncMessage { byte: 0xFC, sample_time: self.sync_time });
                    }
                    self.midi_clock = on;
                }
                EngineCommand::Click(high) => {
                    self.click_left = (SAMPLE_RATE * 0.03) as u32;
                    self.click_phase = 0.0;
                    self.click_hz = if high { 1500.0 } else { 1000.0 };
                }
                EngineCommand::Preview(r) => self.preview = r.map(|(a, b)| (a as usize, (b as usize).min(MAX_RECORD))),
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
        for e in self.events.sync.iter() {
            if self.midi_clock {
                let byte = match e.kind {
                    op8_seq::SyncKind::Start => 0xFA,
                    op8_seq::SyncKind::Clock => 0xF8,
                    op8_seq::SyncKind::Stop => 0xFC,
                };
                // a full queue drops a clock, which an external clock tolerates, rather than wait on the audio thread
                let _ = self.sync_tx.push(SyncMessage { byte, sample_time: self.sync_time + e.offset as u64 });
            }
        }
        self.sync_time += n as u64;
        out_l.fill(0.0);
        out_r.fill(0.0);
        let side_track = op8_seq::side_track(self.seq.project.comp.side);
        for b in self.bus_a.iter_mut().chain(self.bus_b.iter_mut()).chain(self.side.iter_mut()) {
            b[..n].fill(0.0);
        }

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
            // The LFO moves a knob of the instrument (or the volume) around where the player left it.
            let lfo = self.seq.project.tracks[t].lfo;
            let mut vol_scale = 1.0;
            let state = &mut self.lfo[t];
            let value = if lfo.is_on() { state.advance(&lfo, n) } else { 0.0 };
            let want = if lfo.is_on() { lfo.param() } else { None };
            if state.driving != want {
                if let Some(old) = state.driving {
                    c.set_parameter(old, self.base_params[t][old]);
                }
                state.driving = want;
                state.written = -1.0;
            }
            if let Some(i) = want {
                let v = (self.base_params[t][i] + value * lfo.depth_f() * 0.5).clamp(0.0, 1.0);
                if (v - state.written).abs() > 1e-4 {
                    c.set_parameter(i, v);
                    state.written = v;
                }
            }
            if lfo.is_on() && lfo.moves_volume() {
                // a tremolo: it dips from full volume, never above it
                vol_scale = 1.0 - lfo.depth_f() * (0.5 - 0.5 * value);
            }
            // Notes played by hand come first in this block, in the order they were played.
            for i in 0..self.live[t].as_slice().len() {
                let ev = self.live[t].as_slice()[i];
                self.events.notes[t].insert(i, ev);
            }
            self.live[t].clear();
            // An auditioned note joins the sequence's events for this block (sorted by offset).
            if let Some(a) = &mut self.audition[t] {
                if !a.started {
                    a.started = true;
                    self.events.notes[t].insert(0, op8_cartridge::NoteEvent::on(0, a.note, a.velocity));
                }
                if a.left <= n as i32 {
                    let off = op8_cartridge::NoteEvent::off(a.left.max(0) as u32, a.note);
                    let at = self.events.notes[t].as_slice().iter().position(|e| e.sample_offset > off.sample_offset).unwrap_or(self.events.notes[t].as_slice().len());
                    self.events.notes[t].insert(at, off);
                    self.audition[t] = None;
                } else {
                    a.left -= n as i32;
                }
            }
            let evs = self.events.notes[t].as_slice();
            self.note_counts[t] = self.note_counts[t].wrapping_add(
                evs.iter()
                    .filter(|e| e.kind == op8_cartridge::NoteKind::On)
                    .count() as u32,
            );
            let (l, r) = (&mut self.track_l[..n], &mut self.track_r[..n]);
            c.process(l, r, evs);
            drop(c);
            self.track_fx[t].process(&self.seq.project.tracks[t].fx, l, r);
            // the mix: the level of the track, and a balance (the centre is unity, the far side fades to nothing at the edge)
            let (level, pan) = (self.seq.project.tracks[t].level, self.seq.project.tracks[t].pan);
            let vol = (level as f32 / 127.0) * vol_scale;
            let bal = ((pan as f32 - 64.0) / 63.0).clamp(-1.0, 1.0);
            let (gl, gr) = ((1.0 - bal.max(0.0)) * vol, (1.0 + bal.min(0.0)) * vol);
            let (sa, sb) = (self.seq.project.tracks[t].send_a as f32 / 127.0, self.seq.project.tracks[t].send_b as f32 / 127.0);
            let (pre_a, pre_b) = (self.seq.project.sends.is_pre(0), self.seq.project.sends.is_pre(1));
            let mut peak = self.peaks[t] * 0.9;
            for i in 0..n {
                let (a, b) = (l[i] * gl, r[i] * gr);
                out_l[i] += a;
                out_r[i] += b;
                // what goes to the sends is taken after the level and the pan, or (pre-fader) before them
                let (xa, xb) = (if pre_a { (l[i], r[i]) } else { (a, b) }, if pre_b { (l[i], r[i]) } else { (a, b) });
                self.bus_a[0][i] += xa.0 * sa;
                self.bus_a[1][i] += xa.1 * sa;
                self.bus_b[0][i] += xb.0 * sb;
                self.bus_b[1][i] += xb.1 * sb;
                if side_track == Some(t) {
                    self.side[0][i] = a;
                    self.side[1][i] = b;
                }
                peak = peak.max(a.abs()).max(b.abs());
            }
            self.peaks[t] = peak;
        }
        // the delay and the reverb give back into the mix, before the performance effects
        let bpm = self.seq.project.bpm;
        let (a_l, a_r, b_l, b_r) = (&self.bus_a[0][..n], &self.bus_a[1][..n], &self.bus_b[0][..n], &self.bus_b[1][..n]);
        self.sends.process((a_l, a_r), (b_l, b_r), (&mut *out_l, &mut *out_r), &self.seq.project.sends, bpm);

        let spt = self.seq.samples_per_tick() * 24.0;
        self.fx.process(out_l, out_r, spt);
        // the EQ and then the compressor of the master sit after the performance effects, before the metronome and the master volume
        self.eq.process(out_l, out_r, &self.seq.project.eq);
        let side = side_track.map(|_| (&self.side[0][..n], &self.side[1][..n]));
        self.comp.process(out_l, out_r, side, &self.seq.project.comp);
        self.status.comp_reduction.store(self.comp.block_gr.to_bits(), Relaxed);
        self.status.comp_input.store(self.comp.block_in_db.to_bits(), Relaxed);
        // The metronome: a short click at the start of every beat while playing, a higher one on the bar.
        if self.metronome && self.seq.is_playing() {
            let beat = self.seq.tick_position() / 96;
            if beat != self.last_beat {
                self.last_beat = beat;
                self.click_left = (SAMPLE_RATE * 0.03) as u32;
                self.click_phase = 0.0;
                self.click_hz = if beat % 4 == 0 { 1500.0 } else { 1000.0 };
            }
        }
        if self.click_left > 0 {
            for i in 0..n {
                if self.click_left == 0 {
                    break;
                }
                let env = self.click_left as f32 / (SAMPLE_RATE * 0.03);
                let s = (self.click_phase * std::f32::consts::TAU).sin() * env * 0.35;
                out_l[i] += s;
                out_r[i] += s;
                self.click_phase = (self.click_phase + self.click_hz / SAMPLE_RATE).fract();
                self.click_left -= 1;
            }
        }
        if let Some((mut pos, end)) = self.preview {
            if let (Ok(take), false) = (self.status.take.try_lock(), self.rec.is_some()) {
                for k in 0..n {
                    if pos >= end {
                        break;
                    }
                    out_l[k] += take[pos] * 0.8;
                    out_r[k] += take[pos] * 0.8;
                    pos += 1;
                }
            }
            self.preview = (pos < end).then_some((pos, end));
        }
        for s in out_l.iter_mut().chain(out_r.iter_mut()) {
            *s = soft_clip(*s * self.master);
        }
        self.record(out_l, out_r);
    }

    /// SAMPLING: copy the source into `take` while recording; keep the input drained otherwise.
    fn record(&mut self, out_l: &[f32], out_r: &[f32]) {
        let Some(source) = self.rec.or(self.armed.map(|a| a.0)) else {
            if let Some(rx) = self.input.as_mut() {
                while rx.pop().is_ok() {}
            }
            return;
        };
        let st = &self.status;
        let mut len = st.rec_len.load(Relaxed) as usize;
        let mut peak = 0.0f32;
        let Ok(mut take) = st.take.try_lock() else { return };
        for k in 0..out_l.len() {
            let s = match source {
                RecSource::Resample => (out_l[k] + out_r[k]) * 0.5,
                RecSource::Input => self.input.as_mut().and_then(|rx| rx.pop().ok()).unwrap_or(0.0),
            };
            let s = if s.is_finite() { s } else { 0.0 };
            if let (None, Some((_, threshold))) = (self.rec, self.armed) {
                // armed: keep the last few milliseconds, and when a sound crosses the threshold start with them
                self.preroll[self.preroll_at] = s;
                self.preroll_at = (self.preroll_at + 1) % PREROLL;
                peak = peak.max(s.abs());
                if s.abs() >= threshold {
                    for i in 0..PREROLL {
                        take[i] = self.preroll[(self.preroll_at + i) % PREROLL];
                    }
                    len = PREROLL;
                    self.rec = Some(source);
                    self.armed = None;
                    st.armed.store(false, Relaxed);
                    st.recording.store(true, Relaxed);
                }
                continue;
            }
            if len < MAX_RECORD {
                take[len] = s;
                len += 1;
            }
            peak = peak.max(s.abs());
        }
        drop(take);
        st.rec_len.store(len as u32, Relaxed);
        st.rec_peak.store(peak.to_bits(), Relaxed);
        if len >= MAX_RECORD {
            self.rec = None;
            st.recording.store(false, Relaxed);
        }
    }

    fn publish(&self, started: Instant, frames: usize) {
        let s = &self.status;
        s.playing.store(self.seq.is_playing(), Relaxed);
        s.tick.store(self.seq.song_tick() as u32, Relaxed);
        for t in 0..TRACKS {
            s.playhead[t].store(self.seq.playhead(t).step, Relaxed);
            s.pattern[t].store(self.seq.playing_patterns()[t], Relaxed);
            s.phase[t].store((self.seq.step_phase(t) * 255.0) as u8, Relaxed);
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
        let loaded = match &cmd {
            EngineCommand::LoadProject(p) => Some((**p).clone()),
            _ => None,
        };
        let master = if let EngineCommand::MasterVolume(v) = &cmd { Some(v.clamp(0.0, 1.0)) } else { None };
        let metro = if let EngineCommand::Metronome(on) = &cmd { Some(*on) } else { None };
        let play = match &cmd {
            EngineCommand::Play | EngineCommand::PlayFrom(_) => Some(true),
            EngineCommand::Stop => Some(false),
            _ => None,
        };
        if self.tx.push(cmd).is_err() {
            return false;
        }
        if let Some(c) = mirror {
            self.project.apply(&c);
        }
        if let Some(p) = loaded {
            self.project = p;
        }
        if let Some(v) = master {
            self.master = v;
        }
        if let Some(on) = metro {
            self.metronome = on;
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
    use op8_cartridges::{FmBass, KickSyn};
    use op8_seq::Step;

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

    fn render_both(e: &mut Engine, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        for (cl, cr) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            e.render(cl, cr);
        }
        (l, r)
    }

    fn playing_kick(level: u8, pan: u8) -> (Engine, EngineHandle) {
        let (e, mut h) = rig();
        h.seq(Command::SetStep { track: 0, step: 0, value: Step::on(36) });
        h.seq(Command::SetLevel { track: 0, level });
        h.seq(Command::SetPan { track: 0, pan });
        h.send(EngineCommand::Play);
        (e, h)
    }

    // F-A20: the mix: a track's level scales it (down to silence, and the default is what the engine always used), and its pan moves it
    // between the speakers: the far side goes quiet at the edge, the centre is the same in both.
    #[test]
    fn f_a20_a_track_has_a_level_and_a_pan() {
        let peak = |v: &[f32]| v.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let (mut e, _h) = playing_kick(102, 64);
        let (l, r) = render_both(&mut e, 4800);
        let full = peak(&l);
        assert!(full > 0.1 && (full - peak(&r)).abs() < 1e-4, "centred: both speakers the same ({full})");
        let (mut e, _h) = playing_kick(51, 64);
        let half = peak(&render_both(&mut e, 4800).0);
        assert!((half / full - 0.5).abs() < 0.05, "half the level is half the sound: {half} vs {full}");
        let (mut e, _h) = playing_kick(0, 64);
        assert!(render_both(&mut e, 4800).0.iter().all(|&s| s == 0.0), "level 0 is silence");
        let (mut e, _h) = playing_kick(102, 0);
        let (l, r) = render_both(&mut e, 4800);
        assert!(peak(&l) > 0.1 && r.iter().all(|&s| s == 0.0), "hard left: nothing on the right");
        let (mut e, _h) = playing_kick(102, 127);
        let (l, r) = render_both(&mut e, 4800);
        assert!(peak(&r) > 0.1 && l.iter().all(|&s| s == 0.0), "hard right: nothing on the left");
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

    /// A cartridge that only records what its knobs were set to.
    struct Probe {
        seen: Arc<Mutex<Vec<(usize, f32)>>>,
    }
    static PROBE_PARAMS: [op8_cartridge::ParamDescriptor; 4] = [
        op8_cartridge::ParamDescriptor::new("a", 0.5, op8_cartridge::ParamUnit::Percent),
        op8_cartridge::ParamDescriptor::new("b", 0.5, op8_cartridge::ParamUnit::Percent),
        op8_cartridge::ParamDescriptor::new("c", 0.5, op8_cartridge::ParamUnit::Percent),
        op8_cartridge::ParamDescriptor::new("d", 0.5, op8_cartridge::ParamUnit::Percent),
    ];
    impl InstrumentCartridge for Probe {
        fn info(&self) -> op8_cartridge::InstrumentInfo {
            op8_cartridge::InstrumentInfo::new("probe", "test")
        }
        fn process(&mut self, l: &mut [f32], r: &mut [f32], _: &[op8_cartridge::NoteEvent]) {
            l.fill(0.0);
            r.fill(0.0);
        }
        fn parameters(&self) -> &[op8_cartridge::ParamDescriptor; 4] {
            &PROBE_PARAMS
        }
        fn set_parameter(&mut self, index: usize, v: f32) {
            self.seen.lock().unwrap().push((index, v));
        }
        fn draw_ui(&self, _: &mut op8_cartridge::FrameBuffer480x272, _: op8_cartridge::Rgb565) {}
        fn save_state(&self) -> Vec<u8> {
            vec![]
        }
        fn load_state(&mut self, _: &[u8]) {}
    }

    // F-A15: an LFO swings a knob of the instrument around where the player left it, and puts it back when it is turned off.
    #[test]
    fn an_lfo_swings_a_knob_and_puts_it_back() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut slots: [Option<SharedCartridge>; TRACKS] = Default::default();
        slots[0] = shared(Probe { seen: seen.clone() });
        let (mut e, mut h) = Engine::new(Project::default(), slots);
        h.seq(Command::SetLfo {
            track: 0,
            lfo: op8_seq::Lfo { shape: op8_seq::LfoShape::Sine, rate: 200, depth: 255, target: 3 }, // knob 2
        });
        render(&mut e, 48_000);
        let writes: Vec<f32> = seen.lock().unwrap().iter().filter(|(i, _)| *i == 1).map(|(_, v)| *v).collect();
        let (lo, hi) = writes.iter().fold((1.0f32, 0.0f32), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(lo < 0.15 && hi > 0.85, "full depth swings the knob across its range: {lo}..{hi}");
        assert!(seen.lock().unwrap().iter().all(|(i, _)| *i == 1), "and touches no other knob");
        // off: the knob goes back to where the player left it
        h.seq(Command::SetLfo { track: 0, lfo: op8_seq::Lfo { target: 0, ..Default::default() } });
        render(&mut e, 1_024);
        assert_eq!(seen.lock().unwrap().last().copied(), Some((1, 0.5)));
        let n = seen.lock().unwrap().len();
        render(&mut e, 4_800);
        assert_eq!(seen.lock().unwrap().len(), n, "and nothing is written any more");
    }

    // F-A15: an LFO on the volume dips the track from full volume and never raises it.
    #[test]
    fn an_lfo_on_the_volume_is_a_tremolo() {
        let (mut e, mut h) = rig();
        for s in 0..16 {
            h.seq(Command::SetStep { track: 0, step: s, value: Step::on(36) });
        }
        h.send(EngineCommand::Play);
        let plain = render(&mut e, 96_000);
        let plain_peak = plain.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        let (mut e, mut h) = rig();
        for s in 0..16 {
            h.seq(Command::SetStep { track: 0, step: s, value: Step::on(36) });
        }
        h.seq(Command::SetLfo { track: 0, lfo: op8_seq::Lfo { shape: op8_seq::LfoShape::Sine, rate: 120, depth: 255, target: 1 } });
        h.send(EngineCommand::Play);
        let out = render(&mut e, 96_000);
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak <= plain_peak * 1.001, "never louder: {peak} vs {plain_peak}");
        let windows: Vec<f32> = out.chunks(6_000).map(|w| w.iter().fold(0.0f32, |m, x| m.max(x.abs()))).collect();
        let (lo, hi) = windows.iter().fold((9.0f32, 0.0f32), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(lo < hi * 0.4, "the level rises and falls: {lo}..{hi}");
    }

    // F-A17: recording the master takes what plays, from the moment it is asked until it is stopped.
    #[test]
    fn it_records_what_plays() {
        let (mut e, mut h) = rig();
        h.seq(Command::ToggleStep { track: 0, step: 0 });
        h.send(EngineCommand::Play);
        render(&mut e, 128);
        h.send(EngineCommand::Record(Some(RecSource::Resample)));
        let out = render(&mut e, 12_000);
        assert!(h.status.recording.load(Relaxed));
        let n = h.status.rec_len.load(Relaxed) as usize;
        assert_eq!(n, 12_000, "one sample for every one that went out");
        h.send(EngineCommand::Record(None));
        render(&mut e, 256);
        assert!(!h.status.recording.load(Relaxed));
        assert_eq!(h.status.rec_len.load(Relaxed) as usize, n, "stopped means it stops growing");
        let take = h.status.take.lock().unwrap();
        // the take is the mono mix of the output (the kick is centred, so left = right)
        for i in (0..n).step_by(997) {
            assert!((take[i] - out[i]).abs() < 1e-5, "sample {i}");
        }
        assert!(take[..n].iter().any(|x| x.abs() > 0.05), "it heard the kick");
    }

    // F-A23: armed, the recording waits for a sound louder than the threshold and starts just before it (ten milliseconds), so that the
    // beginning of the sound is not cut; stopping disarms it, and nothing is kept while it waits.
    #[test]
    fn f_a23_an_armed_recording_starts_at_the_sound() {
        let (mut e, mut h) = rig();
        h.seq(Command::ToggleStep { track: 0, step: 0 });
        render(&mut e, 128);
        h.send(EngineCommand::RecordWhenLoud(RecSource::Resample, 0.05));
        render(&mut e, 4800);
        assert!(h.status.armed.load(Relaxed) && !h.status.recording.load(Relaxed), "it waits");
        assert_eq!(h.status.rec_len.load(Relaxed), 0, "and keeps nothing while it waits");
        h.send(EngineCommand::Play);
        render(&mut e, 4800);
        assert!(h.status.recording.load(Relaxed) && !h.status.armed.load(Relaxed), "the sound started it");
        let n = h.status.rec_len.load(Relaxed) as usize;
        assert!(n > PREROLL && n <= 4800 + PREROLL, "it has the sound and the ten milliseconds before it: {n}");
        let take = h.status.take.lock().unwrap();
        assert!(take[0].abs() < 0.05 && take[PREROLL - 1].abs() >= 0.05, "the take begins in silence and the sound is right after: {} {}", take[0], take[PREROLL - 1]);
        drop(take);
        h.send(EngineCommand::Record(None));
        render(&mut e, 128);
        assert!(!h.status.recording.load(Relaxed));
        // armed again with nothing playing, then stopped before any sound: it is disarmed
        h.send(EngineCommand::Stop);
        render(&mut e, 48_000);
        h.send(EngineCommand::RecordWhenLoud(RecSource::Resample, 0.05));
        render(&mut e, 128);
        assert!(h.status.armed.load(Relaxed));
        h.send(EngineCommand::Record(None));
        render(&mut e, 128);
        assert!(!h.status.armed.load(Relaxed) && !h.status.recording.load(Relaxed));
    }

    // F-A17: recording the input takes what comes in and nothing else.
    #[test]
    fn it_records_the_input() {
        let (mut e, mut h) = rig();
        let (mut tx, rx) = rtrb::RingBuffer::new(48_000);
        h.send(EngineCommand::SetInput(Box::new(rx)));
        render(&mut e, 128);
        for i in 0..6_000 {
            tx.push(((i % 100) as f32 / 100.0) - 0.5).unwrap();
        }
        h.send(EngineCommand::Record(Some(RecSource::Input)));
        render(&mut e, 4_096);
        h.send(EngineCommand::Record(None));
        render(&mut e, 128);
        let n = h.status.rec_len.load(Relaxed) as usize;
        assert_eq!(n, 4_096);
        let take = h.status.take.lock().unwrap();
        assert!((take[0] + 0.5).abs() < 1e-6 && (take[50] - 0.0).abs() < 1e-6);
        assert!(h.status.rec_peak() <= 0.5);
    }

    // F-A17: ten seconds is the most it keeps, and it stops by itself there.
    #[test]
    fn it_stops_at_ten_seconds() {
        let (mut e, mut h) = rig();
        h.send(EngineCommand::Record(Some(RecSource::Resample)));
        render(&mut e, MAX_RECORD + 5_000);
        assert_eq!(h.status.rec_len.load(Relaxed) as usize, MAX_RECORD);
        assert!(!h.status.recording.load(Relaxed));
    }
}
