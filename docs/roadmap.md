# OP-3 — Roadmap

It comes from the [button map](buttons.md) and from [instruments.md](instruments.md): each MK3 button has an idea assigned, and this roadmap is the order in which they are built. Each step is tried first in the Twin (`twin/op3-twin.html`) and then in Rust. Status as of **2026-11-13**.

## Done

- Engine, sequencer with polymeter, Step Components, Brain and Punch-in FX.
- Cartridges KickSyn, FmBass, MidiOutBridge and **macro** (24 engines, from the Plaits code, MIT). macro has `engine`/`harmonics`/`timbre`/`morph` on page 1 and `decay`/`color`/`variant`/`level` on page 2.
- MK3 driver (pads, knobs, buttons, LEDs, screens), `hw-test`, and deployment on the Pi 5 with systemd.
- Screens on the real MK3: full-width row bands with compression, sent from a thread of their own, with a full refresh every 500 ms (see [architecture.md §7](architecture.md)).
- Visual language: 8 knob colours, three themes, Jost, drawings per instrument, touch overlay.
- Buttons ✓: STEP, CHORDS (Brain), PERFORM (Punch-in), groups A–H, MUTE, ERASE + pad, PLAY, STOP, RESTART, SHIFT, screen 5–6. The rest is in [buttons.md](buttons.md).
- Test bench, scripts and 38 feature cases that run without hardware ([features.md](features.md)).
- Twin with the same screens, macro on track D and an approximate sound in WebAudio.

## Hardware status

Checked on a real MK3 (macOS, Apple Silicon):

- Both screens, animated, complete and in step (photo and video review, feature F-D07).
- RGB565 colour order (the first bar of the calibration frame is red).
- Buttons and pads reach the app (PLAY events seen).

Not yet checked:

- The LED palette beyond red and orange.
- CPU load on the Pi 5 (target in [instruments.md §5](instruments.md)).

Known limits of the setup:

- Behind a USB 2.0 hub, macOS gives the MK3 only 480 mA (`UsbPowerSinkAllocation`). Under heavy load the device can reset itself. Connect it directly, or use `OP3_BRIGHTNESS` (0–100, default 80). `op3 usb-reset` recovers a stuck device; run it with op3 closed.
- NI's services (`NIHardwareAgent`, `NIHostIntegrationAgent`, `NTKDaemon`) and any other `op3` take interface 5: close them first.

## 1. Navigation skeleton (next)

- Top strip with the labels of screen buttons 1–8 (rule 11).
- Screen buttons 1–4: step, components, players, bar. 5–8: sound, shape, fx, mod.
- PAGE ◀ ▶, 4D encoder (▲ ▼ ◀ ▶, push, turn).
- SETTINGS: choose the theme from the MK3, audio and MIDI.

## 2. Playing and recording

- KEYBOARD with octave on the 4D encoder.
- REC (notes and automation), TAP + metronome, FIXED VEL.
- TEMPO, SWING and VOLUME with the 4D encoder.
- AUTO: automation per knob.

## 3. Instruments (the centre)

- Cartridge registry list and PLUG-IN to choose the cartridge of each track (from `builtin_names()`).
- Kit of reusable drawings in `op3-gfx` (oscilloscope, envelope, polar waveform).
- Load instruments as a separate package from `cartridges/` (stage 2 of [instruments.md](instruments.md)).
- More instruments: resonator (from Rings), drums (8 sounds on the pads), sampler. Each one meets [instruments.md](instruments.md): at most 8 knobs, a drawing without text, and any combination sounds good.

## 4. Track sound

- BROWSER: presets per cartridge.
- fx modules (button 7) and mod/LFO (button 8) per track.
- MACRO (Auxiliary): effect sends and Brain.
- NOTE REPEAT and arpeggiator (players, button 3).

## 5. Structure

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
