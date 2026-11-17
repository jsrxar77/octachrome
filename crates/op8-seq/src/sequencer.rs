//! The clock. Turns a [`Project`] into sample-accurate note events.
//!
//! Each call to [`Sequencer::process`] covers one audio block. Ticks (96 PPQN)
//! are placed at exact fractional sample positions and rounded down, so a
//! step at 120 BPM lands on the same sample every time regardless of block
//! size. Events that fall after the current block (swing, delays, ratchets,
//! note-offs) wait in a sorted, fixed-capacity queue.

use crate::{
    fixed::FixedVec,
    project::{Command, Project},
    Chord, Rng, TRACKS,
};
use op8_cartridge::{NoteEvent, NoteKind};

pub const MAX_EVENTS_PER_TRACK: usize = 128;
pub const MAX_PARAM_EVENTS: usize = 64;
const MAX_PENDING: usize = 1024;

/// A parameter-lock change for a track's cartridge.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParamEvent {
    pub track: u8,
    pub sample_offset: u32,
    pub index: u8,
    /// `Some(v)` = lock to `v`, `None` = return to the knob's own value.
    pub value: Option<f32>,
}

/// A MIDI real-time message the sequencer's own time calls for (MIDI clock out).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyncKind {
    /// 0xFA: the sequencer starts from the top.
    #[default]
    Start,
    /// 0xF8: 24 to the quarter note, one every four ticks, whatever the tempo.
    Clock,
    /// 0xFC: the sequencer stopped.
    Stop,
}

/// A real-time message and where in the block it falls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncEvent {
    pub offset: u32,
    pub kind: SyncKind,
}

pub const MAX_SYNC_EVENTS: usize = 16;

/// Everything the sequencer produced for one block.
#[derive(Clone, Debug, Default)]
pub struct BlockEvents {
    pub notes: [FixedVec<NoteEvent, MAX_EVENTS_PER_TRACK>; TRACKS],
    pub params: FixedVec<ParamEvent, MAX_PARAM_EVENTS>,
    /// Start, clock and stop, for the MIDI output.
    pub sync: FixedVec<SyncEvent, MAX_SYNC_EVENTS>,
}

impl BlockEvents {
    pub fn clear(&mut self) {
        self.notes.iter_mut().for_each(FixedVec::clear);
        self.params.clear();
        self.sync.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.notes.iter().all(FixedVec::is_empty) && self.params.is_empty() && self.sync.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Playhead {
    /// Step index within the track, 0..length.
    pub step: u8,
    /// Completed loops of this track since play.
    pub iteration: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Pending {
    time: u64,
    track: u8,
    ev: NoteEvent,
}

impl Pending {
    fn key(&self) -> (u64, u8) {
        (self.time, matches!(self.ev.kind, NoteKind::On) as u8)
    }
}

#[derive(Clone, Copy, Debug)]
struct Repeat {
    start: u64,
    len: u64,
}

pub struct Sequencer {
    pub project: Project,
    sample_rate: f64,
    playing: bool,
    /// Next tick to process.
    tick: u64,
    /// Absolute sample time of `tick`.
    next_tick_at: f64,
    /// Absolute sample time at the start of the next block.
    now: u64,
    last_eff: u64,
    pending: FixedVec<Pending, MAX_PENDING>,
    held: [u128; TRACKS],
    locked: [u8; TRACKS],
    restore_on_next_block: bool,
    /// It stopped while playing: the next block says so to the MIDI output.
    stop_pending: bool,
    rng: [Rng; TRACKS],
    repeat: Option<Repeat>,
    repeat_request: Option<u32>,
    dropped: u32,
}

impl Sequencer {
    pub fn new(project: Project, sample_rate: f32) -> Self {
        let mut s = Self {
            project,
            sample_rate: sample_rate as f64,
            playing: false,
            tick: 0,
            next_tick_at: 0.0,
            now: 0,
            last_eff: 0,
            pending: FixedVec::new(),
            held: [0; TRACKS],
            locked: [0; TRACKS],
            restore_on_next_block: false,
            stop_pending: false,
            rng: [Rng::new(0); TRACKS],
            repeat: None,
            repeat_request: None,
            dropped: 0,
        };
        s.reseed();
        s
    }

    /// Replace the whole project (the FILE button). Whatever sounds is released, and if the
    /// sequencer was playing it starts again from the first step.
    pub fn load_project(&mut self, p: Project) {
        let was_playing = self.playing;
        self.stop();
        self.project = p;
        self.reseed();
        if was_playing {
            self.play();
        }
    }

    fn reseed(&mut self) {
        for (i, r) in self.rng.iter_mut().enumerate() {
            *r = Rng::new(self.project.seed ^ ((i as u64 + 1) << 32));
        }
    }

    pub fn apply(&mut self, cmd: &Command) {
        self.project.apply(cmd);
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Start from the top at the next block.
    pub fn play(&mut self) {
        self.playing = true;
        self.tick = 0;
        self.last_eff = 0;
        self.next_tick_at = self.now as f64;
        self.repeat = None;
        self.reseed();
    }

    /// Start at the next block from `tick` (96 to the quarter note) instead of from the top: where a song position pointer says.
    pub fn play_from(&mut self, tick: u64) {
        self.play();
        self.tick = tick;
        self.last_eff = tick;
    }

    /// Stop and release every sounding note at the start of the next block.
    pub fn stop(&mut self) {
        self.stop_pending |= self.playing;
        self.playing = false;
        let now = self.now;
        self.pending.clear();
        for t in 0..TRACKS {
            let mut held = self.held[t];
            while held != 0 {
                let n = held.trailing_zeros() as u8;
                held &= held - 1;
                self.insert(Pending {
                    time: now,
                    track: t as u8,
                    ev: NoteEvent::off(0, n),
                });
            }
        }
        self.restore_on_next_block = self.locked.iter().any(|&m| m != 0);
    }

    /// Engage (Some, in ticks) or release (None) a beat-repeat window — the
    /// sequencer half of Punch-in FX. Takes effect on the next tick.
    pub fn set_repeat(&mut self, ticks: Option<u32>) {
        self.repeat_request = ticks.map(|t| t.max(1));
        if ticks.is_none() {
            self.repeat = None;
        }
    }

    pub fn samples_per_tick(&self) -> f64 {
        self.sample_rate * 60.0 / (self.project.bpm.max(1.0) as f64 * crate::PPQN as f64)
    }

    /// Where `track` is, for the UI and the pad LEDs.
    pub fn playhead(&self, track: usize) -> Playhead {
        let t = &self.project.tracks[track];
        let step_global = self.last_eff / t.division.ticks() as u64;
        let len = t.len() as u64;
        Playhead {
            step: (step_global % len) as u8,
            iteration: (step_global / len) as u32,
        }
    }

    /// The tick the song has reached (96 per quarter note), counted straight on: a beat repeat that loops a part does not turn it back.
    pub fn song_tick(&self) -> u64 {
        self.tick
    }

    /// The tick the sequencer has reached (96 per quarter note), for the metronome.
    pub fn tick_position(&self) -> u64 {
        self.last_eff
    }

    /// How far into its current step a track is, 0.0..1.0 (for recording to the nearest step).
    pub fn step_phase(&self, track: usize) -> f32 {
        let ticks = self.project.tracks[track].division.ticks() as u64;
        (self.last_eff % ticks) as f32 / ticks as f32
    }

    /// The Brain chord sounding now.
    pub fn current_chord(&self) -> Chord {
        self.project.brain.chord_at(self.last_eff / 24)
    }

    /// Index of the Brain progression slot sounding now.
    pub fn current_chord_slot(&self) -> usize {
        self.project.brain.slot_at(self.last_eff / 24)
    }

    /// Events that did not fit in the queue (should stay 0).
    pub fn dropped_events(&self) -> u32 {
        self.dropped
    }

    /// Produce the events for the next `frames` samples.
    pub fn process(&mut self, frames: usize, out: &mut BlockEvents) {
        out.clear();
        if std::mem::take(&mut self.stop_pending) {
            out.sync.push(SyncEvent { offset: 0, kind: SyncKind::Stop });
        }
        if self.restore_on_next_block {
            self.restore_on_next_block = false;
            for t in 0..TRACKS {
                self.emit_restores(t, 0, 0, out);
            }
        }
        let end = self.now + frames as u64;
        if self.playing {
            while self.next_tick_at < end as f64 {
                let at = (self.next_tick_at.floor() as u64).max(self.now);
                self.on_tick(at, out);
                self.tick += 1;
                self.next_tick_at += self.samples_per_tick();
            }
        }
        let mut n = 0;
        for p in self.pending.iter() {
            if p.time >= end {
                break;
            }
            let mut ev = p.ev;
            ev.sample_offset = p.time.saturating_sub(self.now) as u32;
            let bit = 1u128 << ev.note;
            match ev.kind {
                NoteKind::On => self.held[p.track as usize] |= bit,
                NoteKind::Off => self.held[p.track as usize] &= !bit,
            }
            if !out.notes[p.track as usize].push(ev) {
                self.dropped += 1;
            }
            n += 1;
        }
        self.pending.drain_front(n);
        self.now = end;
    }

    fn on_tick(&mut self, at: u64, out: &mut BlockEvents) {
        let tick = self.tick;
        if let Some(len) = self.repeat_request.take() {
            let len = len as u64;
            self.repeat = Some(Repeat {
                start: tick - tick % len,
                len,
            });
        }
        let (eff, window_start) = match self.repeat {
            Some(r) => (
                r.start + (tick - r.start) % r.len,
                (tick - r.start).is_multiple_of(r.len),
            ),
            None => (tick, false),
        };
        self.last_eff = eff;
        let spt = self.samples_per_tick();
        let offset = (at - self.now) as u32;
        // MIDI clock follows the ticks that go by (not `eff`, which a beat repeat loops): start on the first, a clock on every fourth
        if tick == 0 {
            out.sync.push(SyncEvent { offset, kind: SyncKind::Start });
        }
        if tick % (crate::PPQN as u64 / 24) == 0 {
            out.sync.push(SyncEvent { offset, kind: SyncKind::Clock });
        }

        for tr in 0..TRACKS {
            let track = self.project.tracks[tr];
            let tps = track.division.ticks() as u64;
            if eff % tps != 0 && !window_start {
                continue;
            }
            let step_global = eff / tps;
            let len = track.len() as u64;
            let step = track.steps[(step_global % len) as usize];
            let r = step.resolve(step_global / len, &mut self.rng[tr]);

            let mut lock_mask = 0u8;
            for (i, l) in r.locks.iter().enumerate() {
                if l.is_some() && !track.muted {
                    lock_mask |= 1 << i;
                }
            }
            self.emit_restores(tr, offset, lock_mask, out);
            if track.muted || !r.fire {
                self.locked[tr] = 0;
                continue;
            }
            for (i, l) in r.locks.iter().enumerate() {
                if let Some(v) = *l {
                    let ok = out.params.push(ParamEvent {
                        track: tr as u8,
                        sample_offset: offset,
                        index: i as u8,
                        value: Some(v),
                    });
                    if !ok {
                        self.dropped += 1;
                    }
                }
            }
            self.locked[tr] = lock_mask;

            let note = if track.brain_link {
                let b = &self.project.brain;
                b.apply(r.note, b.chord_at(eff / 24))
            } else {
                r.note
            };
            let step_samples = tps as f64 * spt;
            let swing = if step_global % 2 == 1 {
                (self.project.swing as f64 - 0.5) * 2.0 * step_samples
            } else {
                0.0
            };
            let start = at as f64 + swing + r.delay_ticks as f64 * spt;
            let n = r.ratchet.max(1) as f64;
            let sub = step_samples / n;
            let gate = if r.ratchet > 1 {
                sub * (r.gate_steps as f64).min(1.0)
            } else {
                step_samples * r.gate_steps as f64
            };
            let players = track.players;
            if track.kit || players.mode == crate::PlayMode::Off {
                for k in 0..r.ratchet.max(1) {
                    let on = (start + k as f64 * sub).floor() as u64;
                    let off = on + (gate.floor() as u64).max(1);
                    if track.kit {
                        // a drum kit: one note per lane that is on in this step
                        for lane in (0..8u8).filter(|l| step.lanes & (1 << l) != 0) {
                            self.schedule_note(tr as u8, crate::step::KIT_BASE_NOTE + lane, r.velocity, on, off);
                        }
                    } else {
                        self.schedule_note(tr as u8, note, r.velocity, on, off);
                    }
                }
            } else {
                // the players: a chord on the note, or an arpeggio over it
                let mut notes = [0u8; crate::rack::MAX_PLAYED];
                let n = players.chord_notes(note, &mut notes);
                if players.mode == crate::PlayMode::Chord {
                    let strum = step_samples * players.speed as f64 / 32.0;
                    for (i, &nt) in notes[..n].iter().enumerate() {
                        let on = (start + i as f64 * strum).floor() as u64;
                        let off = on + (gate.floor() as u64).max(1);
                        self.schedule_note(tr as u8, nt, r.velocity, on, off);
                    }
                } else {
                    let count = players.arp_count();
                    let each = step_samples / count as f64;
                    let hold = (each * (r.gate_steps as f64).min(1.0).max(0.1)).floor() as u64;
                    for k in 0..count {
                        let idx = players.arp_index(step_global * count as u64 + k as u64, n, &mut self.rng[tr]);
                        let on = (start + k as f64 * each).floor() as u64;
                        self.schedule_note(tr as u8, notes[idx], r.velocity, on, on + hold.max(1));
                    }
                }
            }
        }
    }

    /// Restore every param that was locked on the previous step of `tr` and
    /// is not locked again by `keep`.
    fn emit_restores(&mut self, tr: usize, offset: u32, keep: u8, out: &mut BlockEvents) {
        let mut m = self.locked[tr] & !keep;
        while m != 0 {
            let i = m.trailing_zeros() as u8;
            m &= m - 1;
            if !out.params.push(ParamEvent {
                track: tr as u8,
                sample_offset: offset,
                index: i,
                value: None,
            }) {
                self.dropped += 1;
            }
        }
    }

    fn schedule_note(&mut self, track: u8, note: u8, velocity: f32, on: u64, off: u64) {
        // A retrigger of a note that is still sounding (or will be) cuts it
        // cleanly first, so a stale note-off can never chop the new note.
        let before = self.pending.len();
        self.pending.retain(|p| {
            !(p.track == track && p.ev.note == note && p.ev.kind == NoteKind::Off && p.time > on)
        });
        if self.pending.len() != before {
            self.insert(Pending {
                time: on,
                track,
                ev: NoteEvent::off(0, note),
            });
        }
        self.insert(Pending {
            time: on,
            track,
            ev: NoteEvent::on(0, note, velocity),
        });
        self.insert(Pending {
            time: off,
            track,
            ev: NoteEvent::off(0, note),
        });
    }

    fn insert(&mut self, p: Pending) {
        let key = p.key();
        let idx = self
            .pending
            .iter()
            .position(|q| q.key() > key)
            .unwrap_or(self.pending.len());
        if !self.pending.insert(idx, p) {
            self.dropped += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Division, Step, StepComponent};

    const SR: f32 = 48_000.0;

    /// Run for `samples` in blocks of `block`, returning (absolute time, track, event).
    fn run(seq: &mut Sequencer, samples: usize, block: usize) -> Vec<(u64, usize, NoteEvent)> {
        let mut out = BlockEvents::default();
        let mut log = Vec::new();
        let mut t = 0u64;
        while (t as usize) < samples {
            seq.process(block, &mut out);
            for (tr, evs) in out.notes.iter().enumerate() {
                for e in evs.iter() {
                    log.push((t + e.sample_offset as u64, tr, *e));
                }
            }
            t += block as u64;
        }
        log
    }

    fn ons(log: &[(u64, usize, NoteEvent)], track: usize) -> Vec<u64> {
        log.iter()
            .filter(|(_, tr, e)| *tr == track && e.kind == NoteKind::On)
            .map(|(t, _, _)| *t)
            .collect()
    }

    fn four_on_the_floor() -> Project {
        let mut p = Project::default();
        for s in [0, 4, 8, 12] {
            p.tracks[0].steps[s] = Step::on(36);
        }
        p
    }

    #[test]
    fn sixteenths_are_sample_exact_at_120() {
        // 120 BPM: a 1/16 is 6000 samples, a beat 24000.
        let mut seq = Sequencer::new(four_on_the_floor(), SR);
        seq.play();
        let log = run(&mut seq, 96_000, 64);
        assert_eq!(ons(&log, 0), [0, 24_000, 48_000, 72_000]);
    }

    #[test]
    fn block_size_does_not_change_timing() {
        let mut p = four_on_the_floor();
        p.bpm = 133.7;
        p.swing = 0.6;
        p.tracks[0].steps[3] = Step::on(40)
            .with(StepComponent::Ratchet(3))
            .with(StepComponent::Delay(5));
        let mut a = Sequencer::new(p, SR);
        let mut b = Sequencer::new(p, SR);
        a.play();
        b.play();
        let la = run(&mut a, 200_000, 64);
        let lb = run(&mut b, 200_000, 1000);
        assert_eq!(ons(&la, 0), ons(&lb, 0));
    }

    #[test]
    fn deterministic_with_randomness() {
        let mut p = Project::default();
        for s in 0..16 {
            p.tracks[2].steps[s] = Step::on(60)
                .with(StepComponent::Chance(50))
                .with(StepComponent::RandomNote(12));
        }
        let go = || {
            let mut s = Sequencer::new(p, SR);
            s.play();
            run(&mut s, 300_000, 128)
        };
        let a = go();
        assert!(!a.is_empty());
        assert_eq!(a, go());
    }

    #[test]
    fn polymeter_realigns_after_lcm() {
        let mut p = Project::default();
        p.tracks[0].length = 3;
        p.tracks[1].length = 4;
        p.tracks[0].steps[0] = Step::on(36);
        p.tracks[1].steps[0] = Step::on(38);
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 6000 * 24 + 1, 64);
        let a = ons(&log, 0);
        let b = ons(&log, 1);
        assert_eq!(a, (0..=8).map(|i| i * 3 * 6000).collect::<Vec<_>>());
        assert_eq!(b, (0..=6).map(|i| i * 4 * 6000).collect::<Vec<_>>());
        let both: Vec<_> = a.iter().filter(|t| b.contains(t)).collect();
        assert_eq!(both, [&0, &72_000, &144_000]); // every 12 steps
    }

    #[test]
    fn division_changes_step_length() {
        let mut p = Project::default();
        p.tracks[0].division = Division::Eighth;
        p.tracks[0].length = 2;
        p.tracks[0].steps[0] = Step::on(36);
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 48_001, 64);
        assert_eq!(ons(&log, 0), [0, 24_000, 48_000]);
    }

    #[test]
    fn ratchet_subdivides_and_gates_close() {
        let mut p = Project::default();
        p.tracks[0].steps[0] = Step::on(36).with(StepComponent::Ratchet(4));
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 6000, 64);
        assert_eq!(ons(&log, 0), [0, 1500, 3000, 4500]);
        // every on has a matching off before the step ends
        let offs = log
            .iter()
            .filter(|(t, _, e)| e.kind == NoteKind::Off && *t > 0)
            .count();
        assert_eq!(offs, 4);
    }

    // F-S09
    #[test]
    fn a_kit_step_plays_one_note_per_lane() {
        let mut p = Project::default();
        p.tracks[0].kit = true;
        p.tracks[0].steps[0] = Step::kit(0b0000_0101); // lanes 0 and 2 together
        p.tracks[0].steps[1] = Step::kit(0b1000_0000); // lane 7 alone
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 12_000, 64);
        let at = |t: u64| {
            let mut n: Vec<u8> = log.iter().filter(|(tt, tr, e)| *tt == t && *tr == 0 && e.kind == NoteKind::On).map(|(_, _, e)| e.note).collect();
            n.sort();
            n
        };
        assert_eq!(at(0), [60, 62], "two lanes in one step sound together");
        assert_eq!(at(6000), [67], "the next step has only its own lane");
    }

    #[test]
    fn leaving_the_kit_turns_lanes_back_into_notes() {
        let mut p = Project::default();
        p.tracks[0].kit = true;
        p.tracks[0].steps[2] = Step::kit(0b0100_0100); // lanes 2 and 6: the first becomes the note
        p.apply(&Command::SetKit { track: 0, kit: false });
        let s = p.tracks[0].steps[2];
        assert!(!p.tracks[0].kit && s.active && s.lanes == 0 && s.note == 62);
    }

    #[test]
    fn kit_commands_toggle_lanes_and_convert_notes() {
        let mut p = Project::default();
        p.tracks[0].steps[3] = Step::on(61); // a snare by its pitch class
        p.apply(&Command::SetKit { track: 0, kit: true });
        assert_eq!(p.tracks[0].steps[3].lanes, 0b10, "an existing note becomes its lane");
        p.apply(&Command::ToggleLane { track: 0, step: 3, lane: 3 });
        assert_eq!(p.tracks[0].steps[3].lanes, 0b1010);
        p.apply(&Command::ToggleLane { track: 0, step: 3, lane: 1 });
        p.apply(&Command::ToggleLane { track: 0, step: 3, lane: 3 });
        assert!(!p.tracks[0].steps[3].active, "no lane left, the step is off");
    }

    #[test]
    fn swing_delays_odd_steps() {
        let mut p = Project::default();
        p.swing = 0.75;
        p.tracks[0].steps[0] = Step::on(36);
        p.tracks[0].steps[1] = Step::on(36);
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 12_000, 64);
        assert_eq!(ons(&log, 0), [0, 9000]);
    }

    #[test]
    fn brain_link_reharmonises() {
        let mut p = Project::default(); // I vi IV V, one bar each
        p.tracks[0].steps[0] = Step::on(64); // E
        p.tracks[0].brain_link = true;
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 6000 * 64, 128);
        let notes: Vec<u8> = log
            .iter()
            .filter(|(_, _, e)| e.kind == NoteKind::On)
            .map(|(_, _, e)| e.note)
            .collect();
        // C: E  | Am: E-3=C# -> C | F: E+5=A | G: E-5=B
        assert_eq!(notes, [64, 60, 69, 59]);
    }

    #[test]
    fn param_locks_apply_then_restore() {
        let mut p = Project::default();
        p.tracks[0].length = 2;
        p.tracks[0].steps[0] = Step::on(36).with(StepComponent::ParamLock {
            index: 1,
            value: 255,
        });
        p.tracks[0].steps[1] = Step::on(36);
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let mut out = BlockEvents::default();
        seq.process(64, &mut out);
        assert_eq!(
            out.params.as_slice(),
            &[ParamEvent {
                track: 0,
                sample_offset: 0,
                index: 1,
                value: Some(1.0)
            }]
        );
        let mut restored = false;
        for _ in 0..200 {
            seq.process(64, &mut out);
            restored |= out.params.iter().any(|e| e.index == 1 && e.value.is_none());
        }
        assert!(restored);
    }

    #[test]
    fn stop_releases_held_notes() {
        let mut p = Project::default();
        p.tracks[0].steps[0] = Step {
            gate: 100,
            ..Step::on(36)
        }
        .with(StepComponent::Hold(4));
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let mut out = BlockEvents::default();
        seq.process(256, &mut out);
        assert!(out.notes[0].iter().any(|e| e.kind == NoteKind::On));
        seq.stop();
        seq.process(256, &mut out);
        assert_eq!(out.notes[0].as_slice(), &[NoteEvent::off(0, 36)]);
        for _ in 0..2000 {
            seq.process(256, &mut out);
            assert!(out.notes[0].is_empty(), "nothing after stop");
        }
    }

    #[test]
    fn beat_repeat_loops_current_window() {
        let mut p = Project::default();
        for s in 0..16 {
            p.tracks[0].steps[s] = Step::on(36 + s as u8);
        }
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let mut out = BlockEvents::default();
        // run to step 4
        for _ in 0..(6000 * 4 / 64) {
            seq.process(64, &mut out);
        }
        seq.set_repeat(Some(48)); // 1/8 window = 2 steps
        let mut notes = Vec::new();
        for _ in 0..(6000 * 8 / 64) {
            seq.process(64, &mut out);
            notes.extend(
                out.notes[0]
                    .iter()
                    .filter(|e| e.kind == NoteKind::On)
                    .map(|e| e.note),
            );
        }
        assert_eq!(&notes[..6], &[40, 41, 40, 41, 40, 41]);
    }

    #[test]
    fn mute_silences_but_keeps_time() {
        let mut p = four_on_the_floor();
        p.tracks[0].muted = true;
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        assert!(ons(&run(&mut seq, 48_000, 64), 0).is_empty());
        seq.apply(&Command::SetMute {
            track: 0,
            muted: false,
        });
        assert!(!ons(&run(&mut seq, 48_000, 64), 0).is_empty());
    }

    fn notes_of(log: &[(u64, usize, NoteEvent)], track: usize) -> Vec<(u64, u8)> {
        log.iter()
            .filter(|(_, tr, e)| *tr == track && e.kind == NoteKind::On)
            .map(|(t, _, e)| (*t, e.note))
            .collect()
    }

    // F-S13: a chord player sounds the whole chord on the note of the step, together or strummed.
    #[test]
    fn a_chord_player_sounds_the_chord() {
        use crate::{ChordType, PlayMode, Players};
        let mut p = Project::default();
        p.tracks[0].steps[0] = Step::on(60);
        p.tracks[0].players = Players { mode: PlayMode::Chord, chord: ChordType::Minor, speed: 0, octaves: 1 };
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 3_000, 64);
        assert_eq!(notes_of(&log, 0), [(0, 60), (0, 63), (0, 67)]);
        // strummed: the notes follow one another inside the step
        p.tracks[0].players.speed = 4;
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 6_000, 64);
        let n = notes_of(&log, 0);
        assert_eq!(n.iter().map(|x| x.1).collect::<Vec<_>>(), [60, 63, 67]);
        assert!(n[0].0 < n[1].0 && n[1].0 < n[2].0 && n[2].0 < 6_000, "{n:?}");
    }

    // F-S13: an arpeggio plays the chord one note after another, in the step's own time, and keeps going round across steps.
    #[test]
    fn an_arpeggio_player_runs_through_the_chord() {
        use crate::{PlayMode, Players};
        let mut p = Project::default();
        p.tracks[0].steps[0] = Step::on(60);
        p.tracks[0].steps[1] = Step::on(60);
        p.tracks[0].players = Players { mode: PlayMode::Up, speed: 1, ..Default::default() }; // three notes per step
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let log = run(&mut seq, 12_000, 64);
        let n = notes_of(&log, 0);
        assert_eq!(n.iter().map(|x| x.1).collect::<Vec<_>>(), [60, 64, 67, 60, 64, 67]);
        assert_eq!(n[1].0 - n[0].0, 2_000, "three notes share a 6000-sample step");
        // down
        p.tracks[0].players.mode = PlayMode::Down;
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let n = notes_of(&run(&mut seq, 5_000, 64), 0);
        assert_eq!(n.iter().map(|x| x.1).collect::<Vec<_>>(), [67, 64, 60]);
    }

    // F-S13: the players leave a drum kit alone.
    #[test]
    fn players_do_not_touch_a_kit() {
        use crate::{PlayMode, Players};
        let mut p = Project::default();
        p.tracks[0].kit = true;
        p.tracks[0].steps[0] = Step::kit(0b11);
        p.tracks[0].players = Players { mode: PlayMode::Chord, ..Default::default() };
        let mut seq = Sequencer::new(p, SR);
        seq.play();
        let n = notes_of(&run(&mut seq, 1_000, 64), 0);
        assert_eq!(n.iter().map(|x| x.1).collect::<Vec<_>>(), [60, 61]);
    }

    // F-S12: shifting a track turns its steps round inside its length.
    #[test]
    fn shifting_a_track_turns_it_round() {
        let mut p = Project::default();
        p.tracks[0].length = 4;
        p.tracks[0].steps[0] = Step::on(36);
        p.tracks[0].steps[9] = Step::on(40); // outside the length: stays
        p.apply(&crate::Command::ShiftTrack { track: 0, by: 1 });
        assert!(p.tracks[0].steps[1].active && !p.tracks[0].steps[0].active);
        p.apply(&crate::Command::ShiftTrack { track: 0, by: -2 });
        assert!(p.tracks[0].steps[3].active, "wraps round the end");
        assert!(p.tracks[0].steps[9].active);
    }
}
