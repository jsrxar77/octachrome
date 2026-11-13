# OP-3 — Architecture

How the system is divided and how everything goes in and out. Read this first, then `CLAUDE.md`.

## What OP-3 is

A standalone groovebox in Rust. It runs on a **Raspberry Pi 4/5** and is developed on **macOS Apple Silicon**. The only control is a **Native Instruments Maschine MK3**: pads, 8 knobs, Smart Strip, LEDs and two 480×272 screens. The product owner is **Javi**, who writes in Rioplatense Spanish; answer in that language.

The model is Teenage Engineering's OP-XY: a sequencer with polymeter, *Step Components*, a harmony track (*Brain*) and *Punch-in FX*. The centre is something else: **instruments are the product**. The sequencer and the buttons stay small and fixed; what grows is the collection of instruments ("cartridges"), each with few knobs and a drawing that explains the sound.

## Documents

These six documents are the only ones in `docs/`. Each fact lives in exactly one of them. The skill `op3-docs` (`.claude/skills/op3-docs/`) maintains them.

| Document | What it decides |
| :-- | :-- |
| `architecture.md` (this one) | How the system is built, how to build and test it, decisions taken, how to work |
| [design.md](design.md) | UX/UI treatise: the 13 rules, colours, typeface and layout. **If the code disagrees, the code is wrong** |
| [buttons.md](buttons.md) | What each MK3 button does (Maschine → OP-XY → OP-3) and its status ✓ ◐ ○ |
| [instruments.md](instruments.md) | What an instrument is, its rules (max 8 parameters, one drawing), the extension mechanism and the candidates |
| [features.md](features.md) | The verifiable behaviours with their IDs, and the test case that checks each |
| [roadmap.md](roadmap.md) | Status, hardware status and the order of work |

The interactive twin lives outside `docs/`: `twin/op3-twin.html` (HTML/WebAudio). Every UI change is tried there first.

## 1. Central idea

There are three worlds that do not know each other:

- **Hardware or simulator** (`op3-hw`): emits `ControlEvent` and receives LEDs and screens. It knows nothing about music.
- **Control and UI** (`op3::controller`, `op3-ui`): turns events into commands and draws. It does not touch audio.
- **Audio engine** (`op3-engine`, `op3-seq`, cartridges): receives commands, produces sound and MIDI. It knows nothing about screens or the MK3.

Communication between the control thread and the audio thread is **lock-free** (`rtrb`) in both directions, plus atomics for the state that audio publishes (playhead, peaks, active chord, dropped events).

```text
 real MK3 ─┐                                  ┌─ bulk (screens, from their own thread)
           ├─ ControlSurface ── App::frame ───┤
 Mock/sim ─┘   (poll)           │             └─ HID (LEDs)
                                ▼
                       Controller::handle  ──EngineCommand──▶ rtrb ──▶ Engine::render (audio)
                                │                                       │
                       op3-ui (draws)                         atomic status, MIDI rtrb
                                                                        │
                                                    midi thread / cpal ◀┘
```

## 2. Crates

| Crate | Responsibility | Depends on |
|---|---|---|
| `op3-gfx` | RGB565 480×272 framebuffer, anti-aliased primitives, Jost typeface, themes, dirty rectangles | – |
| `op3-cartridge` | The `InstrumentCartridge` contract (`process`, `parameters`, `secondary_parameters`, `draw_ui`, `save_state`/`load_state`…) | gfx |
| `op3-seq` | Deterministic sequencer: 96 PPQN, polymeter, Step Components, Brain, `Command` | cartridge |
| `op3-engine` | Audio thread: sequencer, 8 slots, mix, Punch-in FX, Smart Strip filter, `EngineStatus`. Lock-free with `rtrb` | seq, cartridge |
| `op3-plaits` | The macro oscillator from Plaits in C++, vendored **unchanged** (`vendor/`, eurorack f4cfa76 and stmlib d18def8), built with `cc` (`build.rs`, `-DTEST -include cstdio`) behind a C shim (`src/shim.cc`). 24 engines | – |
| `op3-cartridges` | KickSyn, FmBass, MidiOutBridge and **macro** (`macro_osc.rs`). The list is in `builtin_names()` / `create()` of `lib.rs`, which also holds the contract tests every instrument must pass | cartridge, plaits |
| `op3-hw` | `ControlSurface`, `ControlEvent`, `Button`, `LedFrame`, `MockSurface`, the MK3 protocol as pure functions, USB transport (feature `mk3`: hidapi or hidraw, plus nusb) | gfx |
| `op3-ui` | Left and right screen layouts, `UiState`, `Mode`, footer and touch overlay, animation | gfx, seq, engine |
| `op3` | Binary and library: `App`, `Controller`, `demo`, `script`, `bench`, cpal audio, MIDI, `sim`, `hw-test`, `usb-reset` | everything |

Rule: the sequencer and the cartridges do not depend on any driver. Hardware enters only through `op3-hw` and the binary.

## 3. Flow of a frame (60 fps)

`App::frame(now_ms)` in `crates/op3/src/app.rs`:

1. `surface.poll()` brings the events from the hardware or the mock.
2. Injected events (`App::inject`), which come from a script, are added.
3. `Controller::handle` translates each event: it changes `UiState` and sends `EngineCommand` / `Command` to the engine.
4. `Controller::tick` resolves what depends on time (for example, a pad held 300 ms → inspector).
5. `op3-ui` draws the two screens into framebuffers.
6. `DirtyTracker` detects the changed rectangles and only those are reported to the surface. On the real MK3 the surface does not use them: it copies the whole frame and returns at once (see §7).
7. `Controller::leds` builds the complete `LedFrame` and it is written.

Audio runs separately, in blocks of 100 to 128 samples at 48 kHz. For each UI frame there are 800 audio samples.

## 4. Audio engine

- `Engine::render` runs the sequencer (`Sequencer::process`), which emits `NoteEvent`s per track. Each track has a cartridge in its slot.
- The mix goes through Punch-in FX and the Smart Strip filter and comes out in stereo.
- Audio publishes to `EngineStatus` (`playing`, `playhead[t]`, `peak[t]`, `note_count[t]`, `chord_*`, `dropped_events`).
- MIDI cartridges (`MidiOutBridge`) push to an `rtrb<MidiMessage>` queue consumed by a separate thread (`midi.rs`) or by the test bench.
- **Forbidden on the audio thread:** allocating memory. Queues are fixed-size.
- The sequencer is deterministic: randomness uses a per-track seeded PRNG that is re-seeded on every Play. Same project, same output, to the sample.

## 5. Control

`Controller` (`crates/op3/src/controller.rs`) holds `UiState` (mode, selected track, open inspector, right-screen page, active punch, live chord) and knob counters. It is the only one that decides what an event means. Its mode table is in its header.

Modes: `Step`, `Brain`, `Punch`. They change with STEP, CHORDS and PERFORM, and the pads mean different things in each.

## 6. Instruments and their contract

Each instrument implements `InstrumentCartridge`. The mandatory methods are `process`, `parameters`, `secondary_parameters`, `draw_ui`, `save_state` and `load_state`; the optional ones have defaults. Limit: **at most 8 parameters** (4 of sound, 4 of shape). The contract tests every instrument must pass are in `crates/op3-cartridges/src/lib.rs`. The rules for an instrument are in [instruments.md](instruments.md).

## 7. Screens and LEDs

- Left: time (tempo, chord, polymeter rings, pad mirror, knobs 1–4).
- Right: the track's instrument (knobs 5–8, pages with buttons 5 and 6).
- Theme: `op-1` (default), `op-1-field`, `op-xy`. Chosen with `-t` or `OP3_THEME`.
- LEDs: 103 slots (62 buttons, 25 of the strip, 16 pads). The pads are in HID order (top-left = 0), but the pad number the user sees is bottom-left first. See `leds.rs`.

### How the screens reach the MK3

A transfer to a screen takes milliseconds, and both screens animate. If the control loop made the transfer, the drawing would lag and the two screens would update at different moments. So the sending lives in a thread of its own (`crates/op3-hw/src/mk3/display.rs`):

- **The control loop hands over and moves on.** `write_screen` copies the finished frame into a shared buffer and returns. If more frames arrive than the device accepts, the thread sends the latest: they do not pile up.
- **One band of rows per screen.** The thread compares the frame with what it last sent and sends only from the first to the last row that changed, **always full width** (x = 0, width 480). That is the only transfer shape the projects that already work with this screen use; rectangles with arbitrary x or width left cut stripes.
- **Compression.** Besides literal pixels (`00`), the protocol accepts the `01` command (one pixel pair repeated N times). The UI is mostly black, so a full screen drops from 261 KB to about 52 KB and a ~30 ms transfer becomes ~3 ms. The format is in `protocol::encode_band`.
- **Full refresh every 500 ms.** Pixels that do not change (titles, labels) are never sent again; if the device dropped or drew a transfer wrongly they would stay wrong forever. The refresh repairs them by itself.
- **Errors.** If a transfer fails the thread keeps the error and ends; the next `write_screen` returns it and `op3 run` reconnects, releasing the screens first.
- `OP3_DISPLAY_STATS=1` prints once a second how many transfers, how many KB and how many ms.

Measured on the MK3 behind a USB 2.0 hub, with the demo: ~64 transfers per second, ~1.1 MB/s, ~3.4 ms each; at rest, 4 per second (only the refresh).

The protocol data (commands `00` literal, `01` repeat, `02` skip, `03` show, `40` end) come from public reverse-engineering notes; the code is our own. No code from Deckard (GPL-2.0) is copied.

## 8. Simulator and scripts

`op3 sim` and `op3 run --script` use the same `App` and the same engine. The only thing that changes is the surface:

- `op3 sim`: `MockSurface` (in memory), no hardware.
- `op3 run`: the real MK3, with cpal audio and MIDI.

A **script** (`crates/op3/src/script.rs`) is a text file with one action per line (`<seconds> <command>`); the demo is `crates/op3/scripts/demo.op3` and the hardware battery is `bateria-hw.op3`. `Script::parse` validates everything on load and gives the line number if there is an error. `Script::due(now_ms)` returns the actions that have come due. Commands: `btn`, `pad`, `knob`, `touch`, `encoder`, `strip`, `shot`.

## 9. Test bench

It is the way to test the whole system without hardware. It has three parts:

| Part | Where | What it does |
|---|---|---|
| **Bench** (`Bench`) | `crates/op3/src/bench.rs` | Assembles engine + `App` + `MockSurface`. Advances time like the real program (60 fps UI, audio blocks). Records audio, timed MIDI, LEDs and screens. Exposes helpers (`press`, `tap`, `hold`, `knob`, `send`) and measures (`rms_db`, `peak_db`, `silent`, `lit_pixels`, `screen_hash`). |
| **Cases** | `crates/op3/tests/*.rs` | One file per area of [features.md](features.md). Each `#[test]` carries the ID of its feature (`// F-S01`). |
| **Evidence** | `$OP3_BENCH_DIR` | When run with `OP3_BENCH_DIR=…`, each case that asks for it saves PNG screenshots and WAV files for review by hand. |

Principles:

- Each case checks **one behaviour** and asserts it with a number or a concrete state, not with "looks fine".
- Cases are deterministic: the same script gives the same audio (a case checks it).
- Cases do not touch hardware. Testing with the real MK3 stays in `hw-test` and `run --script`, which are manual.
- What the bench cannot judge (whether a sound is pleasant, whether a screen is legible) is marked **manual review** in [features.md](features.md).

Cargo runs each crate's tests from the crate's folder, so give `OP3_BENCH_DIR` as an absolute path.

## 10. Decisions taken (do not reopen without Javi) and known limits

**Decisions**

- **Licence MIT or Apache-2.0.** No GPL dependencies are added. Encdr, autotel/maschine-mk3-linux and Deckard are GPL: from the MK3 protocol only the facts were taken, not the code. Surge XT and Vital are GPL and stay out unless Javi decides otherwise.
- **Mutable Instruments synthesis engines (MIT).** Javi chose to start there. The code can be used, but **without the module names**, because they are trademarks: the Plaits one is called "macro", the Rings one will be called "resonator", and so on. Credit goes in the vendor README.
- **Aesthetic:** the language of the OP-1 and OP-XY (colour tied to each physical control, drawings instead of text, absolute black). We take the ideas, never their graphics or fonts. It must not look like a programming project. Details in [design.md](design.md).
- **8 knobs, 8 colours.** Knobs 1–4 drive the sequencer and appear on the left screen; 5–8 drive the instrument and appear on the right. Three themes: op-1 (default), op-1 field, op-xy.
- **Typeface: Jost** (OFL), weights 300 and 400, tuned by eye to resemble the OP-XY's. FlipMono cannot be used: it is actually SF Mono, licensed only for Apple devices.
- **Instruments:** at most 8 parameters, 4 of sound (button 5) and 4 of shape (button 6). Effects and modulation are per track and the same for all instruments.

**Known limits**

- The real MK3 has 4 audio channels; the engine produces stereo and it is copied to 1/2 and 3/4.
- On macOS, `NIHardwareAgent` and NI's other services take screen interface 5. If they are running, OP-3 warns `screens unavailable` and carries on without screens.
- The MK3 behind a USB 2.0 hub receives 480 mA. Under heavy load or with the device in a bad state it resets. See `OP3_BRIGHTNESS`, and `op3 usb-reset` ([roadmap.md](roadmap.md), hardware status).
- The LED palette is only confirmed for red and orange. The rest is unverified on a unit.

## 11. Build, run and test

A stable Rust and a C++ compiler are needed. On Mac: `xcode-select --install`. On Linux or the Pi: `build-essential` and `libasound2-dev`.

```sh
cargo test --workspace                       # everything, without hardware
cargo clippy --workspace --all-targets
cargo run --release -- sim -s 8 -o out       # simulated MK3: out.wav + .ppm captures
cargo run --release -- sim --script f.op3    # your own script: wav + captures + loudness table
cargo run --release -- run --script f.op3    # the script plays the real MK3
cargo run --release -- hw-test               # test frame and rainbow, with the real MK3
cargo run --release -- usb-reset             # reset a stuck MK3 (with op3 closed)
cargo run --release [-- -t op-xy]            # play
open twin/op3-twin.html                      # the Twin
```

Environment: `OP3_THEME`, `OP3_BRIGHTNESS` (0–100), `OP3_DISPLAY_STATS=1`, `OP3_DUMP_DIR=<dir>` (writes `left.png` and `right.png`, up to four times a second), `OP3_BENCH_DIR=<dir>`.

CI (`.github/workflows/ci.yml`) runs on Linux (tests, clippy, the simulator and `cargo check` for aarch64, with `g++-aarch64-linux-gnu` because op3-plaits compiles C++) and on macOS. For the Pi there are `scripts/build-pi.sh` and `deploy/` (systemd, udev, setup).

## 12. How to work here

- A UI change is tried first in `twin/op3-twin.html` and [design.md](design.md) is updated in the same PR. A change to the treatise is approved by Javi.
- When something changes, update [roadmap.md](roadmap.md), [buttons.md](buttons.md), [instruments.md](instruments.md) or [features.md](features.md) as appropriate.
- On screen everything is lowercase and without codes or IDs (rule 8). Each knob paints only what it moves, in its colour (rule 1).
- No memory is allocated on the audio thread. Instruments must pass the contract tests in `op3-cartridges/src/lib.rs`.
- Tests that touch the global theme use `THEME_LOCK` (`op3-ui`) so they do not fail when run in parallel.
- To vendor third-party code: copy it unchanged, with its licence and a README of origin, as in `crates/op3-plaits/vendor/`.
- Both screens animate. Never assume either one is still.
