# OP-8 — Architecture

How the system is divided and how everything goes in and out. Read this first, then `CLAUDE.md`.

## What OP-8 is

A standalone groovebox in Rust. It runs on a **Raspberry Pi 4/5** and is developed on **macOS Apple Silicon**. The only control is a **Native Instruments Maschine MK3**: pads, 8 knobs, Smart Strip, LEDs and two 480×272 screens. The product owner is **Javi**, who writes in Rioplatense Spanish; answer in that language.

The model is Teenage Engineering's OP-XY: a sequencer with polymeter, *Step Components*, a harmony track (*Brain*) and *Punch-in FX*. The centre is something else: **instruments are the product**. The sequencer and the buttons stay small and fixed; what grows is the collection of instruments ("cartridges"), each with few knobs and a drawing that explains the sound.

## Documents

These six documents are the only ones in `docs/`. Each fact lives in exactly one of them. The skill `op8-docs` (`.claude/skills/op8-docs/`) maintains them.

| Document | What it decides |
| :-- | :-- |
| `architecture.md` (this one) | How the system is built, how to build and test it, decisions taken, how to work |
| [design.md](design.md) | UX/UI treatise: the 13 rules, colours, typeface and layout. **If the code disagrees, the code is wrong** |
| [buttons.md](buttons.md) | What each MK3 button does (Maschine → OP-XY → OP-8) and its status ✓ ◐ ○ |
| [instruments.md](instruments.md) | What an instrument is, its rules (max 8 parameters, one drawing), the extension mechanism and the candidates |
| [features.md](features.md) | The verifiable behaviours with their IDs, and the test case that checks each |
| [roadmap.md](roadmap.md) | Status, hardware status and the order of work |

The interactive twin lives outside `docs/`: `twin/op8-twin.html` (HTML/WebAudio). Every UI change is tried there first.

**The public site.** `guide/` holds the sources of a small static site for people who want to know what OP-8 is and how it is used: the home page (`index.tpl.html`: what OP-8 is with its real screens, the three things that make it different, what you need, where it stands (the counts come from [buttons.md](buttons.md)), and then the interactive parts: the map of the unit, a beat to click, what the pads become, the three themes, the instruments, how to start it), the manual (`manual.tpl.html`: a numbered map of the unit, the ideas, seventeen recipes with pictures, the instruments, where OP-8 keeps the player's files, every button with its status, what to do when something is odd, and what is missing) and a 404 page; both pages are drawn in the matte theme (the default of the machine and the twin too) and share a header, a footer and `site.css`. `python3 scripts/build-site.py` assembles them into `site/` (ignored by git) together with the fonts, the pictures, the brand files, the twin (`twin/op8-twin.html`, linked as "Try it") and generated `robots.txt`, `sitemap.xml` and social-sharing tags; the manual's list of buttons is generated from [buttons.md](buttons.md) there, so it never drifts. `python3 scripts/check-site.py` checks the result like a visitor would (F-G05, also run by `cargo test`): no broken link, anchor or image, a title, a description and one heading on every page. Everything is self-contained (its own Jost fonts, no network at run time). The **placeholders** for the day it is published are in `guide/site.json` (the address of the site and of the repository) and are overridden by `OP8_SITE_URL` and `OP8_REPO_URL`; the check warns while they are still placeholders. `.github/workflows/pages.yml` builds, checks and publishes `site/` to GitHub Pages once Pages is enabled with source "GitHub Actions" (a free plan needs a public repository) and sets the two addresses (the repository variable `OP8_REPO_URL` is the one to fill in). `guide/brand.html` shows the brand mark ([design.md §2.3](design.md)); it stays apart from the site: not in the menu, not in the sitemap, and marked `noindex`. `guide/themes.js` is the guide's copy of the three themes (keep it in step with `theme.rs`), shared by the pages through `localStorage`; `guide/mk3.js` holds the geometry of the unit that every picture takes ([design.md §2.4](design.md)). The recipes are tests (`crates/op8/tests/manual.rs`, F-G04) and `scripts/manual-pictures.sh` takes the pictures from them into `guide/img/` (run it after changing a screen).

## 1. Central idea

There are three worlds that do not know each other:

- **Hardware or simulator** (`op8-hw`): emits `ControlEvent` and receives LEDs and screens. It knows nothing about music.
- **Control and UI** (`op8::controller`, `op8-ui`): turns events into commands and draws. It does not touch audio.
- **Audio engine** (`op8-engine`, `op8-seq`, cartridges): receives commands, produces sound and MIDI. It knows nothing about screens or the MK3.

Communication between the control thread and the audio thread is **lock-free** (`rtrb`) in both directions, plus atomics for the state that audio publishes (playhead, peaks, active chord, dropped events).

```text
 real MK3 ─┐                                  ┌─ bulk (screens, from their own thread)
           ├─ ControlSurface ── App::frame ───┤
 Mock/sim ─┘   (poll)           │             └─ HID (LEDs)
                                ▼
                       Controller::handle  ──EngineCommand──▶ rtrb ──▶ Engine::render (audio)
                                │                                       │
                       op8-ui (draws)                         atomic status, MIDI rtrb
                                                                        │
                                                    midi thread / cpal ◀┘
```

## 2. Crates

| Crate | Responsibility | Depends on |
|---|---|---|
| `op8-gfx` | RGB565 480×272 framebuffer, anti-aliased primitives, Jost typeface, themes, dirty rectangles | – |
| `op8-cartridge` | The `InstrumentCartridge` contract (`process`, `parameters`, `secondary_parameters`, `draw_ui`, `save_state`/`load_state`…) | gfx |
| `op8-seq` | Deterministic sequencer: 96 PPQN, polymeter, Step Components, Brain, `Command` | cartridge |
| `op8-engine` | Audio thread: sequencer, 8 slots, mix, master volume, metronome click, Punch-in FX, Smart Strip filter, `EngineStatus`. Lock-free with `rtrb` | seq, cartridge |
| `op8-plaits` | The macro oscillator from Plaits in C++, vendored **unchanged** (`vendor/`, eurorack f4cfa76 and stmlib d18def8), built with `cc` (`build.rs`, `-DTEST -include cstdio`) behind a C shim (`src/shim.cc`). 24 engines | – |
| `op8-cartridges` | KickSyn, FmBass, MidiOutBridge, **macro** (`macro_osc.rs`) and **drums** (`drums.rs`) and **resonator** (`resonator.rs`). The list is in `builtin_names()` / `create()` of `lib.rs`, which also holds the contract tests every instrument must pass | cartridge, plaits |
| `op8-hw` | `ControlSurface`, `ControlEvent`, `Button`, `LedFrame`, `MockSurface`, the MK3 protocol as pure functions, USB transport (feature `mk3`: hidapi or hidraw, plus nusb) | gfx |
| `op8-ui` | Left and right screen layouts, `UiState`, `Mode`, footer and touch overlay, animation | gfx, seq, engine |
| `op8` | Binary and library: `App`, `Controller`, `demo`, `script`, `bench`, `settings`, cpal audio (with its keeper), MIDI, `sim`, `hw-test`, `usb-reset` | everything |

Rule: the sequencer and the cartridges do not depend on any driver. Hardware enters only through `op8-hw` and the binary.

## 3. Flow of a frame (60 fps)

`App::frame(now_ms)` in `crates/op8/src/app.rs`:

1. `surface.poll()` brings the events from the hardware or the mock.
2. Injected events (`App::inject`), which come from a script, are added.
3. `Controller::handle` translates each event: it changes `UiState` and sends `EngineCommand` / `Command` to the engine.
4. `Controller::tick` resolves what depends on time (for example, a pad held 300 ms → inspector).
5. `op8-ui` draws the two screens into framebuffers.
6. `DirtyTracker` detects the changed rectangles and only those are reported to the surface. On the real MK3 the surface does not use them: it copies the whole frame and returns at once (see §7).
7. `Controller::leds` builds the complete `LedFrame` and it is written.

Audio runs separately, in blocks of 100 to 128 samples at 48 kHz. For each UI frame there are 800 audio samples.

## 4. Audio engine

- `Engine::render` runs the sequencer (`Sequencer::process`), which emits `NoteEvent`s per track. Each track has a cartridge in its slot.
- Each track's audio goes through the track's own **effect** (`crates/op8-engine/src/track_fx.rs`: delay, reverb, chorus, filter or drive, with three knobs; its buffers are allocated once when the engine starts, so the audio thread never allocates) before it is mixed, and the track's **LFO** moves one knob of its instrument (or its volume) once per block, around the value the player left it. Both live in the `Track` of the project (`op8_seq::rack`), so they are saved with it and edited with `Command::SetFx` and `Command::SetLfo`. The **players** (an arpeggio or a chord over each step's note) are applied by the sequencer, which turns one step into several notes (`Command::SetPlayers`).
- The mix goes through Punch-in FX and the Smart Strip filter and comes out in stereo.
- **Recording (SAMPLING).** `EngineCommand::Record(Some(RecSource))` copies, block by block, either the final mix (`Resample`) or the audio that comes in (`Input`, mono, from an `rtrb` ring that `AudioIn` in `audio.rs` fills from a cpal input stream, opened only while SAMPLING is on the input) into `EngineStatus::take`, a ten-second buffer made once when the engine is built; the engine stops by itself when it is full, and the control thread reads `take` only when nothing records. `EngineCommand::Preview` plays a part of the take to the output without allocating.
- Audio publishes to `EngineStatus` (`playing`, `playhead[t]`, `peak[t]`, `note_count[t]`, `chord_*`, `dropped_events`).
- MIDI cartridges (`MidiOutBridge`) push to an `rtrb<MidiMessage>` queue consumed by a separate thread (`midi.rs`) or by the test bench.
- **Forbidden on the audio thread:** allocating memory. Queues are fixed-size.
- The sequencer is deterministic: randomness uses a per-track seeded PRNG that is re-seeded on every Play. Same project, same output, to the sample.
- **The sequencer's clock runs in the audio callback.** If the audio device goes away (the MK3 resets, the cable is pulled) the clock stops with it: PLAY lights up and nothing moves. So the engine does not live inside the stream. `Audio` (`crates/op8/src/audio.rs`) owns it behind a mutex that only the callback locks, and its `Keeper` reopens the stream whenever the error callback reports a failure, retrying about once a second while the device is missing. `op8 run` calls `keep_alive()` every frame, which also covers starting while the device is not there yet.

## 5. Control

`Controller` (`crates/op8/src/controller.rs`) holds `UiState` (mode, selected track, open inspector, right-screen page, active punch, live chord) and knob counters. It is the only one that decides what an event means. Its mode table is in its header.

Main modes: `Instrument` (composing), `Settings`, `File` (projects), `Plugin` (instruments), `Sampling` (recording a sound) and `Browser` (presets); all but the first take both screens and use the pads to choose. Pad modes: `Step`, `Brain`, `Punch`, `Pads` and `Keys`. They change with STEP, CHORDS, PERFORM, PAD MODE and KEYBOARD (`EngineCommand::LiveNote` plays a pad's note by hand), and the pads mean different things in each.

Above them sit the **main modes** (`Main` in `op8-ui`, the "main modes" table of [buttons.md](buttons.md)): `Instrument` for composing (CHANNEL), and `Settings` (SETTINGS), which takes both screens. In `Settings` the controller ignores pads, knobs and groups, turns the 4D encoder and ◀ ▶ into theme changes, and lets PLAY, STOP and RESTART through. Entering it lets go of anything a pad is holding (an engaged effect, a live chord). A pad mode button always returns to `Instrument`. A theme change sets the global theme at once and raises `Controller::theme_changed`; `App` saves it to the settings file if it has a path (`App::settings_path`), so the simulator and the test bench never write to disk.

## 6. Instruments and their contract

Each instrument implements `InstrumentCartridge`. The mandatory methods are `process`, `parameters`, `secondary_parameters`, `draw_ui`, `save_state` and `load_state`; the optional ones have defaults. Limit: **at most 8 parameters** (4 of sound, 4 of shape). The contract tests every instrument must pass are in `crates/op8-cartridges/src/lib.rs`. The rules for an instrument are in [instruments.md](instruments.md).

## 7. Screens and LEDs

- Left: time (tempo, chord, polymeter rings, pad mirror, knobs 1–4).
- Right: the track's instrument (knobs 5–8, pages with buttons 5 (`sound`) and 6 (`shape`)), and the track's effect and LFO (buttons 7 and 8).
- Theme: `matte` (default, and the first of the list), `glossy`, `chrome`. Chosen with `-t` or `OP8_THEME`.
- LEDs: 103 slots (62 buttons, 25 of the strip, 16 pads). The pads are in HID order (top-left = 0), but the pad number the user sees is bottom-left first. See `leds.rs`.

### How the screens reach the MK3

A transfer to a screen takes milliseconds, and both screens animate. If the control loop made the transfer, the drawing would lag and the two screens would update at different moments. So the sending lives in a thread of its own (`crates/op8-hw/src/mk3/display.rs`):

- **The control loop hands over and moves on.** `write_screen` copies the finished frame into a shared buffer and returns. If more frames arrive than the device accepts, the thread sends the latest: they do not pile up.
- **One band of rows per screen.** The thread compares the frame with what it last sent and sends only from the first to the last row that changed, **always full width** (x = 0, width 480). That is the only transfer shape the projects that already work with this screen use; rectangles with arbitrary x or width left cut stripes.
- **Compression.** Besides literal pixels (`00`), the protocol accepts the `01` command (one pixel pair repeated N times). The UI is mostly black, so a full screen drops from 261 KB to about 52 KB and a ~30 ms transfer becomes ~3 ms. The format is in `protocol::encode_band`.
- **Full refresh every 500 ms.** Pixels that do not change (titles, labels) are never sent again; if the device dropped or drew a transfer wrongly they would stay wrong forever. The refresh repairs them by itself.
- **Optional limits, off by default.** A token bucket (`Pacer`, `OP8_DISPLAY_KBPS=<n>`) caps the bytes per second, `OP8_DISPLAY_FPS=<n>` caps the rounds per second, and `OP8_DISPLAY_DELTA=1` skips unchanged pixel pairs with the `02` command (a handful of changed pixels then cost under 600 bytes, and the traffic drops about fourfold). They were built when the resets looked like a bandwidth limit. They are not: the cause was the backlight (§10), and at a safe brightness the MK3 took whole rows at 3.4 MB/s and 90 transfers/s for 50 s. Delta has not been looked at on the real screens, which is why it is off.
- **Errors.** If a transfer fails the thread keeps the error, with how many KB it had sent in the last second, and ends; the next `write_screen` returns it and `op8 run` reconnects, releasing the screens first.
- **Reconnecting waits.** A MK3 that has just reset needs a few seconds; opening it at once made it fail again, and the screens and the sound switched on and off in a loop. `Backoff` (`crates/op8/src/backoff.rs`) waits 4 s after a failure, then 8, 16 and 30 s while failures keep coming, and starts over after a steady minute. The audio keeper also lets the device settle for 3 s after its stream dies. The frame clock then continues from now instead of racing through the frames it missed.
- `OP8_DISPLAY_STATS=1` prints once a second how many transfers, how many KB and how many ms.

Measured on the MK3 behind a USB 2.0 hub, with the demo: ~64 transfers per second, ~1.1 MB/s, ~3.4 ms each; at rest, 4 per second (only the refresh).

The protocol data (commands `00` literal, `01` repeat, `02` skip, `03` show, `40` end) come from public reverse-engineering notes; the code is our own. No code from Deckard (GPL-2.0) is copied.

## 8. Simulator and scripts

`op8 sim` and `op8 run --script` use the same `App` and the same engine. The only thing that changes is the surface:

- `op8 sim`: `MockSurface` (in memory), no hardware.
- `op8 run`: the real MK3, with cpal audio and MIDI.

A **script** (`crates/op8/src/script.rs`) is a text file with one action per line (`<seconds> <command>`); the demo is `crates/op8/scripts/demo.op8` and the hardware battery is `bateria-hw.op8`. `Script::parse` validates everything on load and gives the line number if there is an error. `Script::due(now_ms)` returns the actions that have come due. Commands: `btn`, `pad`, `knob`, `touch`, `encoder`, `strip`, `shot`.

## 9. Test bench

It is the way to test the whole system without hardware. It has three parts:

| Part | Where | What it does |
|---|---|---|
| **Bench** (`Bench`) | `crates/op8/src/bench.rs` | Assembles engine + `App` + `MockSurface`. Advances time like the real program (60 fps UI, audio blocks). Records audio, timed MIDI, LEDs and screens. Exposes helpers (`press`, `tap`, `hold`, `knob`, `send`) and measures (`rms_db`, `peak_db`, `silent`, `lit_pixels`, `screen_hash`). |
| **Cases** | `crates/op8/tests/*.rs` | One file per area of [features.md](features.md). Each `#[test]` carries the ID of its feature (`// F-S01`). |
| **Evidence** | `$OP8_BENCH_DIR` | When run with `OP8_BENCH_DIR=…`, each case that asks for it saves PNG screenshots and WAV files for review by hand. |

Principles:

- Each case checks **one behaviour** and asserts it with a number or a concrete state, not with "looks fine".
- Cases are deterministic: the same script gives the same audio (a case checks it).
- Cases do not touch hardware. Testing with the real MK3 stays in `hw-test` and `run --script`, which are manual.
- What the bench cannot judge (whether a sound is pleasant, whether a screen is legible) is marked **manual review** in [features.md](features.md).

Cargo runs each crate's tests from the crate's folder, so give `OP8_BENCH_DIR` as an absolute path.

### Projects on disk

A project file (`crates/op8/src/projectfile.rs`) is `OP8P`, a version byte, the project (`op8_seq::io`, a plain versioned binary written by hand; version 2 adds each track's players, effect and LFO, and a version 1 file still opens with those off) and, for each of the eight tracks, the name and saved state of its instrument. It is written next to its name and renamed, so a crash never leaves half a project, and read defensively. The player's eight live in `~/.config/op8/projects/user-1.op8p … user-8.op8p` (`OP8_PROJECTS` overrides the folder); the sampler's own WAV files are read from `~/.config/op8/samples/` (`OP8_SAMPLES` overrides it) when the instrument is created (`rec-1.wav` to `rec-8.wav`, what SAMPLING saves, are places 1 to 8; the others fill the places that are left, by name), and a sound saved while a sampler plays reaches it through `InstrumentCartridge::load_sample`; the factory eight are built in (`crates/op8/src/factory.rs`); the presets that come with each instrument are values of its eight knobs in `crates/op8-cartridges/presets.txt` (read by `op8_cartridges::presets`, copied into the twin, kept equal by a test); a preset of the player's is the instrument's `save_state` after a small header (`OP8S` and a version byte), one file per place in `presets/<instrument>/user-N.preset`, written through a temporary file and read back with `load_state`, after which the engine is told each knob's value so that it keeps them. Opening one sends `EngineCommand::LoadProject` and loads each instrument (the running MIDI bridge stays, because it owns the MIDI queue); the sequencer restarts from step 1 if it was playing.

## 10. Decisions taken (do not reopen without Javi) and known limits

**Decisions**

- **Licence MIT or Apache-2.0.** No GPL dependencies are added. Encdr, autotel/maschine-mk3-linux and Deckard are GPL: from the MK3 protocol only the facts were taken, not the code. Surge XT and Vital are GPL and stay out unless Javi decides otherwise.
- **Mutable Instruments synthesis engines (MIT).** Javi chose to start there. The code can be used, but **without the module names**, because they are trademarks: the Plaits one is called "macro", the Rings one will be called "resonator", and so on. Credit goes in the vendor README.
- **Eight and eight (2026-11-13).** 8 factory instruments plus 8 places for installed ones, and 8 factory projects plus 8 of the player's own: the 16 pads ([instruments.md §7](instruments.md)). The factory set is confirmed (KickSyn, FmBass, macro, MIDI out, drums, resonator, sampler, granular; the first four exist) and the rest are still to be built, in that order ([roadmap.md](roadmap.md)).
- **The brand mark and every picture of the unit** follow the MK3's own arrangement of eight and sixteen ([design.md §2.3–2.4](design.md)). The themes are glossy, matte and chrome.
- **Aesthetic:** the language of the OP-1 and OP-XY (colour tied to each physical control, drawings instead of text, absolute black). We take the ideas, never their graphics or fonts. It must not look like a programming project. Details in [design.md](design.md).
- **8 knobs, 8 colours.** Knobs 1–4 drive the sequencer and appear on the left screen; 5–8 drive the instrument and appear on the right. Three themes: matte (default), glossy, chrome.
- **Typeface: Jost** (OFL), weights 300 and 400, tuned by eye to resemble the OP-XY's. FlipMono cannot be used: it is actually SF Mono, licensed only for Apple devices.
- **Instruments:** at most 8 parameters, 4 of sound (button 5) and 4 of shape (button 6). Effects and modulation are per track and the same for all instruments.
- **Settings are chosen on the machine and kept in a small text file** of `key=value` lines (`crates/op8/src/settings.rs`; today only `theme=matte`). Location: `OP8_CONFIG`, else `$XDG_CONFIG_HOME/op8/settings`, else `$HOME/.config/op8/settings`. A missing file, an unknown key or a bad value is ignored (the default is used): a damaged file must never stop the machine from starting. It is written beside itself and renamed, so a power cut leaves the old file or the new one. **At start-up, `-t` / `OP8_THEME` win over the saved theme, which wins over glossy.** Only `op8 run` reads it: `sim` always starts from glossy so it stays reproducible.

**Known limits**

- The real MK3 has 4 audio channels; the engine produces stereo and it is copied to 1/2 and 3/4.
- On macOS, `NIHardwareAgent` and NI's other services take screen interface 5. If they are running, OP-8 warns `screens unavailable` and carries on without screens.
- **The backlight is what resets the MK3.** Behind a USB 2.0 hub it receives 480 mA, and at the old default brightness of 80 it reset within seconds of the music starting. Measured 2026-11-13, 45 s of PLAY each: brightness 10, 40, 50 and 60 held; 70 failed once at 31 s; 80 failed at once. Audio, MIDI, LEDs, screen bandwidth and transfer rate were each ruled out (details in [roadmap.md](roadmap.md)). `op8 usb-reset` recovers a stuck device.

  **The app now reads what the host grants the MK3** (`UsbPowerSinkAllocation` in the I/O registry, macOS only; `crates/op8-hw/src/mk3/power.rs`) each time it opens it, and prints it at start-up (`power: the host gives the MK3 480 mA -> limited: ...`). Below 500 mA the supply is *limited*: the screens go to 50, colour LEDs are capped at level 2 and the other LEDs scaled to 60 %. From 500 mA up it is *full*: screens at 80 and the LEDs untouched. An unknown supply (Linux, or nothing read) counts as limited, because guessing high resets the device and guessing low only dims it. **The 500 mA threshold is provisional**: it separates the 480 seen behind the hub from what a device asks for, and has to be confirmed with the MK3 plugged straight into the Mac. `OP8_POWER=full|limited` forces the plan, and `OP8_BRIGHTNESS` still sets the screens on its own.
- The LED palette is only confirmed for red and orange. The rest is unverified on a unit.

## 11. Build, run and test

A stable Rust and a C++ compiler are needed. On Mac: `xcode-select --install`. On Linux or the Pi: `build-essential` and `libasound2-dev`.

```sh
cargo test --workspace                       # everything, without hardware
cargo clippy --workspace --all-targets
cargo run --release -- sim -s 8 -o out       # simulated MK3: out.wav + .ppm captures
cargo run --release -- sim --script f.op8    # your own script: wav + captures + loudness table
cargo run --release -- run --script f.op8    # the script plays the real MK3
cargo run --release -- hw-test               # test frame and rainbow, with the real MK3
cargo run --release -- usb-reset             # reset a stuck MK3 (with op8 closed)
cargo run --release [-- -t chrome]            # play
scripts/run.sh [-l] [-s] [-r] [args]         # the easy way to play: builds, frees the MK3, waits for it, starts
open twin/op8-twin.html                      # the Twin
```

`scripts/run.sh` closes a running op8 and NI's background services (`NIHardwareAgent`, `NIHostIntegrationAgent`, `NTKDaemon`, which hold the screens), builds, waits up to 15 s for the MK3 on USB, and starts `op8 run`. Its own options come first: `-l` prints every control event (`OP8_LOG_EVENTS`), `-s` the screen traffic (`OP8_DISPLAY_STATS`), `-r` resets the MK3 on the bus first; anything after them (`-t chrome`, `--script f.op8`) goes to `op8 run`.

Environment: `OP8_THEME`, `OP8_CONFIG=<file>` (the settings file), `OP8_BRIGHTNESS` (0–100; by default 50 on a limited supply and 80 on a full one), `OP8_POWER=full|limited` (forces the power plan), `OP8_DISPLAY_STATS=1`, `OP8_DISPLAY_KBPS=<n>`, `OP8_DISPLAY_FPS=<n>` and `OP8_DISPLAY_DELTA=1` (optional screen limits, see §7), the diagnostics `OP8_MIDI=off`, `OP8_NO_LEDS=1` and `OP8_NO_SCREENS=1` (each leaves one part of the MK3 alone, to find what makes it misbehave), `OP8_LOG_EVENTS=1` (prints every control event and every transport change, to tell "the button never arrived" from "it arrived and nothing started"), `OP8_AUDIO_DEVICE=<name>`, `OP8_AUDIO_INPUT=<name>`, `OP8_PRESETS=<dir>` (where the presets are; by default `presets/` next to the settings file), `OP8_BUFFER=<frames>`, `OP8_DUMP_DIR=<dir>` (writes `left.png` and `right.png`, up to four times a second), `OP8_BENCH_DIR=<dir>`.

CI (`.github/workflows/ci.yml`) runs on Linux (tests, clippy, the simulator and `cargo check` for aarch64, with `g++-aarch64-linux-gnu` because op8-plaits compiles C++) and on macOS. For the Pi there are `scripts/build-pi.sh` and `deploy/` (systemd, udev, setup).

## 12. How to work here

- A UI change is tried first in `twin/op8-twin.html` and [design.md](design.md) is updated in the same PR. A change to the treatise is approved by Javi.
- When something changes, update [roadmap.md](roadmap.md), [buttons.md](buttons.md), [instruments.md](instruments.md) or [features.md](features.md) as appropriate.
- On screen everything is lowercase and without codes or IDs (rule 8). Each knob paints only what it moves, in its colour (rule 1).
- No memory is allocated on the audio thread. Instruments must pass the contract tests in `op8-cartridges/src/lib.rs`.
- Tests that touch the global theme use `THEME_LOCK` (`op8-ui`) so they do not fail when run in parallel.
- To vendor third-party code: copy it unchanged, with its licence and a README of origin, as in `crates/op8-plaits/vendor/`.
- Both screens animate. Never assume either one is still.

## 13. Related projects

Other projects that drive a Maschine MK3 or a similar controller. The first list comes from our own earlier survey (Monolith's `docs/github-ecosystem-references.md` and `docs/deckard-reference-architecture.md`); the details were checked on GitHub on 2026-11-13 (licence, language, last push, open issues). **GPL projects are read for facts only; their code is never copied** (see §10).

| Project | What it is | Licence · language · state | What it gives OP-8 |
|---|---|---|---|
| [Panic-Signals/Deckard](https://github.com/Panic-Signals/Deckard) | NI Maschine-like software with its own MK3 integration | GPL-2.0 · Rust · active (pushed 2026-11-13) | **The closest in spirit**: a standalone groovebox that drives the MK3's pads, LEDs and screens from Rust. Its screen method is the one in §7 (dirty rows, repeated-pair compression, a thread of its own). Monolith's `crates/mk3` is a copy of its driver |
| [autotel/maschine-mk3-linux](https://github.com/autotel/maschine-mk3-linux) | MK3 driver for Linux: MIDI, screens, a config file | GPL-3.0 · Rust · active | The protocol documentation OP-8's driver was written from (facts only). Issue #1 is the nearest precedent for us: a MK3 on a **Raspberry Pi 5** as a live instrument, both screens working |
| [RufusIbiza/encdr-maschine-bitwig](https://github.com/RufusIbiza/encdr-maschine-bitwig) (Embit) and Encdr | Bitwig controller over HID, in Rust | GPL-3.0 · Rust | A second HID implementation to compare reports against |
| [santiagojorda/maschine-mk3-driver](https://github.com/santiagojorda/maschine-mk3-driver) | Python library for the two screens | no licence declared · Python · active | A simple, working reference of the screen transfer (20–30 fps, only the changed regions) and of reopening after a bus drop |
| santiagojorda/maschine-mk3-as-ableton-push | The MK3 as an Ableton Push, over UDP | not reachable on 2026-11-13 (404: private or renamed) | Only what Monolith's survey says about it |
| [shaduzlabs/cabl](https://github.com/shaduzlabs/cabl), [Drachenkaetzchen/cabl](https://github.com/Drachenkaetzchen/cabl) | Controller abstraction layer in C++ | MIT · C++ · stopped in 2018 | The HID report layout, as history; MK3 support was never finished (issue #9) |
| [chiakibeats/CustomMaschineMK3](https://github.com/chiakibeats/CustomMaschineMK3) | Ableton MIDI Remote Script for the MK3 | GPL-3.0 · Python · active | How Ableton sees the MK3 through NI's own Controller Editor; not a driver |
| [git-moss/DrivenByMoss](https://github.com/git-moss/DrivenByMoss) | Bitwig and Reaper extensions for many controllers, MK3 included | LGPL-3.0 · Java · active | Encoder acceleration and the DAW-side design of a control surface |
| [r00tman/maschine-mikro-mk3-driver](https://github.com/r00tman/maschine-mikro-mk3-driver) | MIDI driver for the Mikro MK3 | MIT · Rust | A different device (no MK3 screens), a small Rust HID example |
| [asutherland/ni-controllers-lib](https://github.com/asutherland/ni-controllers-lib) | Node library for several NI controllers | no licence declared · TypeScript | The original source of the screen packet notes that other projects cite |
| [morluto/rea](https://github.com/morluto/rea) | A reverse-engineering toolkit for AI agents | — | How NI's own agent could be inspected to confirm descriptors |
| [owainlewis/machinist](https://github.com/owainlewis/machinist) | **Not a MK3 project.** Infrastructure for AI coding workflows | MIT · Go | Nothing about the MK3 or audio. Monolith cites it only for its heartbeat and lease design (`docs/synchronization.md`) |

### What they say about the problems we hit

- **The backlight reset.** No project's public issues mention a MK3 that resets when its screens are bright (the issue lists of Deckard, autotel, Embit, santiagojorda, r00tman, CustomMaschineMK3, cabl and DrivenByMoss were read). What does exist: NI lists the MK3's power supply as **optional** and says connecting it makes the LEDs and screens brighter ([NI support](https://support.native-instruments.com/support/solutions/articles/69000879997-which-ni-hardware-devices-require-a-power-supply-)), and a forum user with a long USB run reported the MK3 crashing and a powered hub not matching the supply ([NI community](https://community.native-instruments.com/discussion/comment/36165)). Both are consistent with our measurement (§10) that a bright backlight on 480 mA resets the device, but neither is a specification. The supply is the untried fix: it is an NI accessory, not always in the box.
- **A device stuck on the bus.** autotel issue #1 reports a MK3 that stopped sending HID input after the host re-enumerated it: the interrupt endpoint only NAKed, a USB reset did not help, and unplugging both the MK3's own power and its USB did. Our stuck state (HID writes failing after a reset) was cleared by a USB reset, but the lesson is the same: a MK3 can hold state of its own, and a full power cycle is the last resort.
- **Screens on a Raspberry Pi 5.** The same issue reports Pi 5, Debian 12, kernel 6.12 PREEMPT_RT, JACK2 and a systemd service, with pads, knobs, buttons, LEDs and both screens working, and a local patch to leave interface 5 free for their own screen renderer. The deployment §11 describes is not untried ground.
- **How to send the screens.** Deckard, autotel and santiagojorda's driver all converge on what §7 does: send only the changed rows, at no more than 60 fps, from a thread apart from the controls. autotel measures a single knob move at about 30 KB instead of 261 KB; ours is the same order.
- **LED shades.** Deckard issues #154–#156 report that on the MK3 the "bright" shade the app asks for comes out as the white-mixed flash level, so a selected and a flashing pad look alike. OP-8 uses level 3 for "bright" and 1 for "dim" (`leds.rs`), so expect the same on the unit; it is part of the unverified LED palette ([roadmap.md](roadmap.md)).
- **macOS transfer timeouts.** cabl issue #2 (2017, OS X 10.11) is a libusb `transfer error: timed out` that the author called harmless. We saw none.
