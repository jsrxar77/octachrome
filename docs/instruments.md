# OP-8 — Instruments

**Instruments are the centre of OP-8.** The sequencer, the Brain, Punch-in and the buttons always do the same and stay small. What grows is the collection of instruments ("cartridges"): it starts with a small set, anyone can add a new one, and each is used as quickly as on an OP-XY.

This document rules together with the [UX/UI Treatise](design.md) and the [button map](buttons.md).

## 1. What an instrument is

An instrument is a piece of code that fulfils the `InstrumentCartridge` contract (`crates/op8-cartridge`). It has four parts, always the same:

| Part | What it is | Limit |
| :-- | :-- | :-- |
| **Sound** | Receives notes, returns stereo audio at 48 kHz | Must fit the CPU budget of the Pi (section 5) |
| **Parameters** | 4 of sound (button 5, knobs 5–8) and, if needed, 4 of shape (button 6) | **At most 8.** If the engine has more, the instrument picks the 8 that matter |
| **Drawing** | An animated graphic that shows what the sound does, in the central area of the right screen | Uses the colours of knobs 5–8 for what those knobs move |
| **State** | Save and load its values (presets and parameter locks) | A list of numbers, versioned |

The track's effect and modulation (buttons 7 and 8) are not the instrument's: they are the same for all, which keeps each instrument simple. The effect is one of delay, reverb, chorus, filter or drive; the LFO can move any of the instrument's eight knobs, so what an instrument offers to move is exactly its parameter list.

## 2. Rules for an instrument

1. **One sound idea.** A kick, an FM bass, a string chord, a resonator. Not a synth with 200 parameters.
2. **Eight knobs are enough.** The first 4 are the ones that change the sound most. One-word names, lowercase (`tune`, `decay`, `color`).
3. **The drawing explains the sound.** Someone who does not know synthesis has to see what changes when each knob is turned. No text in the drawing.
4. **It always sounds good.** Any combination of the 8 knobs has to give a usable sound (as on the OP-1 and OP-XY).
5. **Fast.** It loads in under 100 ms and does not allocate memory while sounding.

## 3. How an instrument is added (extension mechanism)

In three stages, from the simplest to the most open:

| Stage | How | For whom | Status |
| :-- | :-- | :-- | :-- |
| **1. In code** | A Rust crate that implements `InstrumentCartridge` and is registered in `builtin_names()` / `create()` (`crates/op8-cartridges/src/lib.rs`). Compiled with OP-8. It can wrap MIT-licensed C/C++ code (like macro) | Us and anyone who knows Rust | ◐ (contract and list done; choosing them with PLUG-IN is missing) |
| **2. Separate package** | The instrument is compiled as a library (`.so` on the Pi, `.dylib` on Mac) and copied to a `cartridges/` folder. OP-8 loads it at start-up | Anyone who programs, without touching OP-8 | ○ |
| **3. Without compiling** | Instruments described in a patch language (for example Faust or a format of our own) that OP-8 converts | People who design sound but do not program | ○ (to investigate) |

**Drawing kit:** `op8-gfx` already has what the drawings need (smoothed circles, arcs, lines and boxes, the Jost typeface, theme and per-knob colours). Later a small library of reusable drawings is added: oscilloscope, envelope, polar waveform, keyboard, waves. That way a new instrument looks good without designing everything from scratch.

## 4. Initial set and candidates

### Already done

| Instrument | Idea | Drawing |
| :-- | :-- | :-- |
| **KickSyn** | Synthetic kick: tone, decay, punch, saturation | A drum head seen from above |
| **FmBass** | Two-operator FM bass | A planet and moon in orbit, the envelope as a mountain |
| **MidiOutBridge** | Sends the track out the MK3's DIN MIDI to an external synth | A DIN connector and notes leaving through the cable |
| **resonator** | A string or a bank of bells that rings when a note strikes it; four voices (a fifth note takes the oldest). Sound: `type` (string, bells), `structure`, `bright`, `damping`. Shape: `decay`, `color`, `spread` (how far apart the four voices sit in the stereo field), `level`. Built on the string and modal engines of the Plaits code (MIT) | A string held at both ends that shivers with the sound (structure bows it, bright thickens it), or eight rings spaced as the modes of a bell are, fading outward with the damping |
| **drums** | Eight percussion sounds on one track, chosen by the pitch of the note: C kick, C# snare, D closed hat, D# open hat, E tom, F clap, F# rim, G crash (the notes above G wrap round). The knobs edit the sound picked with the `sound` knob and each sound keeps its own values. Sound: `tune`, `decay`, `tone`, `snap`. Shape: `body`, `level`, `pan`, `sound`. Built on the percussion engines of the Plaits code (MIT) | The eight sounds as a 4 × 2 grid of keys, each with a small picture that flashes when it plays; the chosen one large, with the four colours of knobs 5–8 showing tune (height), decay (size), tone (rings) and snap (spikes) |
| **sampler** | Plays recorded sounds with the keyboard and the steps. Sixteen places like everything else: eight sounds that come with it (pluck, bell, pad, voice, thump, sweep, glass, glitch, all made by the code, so nothing has to be installed) and eight for the player's own WAV files from `~/.config/op8/samples/` (`OP8_SAMPLES` overrides it), in the order of their names; any rate, 8–32 bit or float, mono or stereo, up to ten seconds. A note plays the sound at its own pitch for C4 and shifted by the interval for others; eight voices. Sound: `sample`, `start`, `length`, `tune`. Shape: `attack`, `release`, `loop`, `level`. One shot plays through and fades over the last `release`; loop repeats the window while the note is held and fades when it is let go | The wave of the chosen sound across the screen, the part that plays lit between two marks (`start`, `length`), a mark for the tune, a white line for every voice that is playing |
| **granular** | Plays a sound as a cloud of grains (very short, overlapping pieces). The sound is one of the sampler's sixteen places (the eight that come with it and the player's own; what SAMPLING records goes in at once). Up to four held notes: each grain takes its pitch from them in turn (C4 is the sound's own). Sound: `sample`, `position` (where in the sound grains are cut), `size` (32 ms to 512 ms), `density` (regular below the middle, random above, a dead zone in the middle where only the grain a note starts is heard). Shape: `tune` (± 24 semitones), `texture` (the window of a grain: a trapezoid when low, a raised cosine when high), `spread` (between the speakers), `level`. Up to 32 grains at once; the quality of their interpolation falls as they get busy. Built on the granular player of Clouds (MIT) | The wave of the chosen sound, a band over the part where grains can start, a line where they are cut from (`position`) and one under it as long as a grain (`size`), and every grain that sounds as a bar where it reads, as tall as its window is open and higher or lower as it sits between the speakers (`spread`); the pitch as a mark on a short scale |
| **macro** | 24 engines in one (virtual analogue, FM, wavetable, chords, voice, strings, percussion), from the Plaits code (MIT, `crates/op8-plaits`). Sound: `engine`, `harmonics`, `timbre`, `morph`. Shape: `decay`, `color`, `variant`, `level` | A ring of 24 engines, a polar waveform that beats with the level, and a timbre × morph field |

macro is monophonic and uses gate semantics (the 6-operator FM engines need the held gate). It goes to sleep and gives exact zeros when it is not sounding.

### Candidates (known engines that run on a Pi)

| Source | What it brings | Licence | May it be included? |
| :-- | :-- | :-- | :-- |
| **Plaits** (Mutable Instruments) | 16 synthesis engines in one: virtual analogue, FM, wavetable, chords, voice, strings, percussion. Already designed for 3–4 knobs | MIT | **Yes.** There are already partial Rust ports (for example the DX7 engine) |
| **Braids** (Mutable Instruments) | More than 40 oscillators on a single "timbre/colour" control | MIT | **Yes** |
| **Rings / Elements** (Mutable Instruments) | Physical resonators: strings, bells, tubes | MIT | **Yes** |
| **Clouds / Beads** (Mutable Instruments) | Granular on a sample | MIT | **Yes** (useful for the future sampler) |
| **Surge XT** | A huge synth with very many oscillators and filters | GPL-3.0 | **Only if OP-8 becomes GPL**, or if only its ideas are used, without its code |
| **Vital** | Wavetable | GPL-3.0 | Same as Surge |

Clouds' granular code was checked on 2026-11-16: the files carry the MIT notice (Copyright 2014 Emilie Gillet), which is kept in the source of the granular.

Legal note on Mutable Instruments: the code of the STM32 modules (Plaits, Braids, Rings, Elements, Clouds, Beads) is MIT, but the name "Mutable Instruments" is a registered trademark and its author asks that the module names not be used in derived works. OP-8 instruments based on that code carry names of their own (for example "macro", "resonator") and say in the credits where they come from.

### Proposed order

1. ~~**macro**~~ (from Plaits): done.
2. ~~**resonator**~~: done (string and bells, from the Plaits modal and string engines).
3. ~~**drums**~~: done (eight sounds chosen by the pitch of the note).
4. ~~**sampler**~~, stages 1 and 2 done (plays the eight that come with it and the player's WAV files; SAMPLING records what OP-8 plays or the audio interface's input, cuts it and saves it into one of the player's eight places). Stage 3, BROWSER, is done for every instrument (presets per instrument, the player's eight; the sampler's preset keeps the sound chosen and its cut).

## 5. Budget on the Raspberry Pi 5

It has to be measured on the real Pi, but the target is: 8 tracks sounding at once, 128-sample buffers (2.7 ms), and under 60 % of one core for all the audio. Each instrument declares how many voices it uses and OP-8 warns if a combination does not fit. The CPU on the Pi 5 has not been measured yet.

## 6. What OP-8 is not going to be

- A DAW or a VST plug-in host: instruments are few parameters and a drawing, not enormous panels.
- A box of thousands of parameters: if something needs more than 8 knobs, it is two instruments.

## 7. Eight and eight

Everything in OP-8 is built on eight, and the MK3 has sixteen pads. So the library is **8 factory instruments plus 8 places for the ones you install**, and the projects are **8 factory projects plus 8 of your own**. On the pads: pads 1–8 (the top two rows) are what ships, pads 9–16 (the bottom two rows) are yours. Decided by Javi on 2026-11-13.

| | Factory (pads 1–8) | Yours (pads 9–16) |
| :-- | :-- | :-- |
| **Instruments** (PLUG-IN) | 8 that always ship and cannot be removed | 8 places for instruments installed as separate packages (stage 2 and 3 of §3) |
| **Projects** (FILE) | 8 music projects that always ship (done: eight short pieces in eight styles: house, boom bap, dub, polyrhythm, acid, electro, ambient, breaks; [§9](#9-the-eight-pieces-that-come-with-it)) | 8 places for the player's own projects (done) |

The factory eight, confirmed by Javi on 2026-11-14 (all exist since 2026-11-16). **They are ordered from the most guttural to the most refined, drums first, the leads next to last and the MIDI out last (decided by Javi on 2026-11-15)**; that order is the order of the pads of PLUG-IN and of the standard tracks: A drums, B KickSyn, C FmBass, D resonator, E sampler, F granular, G macro, H MIDI out. The MIDI out is last because it makes no sound of its own, and because the MK3 will eventually also take input: MIDI out and sound in. The order of work is drums and resonator (done), sampler (stage 1 done), granular.

| # | Instrument | Idea | Status |
| :-- | :-- | :-- | :-- |
| 1 | **drums** | Eight percussion sounds on one track, in eight lanes | ✓ |
| 2 | **KickSyn** | Synthetic kick | ✓ |
| 3 | **FmBass** | Two-operator FM bass | ✓ |
| 4 | **resonator** | Plucked strings and struck bells, four voices | ✓ |
| 5 | **sampler** | Play sounds: eight that come with it and the player's own (SAMPLING records them) | ◐ |
| 6 | **granular** | Granular on a sample (the grain scheduler of Clouds, MIT, rewritten in Rust) | ✓ |
| 7 | **macro** | 24 engines in one (from Plaits): the leads | ✓ |
| 8 | **MidiOutBridge** | The track out to an external synth (later also sound in) | ✓ |

Rules: the factory eight meet §2 like any other instrument; each factory project shows one thing (an instrument, a rhythm idea, the Brain) and is made only with the factory eight, so it always loads. Removing, replacing or overwriting a factory slot is not possible; a user slot starts empty.

### How drums are programmed: eight lanes

A drum track is a **kit** with eight lanes, one per sound, and a step can have several lanes on at once (a kick and a hat together). It is not a melody: a step has no note, its sounds are lanes.

- **Right screen (the sound):** screen button 6 (`shape`) opens the second page, whose last knob, `sound`, chooses the sound; you hear it as you turn. The other knobs edit that sound (tune, decay, tone, snap, body, level, pan) and also play it.
- **Left screen (the sequencer):** the eight lanes by sixteen steps, the chosen lane lit and the playhead marked. The sixteen pads are the steps of the chosen lane: **tap a pad** and that sound is on that step (it plays); tap again and it is off. The other lanes of the step stay as they are.
- **Components** (ratchet, chance, every…), velocity and gate are per step and apply to all its lanes; they are in the step inspector on the left (hold a pad, or screen button 2). A kit step has no `note` knob.
- Lane n plays note 60 + n; its pitch class is what picks the sound (C kick, C# snare, D closed hat, D# open hat, E tom, F clap, F# rim, G crash). A track becomes a kit the first time you tap a pad on a track whose instrument is drums.

Still to come: the chromatic KEYBOARD and PAD MODE, which will play the eight sounds straight from the pads ([buttons.md](buttons.md)).

## 8. Presets that come with the instruments

Each of the six sounding instruments has **eight presets** of its own, shown in the top two rows of BROWSER (the player's eight are the bottom two; see [buttons.md](buttons.md)). A preset is the value of the instrument's eight knobs (a knob it does not name, `-`, stays where it is: the drums' presets leave `sound`, `level` and `pan` alone, so a preset shapes whichever sound is chosen). They live in one text file, `crates/op8-cartridges/presets.txt`, read by the program and the tests and copied into the twin (a test keeps the copy equal).

They were written after the kinds of sound that grooveboxes of this family ship (bass, keys, leads, pads, effects and drum kits; no list of any other maker's factory sounds was copied, and the names are our own). The tests check that every one plays, is not silent or clipped, differs from the other seven, and that the names are true (deep is lower than click, boom rings longer than tight, metal is brighter than sub, gong outlasts muted). **They are a first set to be tuned by ear on the unit**: keep the ones that sound right, change the rest by saving yours in BROWSER and sending them over.

| Instrument | The eight presets |
| :-- | :-- |
| KickSyn | deep, punch, boom, tight, click, dirty, soft, hard |
| FmBass | sub, round, growl, metal, pluck, slide, acid, fat |
| resonator | nylon, steel, harp, muted, bell, glass, gong, chime |
| drums | tight, boomy, bright, dull, snappy, loose, lofi, hard (applied to the sound that is chosen) |
| sampler | pluck, bell, pad, voice, thump, sweep, glass, tick (they choose one of the sounds that come with it and how it is cut) |
| macro | lead, strings, chip, organ, chords, swarm, pluck, modal (each on its own engine) |
| midi out | none: its knobs are channel, transpose, velocity and `cc74` |

## 9. The eight pieces that come with it

The factory projects (FILE, the top two rows) are **eight short pieces in eight styles**, so that the first thing anyone hears is what the machine can do. They use the factory instruments with their presets ([§8](#8-presets-that-come-with-the-instruments)), the sampler, the track effects, the LFO, the Brain, players and step components, and each is a different tempo, feel and mix of tracks. Their levels are brought to about the same loudness (a test measures it). **Like the presets they were made by measurement and are a first set**: keep the ones that move you and replace the rest.

| # | Piece | Tempo | What it shows |
| :-- | :-- | :-- | :-- |
| 1 | **house** | 124 | A kick on every beat, offbeat open hats and a clap, a round bass between the kicks, chord stabs that follow the Brain with an echo, a pad in a room |
| 2 | **boom bap** | 90, heavy swing | A lazy kit, a sub bass under a soft kick, a voice sample, seventh chords on the off beats |
| 3 | **dub** | 70 | One deep kick, a sub bass whose tone breathes (an LFO), bells thrown into an echo, a pad in a big room |
| 4 | **polyrhythm** | 96 | Five tracks of 3, 5, 7, 9 and 11 steps that never line up, over a slow chord |
| 5 | **acid** | 130 | A bass line that opens and closes by itself (an LFO) through a drive, hats, a clap, ratchets |
| 6 | **electro** | 110 | A dry kit, a hard bass, an arpeggio that runs over the chords with an echo |
| 7 | **ambient** | 60 | Nothing hits: a glass line, strings with a chorus and a sampled pad that holds through a loop and follows the chords |
| 8 | **breaks** | 140 | A kit that never plays the same bar twice (chance, ratchets, "every"), a growling bass, random notes and glitches |

An empty place of the player's in FILE opens a **new empty project** (the standard instruments, nothing in the steps); SHIFT + tap saves it there.

## 10. Credits and where each instrument comes from

| Instrument | Written for OP-8 or built on | Licence |
| :-- | :-- | :-- |
| drums | The percussion models of Plaits (kick, snare, hi-hat, noise, modal); the lanes, per-sound settings and drawing are OP-8's | MIT (Plaits, `crates/op8-plaits`) |
| KickSyn | Written for OP-8 (a sine with an exponential pitch sweep, a decay, a noise click and a tanh drive) | OP-8's |
| FmBass | Written for OP-8 (two-operator FM with a sub-octave) | OP-8's |
| resonator | The string and modal engines of Plaits | MIT |
| sampler | Written for OP-8 | OP-8's |
| macro | The engines of Plaits (24 in the vendored set), with OP-8's own pages and drawing | MIT |
| midi out | Written for OP-8 | OP-8's |
| granular | The granular player of Clouds (Mutable Instruments), rewritten in Rust for a stored sound: how grains are scheduled (regular or random, with a dead zone), the window of a grain, the cut-down of a grain that would run past the end, the gain that follows how many sound, and the interpolation that gets cheaper as grains get busy; the bank of sounds (shared with the sampler), the pages, the drawing and the pitch from held notes are OP-8's | MIT (Clouds; notice in `crates/op8-cartridges/src/granular.rs`) |

Plaits is by Émilie Gillet (Mutable Instruments). The name "Mutable Instruments" is a registered trademark: it appears only in credits, never as the name of an instrument. The manual says all of this in plain words under "What each one is for, and where it comes from" ([the manual](../guide/manual.tpl.html) is generated into the site).

