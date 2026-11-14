# OP-8 — Roadmap

It comes from the [button map](buttons.md) and from [instruments.md](instruments.md): each MK3 button has an idea assigned, and this roadmap is the order in which they are built. Each step is tried first in the Twin (`twin/op8-twin.html`) and then in Rust. Status as of **2026-11-13**.

## Done

- Engine, sequencer with polymeter, Step Components, Brain and Punch-in FX.
- Cartridges KickSyn, FmBass, MidiOutBridge and **macro** (24 engines, from the Plaits code, MIT). macro has `engine`/`harmonics`/`timbre`/`morph` on page 1 and `decay`/`color`/`variant`/`level` on page 2.
- MK3 driver (pads, knobs, buttons, LEDs, screens), `hw-test`, and deployment on the Pi 5 with systemd.
- Screens on the real MK3: full-width row bands with compression, sent from a thread of their own, with a full refresh every 500 ms (see [architecture.md §7](architecture.md)).
- Visual language: 8 knob colours, three themes, Jost, drawings per instrument, touch overlay.
- Buttons ✓: STEP, CHORDS (Brain), PERFORM (Punch-in), groups A–H, MUTE, ERASE + pad, PLAY, STOP, RESTART, SHIFT, screen 5–6. The rest is in [buttons.md](buttons.md).
- Test bench, scripts and feature cases that run without hardware ([features.md](features.md)).
- Main modes: CHANNEL (composing) and SETTINGS (both screens). Settings has one page today, the theme: it is chosen with the 4D encoder and kept in a settings file ([architecture.md §10](architecture.md)). Tried in the Twin first.
- The audio comes back by itself when the MK3's audio device drops (it used to stay dead, leaving the sequencer frozen).
- The name is OP-8. The brand mark is defined (eight rainbow buttons, 4 + 4 as the unit's group keys or eight knobs in a row, the eighth white as the octave), the themes are glossy, matte and chrome, and every picture of the unit follows the real front panel ([design.md §2.3–2.4](design.md)). Guide, manual and brand page are in `guide/`.
- Twin with the same screens, macro on track D and an approximate sound in WebAudio.

## Hardware status

Checked on a real MK3 (macOS, Apple Silicon):

- Both screens, animated, complete and in step (photo and video review, feature F-D07).
- RGB565 colour order (the first bar of the calibration frame is red).
- Buttons and pads reach the app (PLAY events seen).

Not yet checked:

- The SETTINGS button, its LED and the 4D encoder in settings (F-C08), and that the audio, screens and PLAY recover after pulling the cable (F-A07).
- Themes other than glossy on the real screens.
- The MK3 plugged straight into the Mac (F-H06): what the `power:` line reports there, and whether brightness 80 then holds. The 500 mA threshold between limited and full is provisional until then. NI's optional power supply is the other fix people point to ([architecture.md §13](architecture.md)).
- The LED palette beyond red and orange, and whether level 3 comes out white-mixed (Deckard issue #154, [architecture.md §13](architecture.md)).
- CPU load on the Pi 5 (target in [instruments.md §5](instruments.md)).

**The MK3 reset itself as soon as the music played (2026-11-13), and the cause is the screens' backlight.** The symptom was `display bulk transfer: unknown error`, then the audio device disappearing and the app reconnecting in a loop. Each run below was 40–50 s with PLAY pressed at 6–8 s, on a MK3 behind a USB 2.0 hub (480 mA):

| What was changed | Resets |
|---|---|
| Nothing (brightness 80, the old default) | within seconds, every time |
| Audio sent to the Mac's speakers instead of the MK3 | still reset |
| No MIDI to the MK3 | still reset |
| No LED writes | still reset |
| **No screens at all** | **none** |
| Screens at 30 and at 10 rounds a second; bytes cut fourfold | still reset |
| Brightness 10, 40, 50, 60 | none |
| Brightness 70 | one, after 31 s |
| Brightness 50 with whole rows, no limits, 3.4 MB/s, 90 transfers/s | none |

So the screens are needed for the reset, and only their brightness decides it. That fits the 480 mA the hub allows. The default brightness is now 50 (`OP8_BRIGHTNESS`). Feature F-H03 (five minutes of real use) is still to be checked on the unit, and connecting the MK3 straight to the Mac should leave more margin; that has not been tried.

Known limits of the setup:

- Behind a USB 2.0 hub, macOS gives the MK3 only 480 mA (`UsbPowerSinkAllocation`). A bright backlight resets the device. The app reads what the host grants (`power:` line at start-up) and uses screens at 50 and capped LEDs below 500 mA, screens at 80 from 500 mA up; `OP8_BRIGHTNESS` and `OP8_POWER` override it. Connect the MK3 directly to the Mac for more margin. `op8 usb-reset` recovers a stuck device; run it with op8 closed.
- NI's services (`NIHardwareAgent`, `NIHostIntegrationAgent`, `NTKDaemon`) and any other `op8` take interface 5: close them first.

## 1. Navigation skeleton (next)

- Top strip with the labels of screen buttons 1–8 (rule 11).
- Screen buttons 1–4: step, components, players, bar. 5–8: sound, shape, fx, mod.
- PAGE ◀ ▶, 4D encoder (▲ ▼ ◀ ▶, push, turn).
- SETTINGS: audio, MIDI and brightness as further pages (the theme page is done). The 4D ▲ ▼ chooses the page once there is more than one, and the right screen then shows its page dots.
- Open question: the track colours (A–H) are one fixed palette in all three themes, and look loud next to the muted matte. Decide in the Twin whether each theme gets its own.

## 2. Playing and recording

- KEYBOARD with octave on the 4D encoder.
- REC (notes and automation), TAP + metronome, FIXED VEL.
- TEMPO, SWING and VOLUME with the 4D encoder.
- AUTO: automation per knob.

## 3. Instruments (the centre)

- **Eight and eight.** 8 factory instruments + 8 installed ones ([instruments.md §7](instruments.md)). Factory today: KickSyn, FmBass, macro, MidiOutBridge. To build: resonator, drums, sampler, granular.
- PLUG-IN shows the 16 places on the 4×4 pads (1–8 factory, 9–16 installed); an empty installed place says so.

- Cartridge registry list and PLUG-IN to choose the cartridge of each track (from `builtin_names()`).
- Kit of reusable drawings in `op8-gfx` (oscilloscope, envelope, polar waveform).
- Load instruments as a separate package from `cartridges/` (stage 2 of [instruments.md](instruments.md)).
- More instruments: resonator (from Rings), drums (8 sounds on the pads), sampler. Each one meets [instruments.md](instruments.md): at most 8 knobs, a drawing without text, and any combination sounds good.

## 4. Track sound

- BROWSER: presets per cartridge.
- fx modules (button 7) and mod/LFO (button 8) per track.
- MACRO (Auxiliary): effect sends and Brain.
- NOTE REPEAT and arpeggiator (players, button 3).

## 5. Structure

- **Eight and eight projects.** 8 factory projects (one idea each, made only with the factory instruments) and 8 for the player; FILE shows them on the 16 pads ([instruments.md §7](instruments.md)). Needs save and load first (§7).

- PATTERN: 16 patterns per track.
- SCENE and ARRANGER: scenes and song.
- DUPLICATE, SELECT, SOLO, VARIATION, EVENTS, LOCK.
- FILE: projects, save and open.

## 6. Mix and samples

- MIXER: level, pan, EQ and master compressor.
- SAMPLING and a sampler cartridge.
- Smart Strip with PITCH, MOD and NOTES.

## 7. Engine and quality

- Write chords into the Brain progression from the pads (today the pads only play *live* chords).
- Parameter locks with sample accuracy (today they apply at the start of the block, up to 2.7 ms early).
- A triple-buffer snapshot of the cartridge UI so that `draw_ui` never shares the mutex with the audio.
- Real-time priority for the audio thread on Linux (`audio_thread_priority`), and a latency measurement on the Pi 5.
- MIDI with timestamps (today it goes out as soon as the queue is drained, up to 1 ms plus one block).
- Save and load projects and presets (`save_state` already exists in the cartridges).
