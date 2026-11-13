# OP-3 — Instruments

**Instruments are the centre of OP-3.** The sequencer, the Brain, Punch-in and the buttons always do the same and stay small. What grows is the collection of instruments ("cartridges"): it starts with a small set, anyone can add a new one, and each is used as quickly as on an OP-XY.

This document rules together with the [UX/UI Treatise](design.md) and the [button map](buttons.md).

## 1. What an instrument is

An instrument is a piece of code that fulfils the `InstrumentCartridge` contract (`crates/op3-cartridge`). It has four parts, always the same:

| Part | What it is | Limit |
| :-- | :-- | :-- |
| **Sound** | Receives notes, returns stereo audio at 48 kHz | Must fit the CPU budget of the Pi (section 5) |
| **Parameters** | 4 of sound (button 5, knobs 5–8) and, if needed, 4 of shape (button 6) | **At most 8.** If the engine has more, the instrument picks the 8 that matter |
| **Drawing** | An animated graphic that shows what the sound does, in the central area of the right screen | Uses the colours of knobs 5–8 for what those knobs move |
| **State** | Save and load its values (presets and parameter locks) | A list of numbers, versioned |

The track's effect and modulation (buttons 7 and 8) are not the instrument's: they are the same for all, which keeps each instrument simple.

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
| **1. In code** | A Rust crate that implements `InstrumentCartridge` and is registered in `builtin_names()` / `create()` (`crates/op3-cartridges/src/lib.rs`). Compiled with OP-3. It can wrap MIT-licensed C/C++ code (like macro) | Us and anyone who knows Rust | ◐ (contract and list done; choosing them with PLUG-IN is missing) |
| **2. Separate package** | The instrument is compiled as a library (`.so` on the Pi, `.dylib` on Mac) and copied to a `cartridges/` folder. OP-3 loads it at start-up | Anyone who programs, without touching OP-3 | ○ |
| **3. Without compiling** | Instruments described in a patch language (for example Faust or a format of our own) that OP-3 converts | People who design sound but do not program | ○ (to investigate) |

**Drawing kit:** `op3-gfx` already has what the drawings need (smoothed circles, arcs, lines and boxes, the Jost typeface, theme and per-knob colours). Later a small library of reusable drawings is added: oscilloscope, envelope, polar waveform, keyboard, waves. That way a new instrument looks good without designing everything from scratch.

## 4. Initial set and candidates

### Already done

| Instrument | Idea | Drawing |
| :-- | :-- | :-- |
| **KickSyn** | Synthetic kick: tone, decay, punch, saturation | A drum head seen from above |
| **FmBass** | Two-operator FM bass | A planet and moon in orbit, the envelope as a mountain |
| **MidiOutBridge** | Sends the track out the MK3's DIN MIDI to an external synth | A DIN connector and notes leaving through the cable |
| **macro** | 24 engines in one (virtual analogue, FM, wavetable, chords, voice, strings, percussion), from the Plaits code (MIT, `crates/op3-plaits`). Sound: `engine`, `harmonics`, `timbre`, `morph`. Shape: `decay`, `color`, `variant`, `level` | A ring of 24 engines, a polar waveform that beats with the level, and a timbre × morph field |

macro is monophonic and uses gate semantics (the 6-operator FM engines need the held gate). It goes to sleep and gives exact zeros when it is not sounding.

### Candidates (known engines that run on a Pi)

| Source | What it brings | Licence | May it be included? |
| :-- | :-- | :-- | :-- |
| **Plaits** (Mutable Instruments) | 16 synthesis engines in one: virtual analogue, FM, wavetable, chords, voice, strings, percussion. Already designed for 3–4 knobs | MIT | **Yes.** There are already partial Rust ports (for example the DX7 engine) |
| **Braids** (Mutable Instruments) | More than 40 oscillators on a single "timbre/colour" control | MIT | **Yes** |
| **Rings / Elements** (Mutable Instruments) | Physical resonators: strings, bells, tubes | MIT | **Yes** |
| **Clouds / Beads** (Mutable Instruments) | Granular on a sample | MIT | **Yes** (useful for the future sampler) |
| **Surge XT** | A huge synth with very many oscillators and filters | GPL-3.0 | **Only if OP-3 becomes GPL**, or if only its ideas are used, without its code |
| **Vital** | Wavetable | GPL-3.0 | Same as Surge |

Legal note on Mutable Instruments: the code of the STM32 modules (Plaits, Braids, Rings, Elements, Clouds, Beads) is MIT, but the name "Mutable Instruments" is a registered trademark and its author asks that the module names not be used in derived works. OP-3 instruments based on that code carry names of their own (for example "macro", "resonator") and say in the credits where they come from.

### Proposed order

1. ~~**macro**~~ (from Plaits): done.
2. **resonator** (from Rings): strings and bells.
3. **drums**: a drum instrument with 8 sounds on pads 9–16 (PAD MODE).
4. **sampler**: record with SAMPLING and play with the keyboard.

## 5. Budget on the Raspberry Pi 5

It has to be measured on the real Pi, but the target is: 8 tracks sounding at once, 128-sample buffers (2.7 ms), and under 60 % of one core for all the audio. Each instrument declares how many voices it uses and OP-3 warns if a combination does not fit. The CPU on the Pi 5 has not been measured yet.

## 6. What OP-3 is not going to be

- A DAW or a VST plug-in host: instruments are few parameters and a drawing, not enormous panels.
- A box of thousands of parameters: if something needs more than 8 knobs, it is two instruments.
