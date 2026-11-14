//! A default set: the cartridges in their slots and a small pattern, used by
//! `op8 run` on first boot and by `op8 sim`.

use op8_cartridge::InstrumentCartridge;
use op8_cartridges::{Drums, FmBass, KickSyn, MacroOsc, MidiMessage, MidiOutBridge};
use op8_engine::{Engine, EngineHandle, SharedCartridge};
use op8_seq::{Project, Step, StepComponent, TRACKS};
use std::sync::{Arc, Mutex};

fn shared(c: impl InstrumentCartridge + 'static) -> Option<SharedCartridge> {
    Some(Arc::new(Mutex::new(
        Box::new(c) as Box<dyn InstrumentCartridge>
    )))
}

/// A: KickSyn, B: FmBass, C: MidiOutBridge, D: macro, E: drums; F–H empty.
pub fn slots() -> (
    [Option<SharedCartridge>; TRACKS],
    rtrb::Consumer<MidiMessage>,
) {
    let (bridge, midi_rx) = MidiOutBridge::new(0);
    let mut s: [Option<SharedCartridge>; TRACKS] = Default::default();
    s[0] = shared(KickSyn::new());
    s[1] = shared(FmBass::new());
    s[2] = shared(bridge);
    s[3] = shared(MacroOsc::new());
    s[4] = shared(Drums::new());
    (s, midi_rx)
}

/// Four-on-the-floor kick, a 7-step polymetric bass line linked to the Brain
/// (I–vi–IV–V), and a 5-step MIDI arp with components.
pub fn project() -> Project {
    let mut p = Project::default();
    p.bpm = 122.0;
    p.swing = 0.54;
    for s in [0, 4, 8, 12] {
        p.tracks[0].steps[s] = Step::on(36);
    }
    p.tracks[0].steps[14] = Step::on(36)
        .with(StepComponent::Chance(50))
        .with(StepComponent::Velocity(-40));

    let bass = &mut p.tracks[1];
    bass.length = 7;
    bass.brain_link = true;
    bass.steps[0] = Step::on(36);
    bass.steps[2] = Step::on(48).with(StepComponent::Ratchet(2));
    bass.steps[3] = Step::on(43);
    bass.steps[5] = Step::on(40)
        .with(StepComponent::Every { n: 2, of: 2 })
        .with(StepComponent::Octave(1));

    let arp = &mut p.tracks[2];
    arp.length = 5;
    arp.brain_link = true;
    arp.steps[0] = Step::on(60);
    arp.steps[1] = Step::on(64);
    arp.steps[2] = Step::on(67).with(StepComponent::RandomNote(5));
    arp.steps[4] = Step::on(72).with(StepComponent::Chance(60));

    // macro: a slow chord-ish line on the 'va' engine, linked to the Brain.
    let synth = &mut p.tracks[3];
    synth.length = 16;
    synth.brain_link = true;
    synth.steps[0] = Step::on(60);
    synth.steps[6] = Step::on(67);
    synth.steps[10] = Step::on(64).with(StepComponent::Octave(1));
    // drums: a backbeat. The note picks the sound by pitch class:
    // 49 = C# snare, 50 = D closed hat, 51 = D# open hat, 54 = F# rim.
    let kit = &mut p.tracks[4];
    kit.length = 16;
    for s in [4, 12] {
        kit.steps[s] = Step::on(49);
    }
    for s in [2, 6, 10] {
        kit.steps[s] = Step::on(50);
    }
    kit.steps[14] = Step::on(51);
    kit.steps[7] = Step::on(54).with(StepComponent::Chance(60));
    p
}

pub fn rig() -> (Engine, EngineHandle, rtrb::Consumer<MidiMessage>) {
    let (slots, midi) = slots();
    let (e, h) = Engine::new(project(), slots);
    (e, h, midi)
}

/// Same slots, empty pattern.
pub fn empty_rig() -> (Engine, EngineHandle, rtrb::Consumer<MidiMessage>) {
    let (slots, midi) = slots();
    let (e, h) = Engine::new(Project::default(), slots);
    (e, h, midi)
}
