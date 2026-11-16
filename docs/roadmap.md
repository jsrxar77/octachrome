# OP-8 — Roadmap

It comes from the [button map](buttons.md) and from [instruments.md](instruments.md): each MK3 button has an idea assigned, and this roadmap is the order in which they are built. Each step is tried first in the Twin (`twin/op8-twin.html`) and then in Rust. Status as of **2026-11-13**.

## Done

- FILE: save and open projects (the factory eight and the player's eight; [architecture.md](architecture.md)).
- Engine, sequencer with polymeter, Step Components, Brain and Punch-in FX.
- Cartridges KickSyn, FmBass, MidiOutBridge, **macro**, **drums** (eight percussion sounds in eight lanes) and **resonator** (strings and bells, four voices) (24 engines, from the Plaits code, MIT). macro has `engine`/`harmonics`/`timbre`/`morph` on page 1 and `decay`/`color`/`variant`/`level` on page 2.
- MK3 driver (pads, knobs, buttons, LEDs, screens), `hw-test`, and deployment on the Pi 5 with systemd.
- Screens on the real MK3: full-width row bands with compression, sent from a thread of their own, with a full refresh every 500 ms (see [architecture.md §7](architecture.md)).
- Visual language: 8 knob colours, three themes, Jost, drawings per instrument, touch overlay.
- Buttons ✓: STEP, CHORDS (Brain), PERFORM (Punch-in), groups A–H, MUTE, ERASE + pad, PLAY, STOP, RESTART, SHIFT, screens 1–8. The rest is in [buttons.md](buttons.md).
- The step grid shows active and empty steps far apart (screen and pad LEDs), and the screens name their buttons in a strip along the top.
- The light of every button follows one rule (dark, a very faint white, a strong white when pressed or in focus; [design.md §2.6](design.md)).
- PAD MODE and KEYBOARD: the pads play the instrument by hand (a drum kit's eight sounds, the Brain's scale, or a chromatic keyboard), without recording.
- Test bench, scripts and feature cases that run without hardware ([features.md](features.md)).
- Main modes: CHANNEL (composing) and SETTINGS (both screens). Settings has one page today, the theme: it is chosen with the 4D encoder and kept in a settings file ([architecture.md §10](architecture.md)). Tried in the Twin first.
- The audio comes back by itself when the MK3's audio device drops (it used to stay dead, leaving the sequencer frozen).
- The name is OP-8. The brand mark is defined (eight rainbow buttons, 4 + 4 as the unit's group keys or eight knobs in a row, the eighth white as the octave), the themes are glossy (bright), matte (soft) and chrome (one monochrome range) with their own backlight and LED levels, and every picture of the unit follows the real front panel ([design.md §2.3–2.4](design.md)). Guide, manual and brand page are in `guide/`.
- Twin with the same screens, macro on track G and an approximate sound in WebAudio.

## Hardware status

Checked on a real MK3 (macOS, Apple Silicon):

- Both screens, animated, complete and in step (photo and video review, feature F-D07).
- RGB565 colour order (the first bar of the calibration frame is red).
- Buttons and pads reach the app (PLAY events seen).

Not yet checked:

- The SETTINGS button, its LED and the 4D encoder in settings (F-C08), and that the audio, screens and PLAY recover after pulling the cable (F-A07).
- Themes other than glossy on the real screens, and how glossy, matte and chrome look and feel with their backlight and LED levels on the unit; chrome's colours and light are provisional.
- The MK3 plugged straight into the Mac (F-H06): what the `power:` line reports there, and whether brightness 80 then holds. The 500 mA threshold between limited and full is provisional until then. NI's optional power supply is the other fix people point to ([architecture.md §13](architecture.md)).
- The LED hues the three themes use for the pads and group keys (matte's `LightOrange`, `WarmYellow`, `Mint`, `Plum`, `Fuchsia`; chrome's magenta to blue range), and the LED palette beyond red and orange, and whether level 3 comes out white-mixed (Deckard issue #154, [architecture.md §13](architecture.md)).
- CPU load on the Pi 5 (target in [instruments.md §5](instruments.md)), and the same on the Pi 4: audio load with eight slots sounding, latency with real-time priority, and the cost of the screens. Until then the hardware below is an estimate, not a measurement (2026-11-15):
  - **Minimum: Raspberry Pi 4, 2 GB** (Cortex-A72). Should work, but tight; it is the floor only once measured.
  - **Standard: Raspberry Pi 5, 4 GB** (Cortex-A76), with the official 27 W supply so the USB port can power the MK3. This is the product target.
  - **Maximum: Pi 5, 8 GB with the active cooler**, or an RK3588 board (Orange Pi 5, Rock 5B), only if much heavier instruments or effects come; the Linux and audio support of RK3588 is worse.
  - **Discarded: Pi Zero 2 W and Pi 3** (A53 cores too slow, little RAM, weak USB).
- SAMPLING on the unit: the MK3's input (level, microphone permission on the Mac, the stream reopening), the resample source, and the saved file reaching a playing sampler (F-A18).

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

- Done: all eight screen buttons are named in the strip and work: 1 step, 2 components, 3 players, 4 bar, 5 sound, 6 shape, 7 fx, 8 mod ([design.md §4](design.md)).
- PAGE ◀ ▶, 4D encoder (▲ ▼ ◀ ▶, push, turn).
- SETTINGS: audio, MIDI and brightness as further pages (the theme page is done). The 4D ▲ ▼ chooses the page once there is more than one, and the right screen then shows its page dots.

## 2. Playing and recording

- Done: PAD MODE and KEYBOARD play the track's instrument from the pads (octave on the 4D encoder). They are not recorded yet: that is REC, below.
- Done: REC writes notes played by hand into the track at the nearest step, or the one sounding, or the next (the bar page; a drum kit: into its lane). To do: recording chords and knob movements (automation), the count-in.
- Done: TAP (tap tempo, SHIFT + TAP the metronome), TEMPO, SWING and VOLUME with the 4D encoder (master volume), and FIXED VEL. To do: a track's volume with a group held, the count-in.
- AUTO: automation per knob.

## 3. Instruments (the centre)

- **Eight and eight.** 8 factory instruments + 8 installed ones ([instruments.md §7](instruments.md)). Factory today: drums, KickSyn, FmBass, resonator, sampler, macro and the MIDI out, ordered from the most guttural to the most refined (leads next to last, the MIDI out last; the granular goes between the sampler and the macro). The sampler plays sounds and records them (stages 1 and 2 done), and BROWSER keeps presets; to build: granular.
- Done: PLUG-IN shows the 16 places on the pads (the factory eight on top, eight places for installed ones below) and puts the chosen instrument on the selected track.

- Kit of reusable drawings in `op8-gfx` (oscilloscope, envelope, polar waveform).
- Load instruments as a separate package from `cartridges/` (stage 2 of [instruments.md](instruments.md)).
- More instruments: resonator (from Rings), drums (8 sounds on the pads), sampler. Each one meets [instruments.md](instruments.md): at most 8 knobs, a drawing without text, and any combination sounds good.

## 4. Track sound

- Done: BROWSER (presets per instrument, with eight that come with each instrument). To do: tune them by ear on the unit.
- Done: an effect per track (delay, reverb, chorus, filter, drive; button 7) and an LFO per track (button 8). To do: more effects (check the CPU on the Pi first), an LFO that follows the tempo, a second LFO, and an effect send shared by all tracks.
- MACRO (Auxiliary): effect sends and Brain.
- Done: the players (button 3): an arpeggio or a chord over each step. To do: NOTE REPEAT, an arpeggio that takes its notes from the Brain's chord instead of its own, and players on a drum kit (a roll).

## 5. Structure

- **Eight and eight projects.** 8 factory projects (eight short pieces in eight styles, made only with the factory instruments and their presets: [instruments.md §9](instruments.md)) and 8 for the player; FILE shows them on the 16 pads ([instruments.md §7](instruments.md)). Needs save and load first (§7).

- PATTERN: 16 patterns per track.
- SCENE and ARRANGER: scenes and song.
- DUPLICATE, SELECT, SOLO, VARIATION, EVENTS, LOCK.
- Done: FILE opens the eight factory projects and the player's eight and saves into them (the projects folder). To do: naming a project, deleting one, saving a copy.

## 6. Mix and samples

- MIXER: level, pan, EQ and master compressor.
- Done: the sampler cartridge (eight sounds that come with it, the player's WAV files from `~/.config/op8/samples/`). SAMPLING records what OP-8 plays or the audio input, cuts it and saves it into the sampler (2026-11-15; not yet tried on the unit). To do: moving the cut while it plays, a recording threshold and count-in, and stereo or longer recordings.
- Smart Strip with PITCH, MOD and NOTES.

## 7. Engine and quality

- Write chords into the Brain progression from the pads (today the pads only play *live* chords).
- Parameter locks with sample accuracy (today they apply at the start of the block, up to 2.7 ms early).
- A triple-buffer snapshot of the cartridge UI so that `draw_ui` never shares the mutex with the audio.
- Real-time priority for the audio thread on Linux (`audio_thread_priority`), and a latency measurement on the Pi 5.
- MIDI with timestamps (today it goes out as soon as the queue is drained, up to 1 ms plus one block).
- Tuning the presets that come with the instruments by ear (BROWSER already keeps the player's); projects already save the instruments' state.

## 8. The batch of 2026-11-15 (in order)

Asked by Javi on 2026-11-15, one pull request each, in this order:

1. **Buttons 1 and 5, and the theme order.** Done: screen button 5 is `sound` (the first page, with what matters most of the instrument) and 6 is `shape`; choosing a track puts the screens on button 1 (steps) and button 5 (sound); matte is the default theme (it was chrome for a day, 2026-11-15), in the machine, the twin, the guide and the manual, and comes first in the list (glossy and chrome follow). Not yet seen on the unit.
2. **BROWSER (presets), the sampler's stage 3.** Done (2026-11-15, not yet tried on the unit): the button opens a main mode on both screens; the sixteen pads are sixteen presets of the selected track's instrument (the top two rows the instrument's own, the bottom two the player's); a tap loads one and plays a note, SHIFT + tap on the bottom rows saves the current knobs into `~/.config/op8/presets/<instrument>/user-N.preset`. Presets are per instrument. The presets that come with the instruments are done as a first set of eight each (2026-11-15, [instruments.md §8](instruments.md)), checked by measurement and not yet by ear. To do: tune them on the unit, naming and deleting a preset.
3. **The manual, complete and in matte.** Done (2026-11-15): a diagram of the unit at the start with every part named and numbered (buttons, screens, knobs, pads, groups, transport, 4D encoder, Smart Strip), so that the rest of the manual can say "button 5" or "group G"; the manual checked against the code (the instruments, "where OP-8 keeps your things", "if something is odd", "not yet"); the page and all its pictures in the matte theme; recipes 16 (SAMPLING) and 17 (BROWSER).
4. **The public site (guide and manual).** Asked on 2026-11-15: the guide and the manual as one introduction to the project that can be published, well connected, in the matte theme (matte first, then glossy, then chrome; brand stays apart). Done: the sources are templates with a shared header, footer and style; `scripts/build-site.py` builds `site/` (pages, 404, `robots.txt`, `sitemap.xml`, the sharing image, social tags, the twin as "Try it"); `scripts/check-site.py` checks links, anchors, images and metadata and runs in `cargo test` (F-G05); the Pages workflow builds, checks and publishes it; the manual works on a phone. The home page now says what OP-8 is, what you need (the MK3 and a Pi 5, with a Pi 4 as the floor) and where it stands, in plain words for someone who has never heard of it. To do: look at it on a phone and a laptop once published; **to publish**, fill in the address of the site and of the repository (`guide/site.json`, or `OP8_SITE_URL` and the repository variable `OP8_REPO_URL`) and enable Pages (Settings, Pages, source GitHub Actions; a public repository or a plan that includes Pages), then look at it once on a phone and a laptop.
5. **Eight pieces and the order of the instruments.** Asked on 2026-11-15: eight musical examples to hear what the machine can do, and the MIDI out last. Done: the factory projects are house, boom bap, dub, polyrhythm, acid, electro, ambient and breaks (levels matched, F-F07); PLUG-IN and the standard tracks run drums, KickSyn, FmBass, resonator, sampler, (granular), macro, MIDI out; an empty place of the player's in FILE opens a new empty project; a pad that closes FILE or PLUG-IN no longer toggles a step when it is let go. To do: tune the pieces and the presets by ear on the unit.
6. **Navigation with the 4D encoder.** Asked on 2026-11-15: every screen has items that can be reached with the central 4D encoder, global and easy. Decided: a focus that is always visible (a ring and a dot), the encoder turns it, the arrows move it across the screens, push activates or grabs a value, and the tempo is only turned while TEMPO is held. Done (2026-11-15; no dot, only the frame): the mechanism and every view: FILE, PLUG-IN, BROWSER, SAMPLING (with its four values) and composing (the screen buttons 1–8, the tracks A–H, the pads of the grid and the eight values under the screens, which push grabs and the 4D edits), and an opened piece shows its music (F-N01 to F-N10). Not yet tried on the unit. 2026-11-16: the frame of a track hugs its round capsule, and PERFORM works with the 4D (the focus visits the sixteen effects where they are drawn, push engages one and keeps it, turning is its intensity; Brain lists no pads) (F-N11, F-N12). 2026-11-16, after a photo of the unit: the ink over a coloured fill is made to read (`readable`, F-D19) and no ring goes off the screen, the ring of a screen button lying on its own capsule (F-N13). 2026-11-16: the eight sounds of a drum kit can be chosen with the 4D from the right screen (F-N14). To do: the step inspector's own items (its components and values) so that it can be reached the same way.
7. **After that.** Granular (reuses the sampler); KEYBOARD on a drum kit like PAD MODE; a basic mixer; naming and deleting projects; PATTERN, SCENE and ARRANGER; real-MK3 verification of everything marked "not yet tried" above.
