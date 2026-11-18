# Octachrome — Roadmap

It comes from the [button map](buttons.md) and from [instruments.md](instruments.md): each MK3 button has an idea assigned, and this roadmap is the order in which they are built. Each step is tried first in the Twin (`twin/octachrome-twin.html`) and then in Rust. Status as of **2026-11-13**.

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
- The name is Octachrome. The brand mark is defined (eight rainbow buttons, 4 + 4 as the unit's group keys or eight knobs in a row, the eighth white as the octave), the themes are glossy (bright), matte (soft) and chrome (one monochrome range) with their own backlight and LED levels, and every picture of the unit follows the real front panel ([design.md §2.3–2.4](design.md)). Guide, manual and brand page are in `guide/`.
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

So the screens are needed for the reset, and only their brightness decides it. That fits the 480 mA the hub allows. The default brightness is now 50 (`OCTACHROME_BRIGHTNESS`). Feature F-H03 (five minutes of real use) is still to be checked on the unit, and connecting the MK3 straight to the Mac should leave more margin; that has not been tried.

Known limits of the setup:

- Behind a USB 2.0 hub, macOS gives the MK3 only 480 mA (`UsbPowerSinkAllocation`). A bright backlight resets the device. The app reads what the host grants (`power:` line at start-up) and uses screens at 50 and capped LEDs below 500 mA, screens at 80 from 500 mA up; `OCTACHROME_BRIGHTNESS` and `OCTACHROME_POWER` override it. Connect the MK3 directly to the Mac for more margin. `octachrome usb-reset` recovers a stuck device; run it with octachrome closed.
- NI's services (`NIHardwareAgent`, `NIHostIntegrationAgent`, `NTKDaemon`) and any other `octachrome` take interface 5: close them first.

## 1. Navigation skeleton (next)

- Done: all eight screen buttons are named in the strip and work: 1 step, 2 components, 3 players, 4 bar, 5 sound, 6 shape, 7 fx, 8 mod ([design.md §4](design.md)).
- PAGE ◀ ▶, 4D encoder (▲ ▼ ◀ ▶, push, turn).
- SETTINGS: audio, MIDI and brightness as further pages (the theme page is done). The 4D ▲ ▼ chooses the page once there is more than one, and the right screen then shows its page dots.

## 2. Playing and recording

- Done: PAD MODE and KEYBOARD play the track's instrument from the pads (octave on the 4D encoder). They are not recorded yet: that is REC, below.
- Done: REC writes notes played by hand into the track at the nearest step, or the one sounding, or the next (the bar page; a drum kit: into its lane). To do: recording chords and knob movements (automation), and a count-in for this REC (SAMPLING has its own, 2026-11-16).
- Done: TAP (tap tempo, SHIFT + TAP the metronome), TEMPO, SWING and VOLUME with the 4D encoder (master volume), and FIXED VEL. A track's volume and pan are done (VOLUME held and the knobs, 2026-11-16). To do: a count-in for the REC that writes notes (SAMPLING has its own: TAP, F-K20).
- AUTO: automation per knob.

## 3. Instruments (the centre)

- **Eight and eight.** 8 factory instruments + 8 installed ones ([instruments.md §7](instruments.md)). Factory today: drums, KickSyn, FmBass, resonator, sampler, macro and the MIDI out, ordered from the most guttural to the most refined (leads next to last, the MIDI out last; the granular goes between the sampler and the macro). The sampler plays sounds and records them (stages 1 and 2 done), and BROWSER keeps presets; to build: granular.
- Done: PLUG-IN shows the 16 places on the pads (the factory eight on top, eight places for installed ones below) and puts the chosen instrument on the selected track.

- Kit of reusable drawings in `octachrome-gfx` (oscilloscope, envelope, polar waveform).
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
- DUPLICATE, SELECT, SOLO, VARIATION, EVENTS, LOCK.
- Done: FILE opens the eight factory projects and the player's eight and saves into them (the projects folder). To do: naming a project, deleting one, saving a copy.

## 6. Mix and samples

- MIXER: level, pan, sends, the master compressor and the master EQ are done (2026-11-17).
- Done: the sampler cartridge (eight sounds that come with it, the player's WAV files from `~/.config/octachrome/samples/`). SAMPLING records what Octachrome plays or the audio input, cuts it and saves it into the sampler (2026-11-15; not yet tried on the unit). Since then: the armed recording (F-A23), the count-in (F-K20) and the cut that moves while it plays (F-A25). To do: stereo or longer recordings.
- Smart Strip with PITCH, MOD and NOTES.

## 7. Engine and quality

- Write chords into the Brain progression from the pads (today the pads only play *live* chords).
- Parameter locks with sample accuracy (today they apply at the start of the block, up to 2.7 ms early).
- A triple-buffer snapshot of the cartridge UI so that `draw_ui` never shares the mutex with the audio.
- Real-time priority for the audio thread on Linux (`audio_thread_priority`), and a latency measurement on the Pi 5.
- MIDI with timestamps (today it goes out as soon as the queue is drained, up to 1 ms plus one block; the MIDI clock out has the same limit).
- Tuning the presets that come with the instruments by ear (BROWSER already keeps the player's); projects already save the instruments' state.

## 8. The batch of 2026-11-15 (in order)

Asked by Javi on 2026-11-15, one pull request each, in this order:

1. **Buttons 1 and 5, and the theme order.** Done: screen button 5 is `sound` (the first page, with what matters most of the instrument) and 6 is `shape`; choosing a track puts the screens on button 1 (steps) and button 5 (sound); matte was the default theme from 2026-11-15; since 2026-11-16 (Javi) **chrome is the default of the unit and the twin and comes first in the list (chrome, matte, glossy)**, while the guide and the manual stay drawn in matte. Not yet seen on the unit.
2. **BROWSER (presets), the sampler's stage 3.** Done (2026-11-15, not yet tried on the unit): the button opens a main mode on both screens; the sixteen pads are sixteen presets of the selected track's instrument (the top two rows the instrument's own, the bottom two the player's); a tap loads one and plays a note, SHIFT + tap on the bottom rows saves the current knobs into `~/.config/octachrome/presets/<instrument>/user-N.preset`. Presets are per instrument. The presets that come with the instruments are done as a first set of eight each (2026-11-15, [instruments.md §8](instruments.md)), checked by measurement and not yet by ear. To do: tune them on the unit, naming and deleting a preset.
3. **The manual, complete** (its pictures were in matte; since 2026-11-17 they are in chrome, the theme the unit starts in). Done (2026-11-15): a diagram of the unit at the start with every part named and numbered (buttons, screens, knobs, pads, groups, transport, 4D encoder, Smart Strip), so that the rest of the manual can say "button 5" or "group G"; the manual checked against the code (the instruments, "where Octachrome keeps your things", "if something is odd", "not yet"); the page and all its pictures in the matte theme; recipes 16 (SAMPLING) and 17 (BROWSER).
4. **The public site (guide and manual).** Asked on 2026-11-15: the guide and the manual as one introduction to the project that can be published, well connected, in the matte theme (matte first, then glossy, then chrome; brand stays apart). Done: the sources are templates with a shared header, footer and style; `scripts/build-site.py` builds `site/` (pages, 404, `robots.txt`, `sitemap.xml`, the sharing image, social tags, the twin as "Try it"); `scripts/check-site.py` checks links, anchors, images and metadata and runs in `cargo test` (F-G05); the Pages workflow builds, checks and publishes it; the manual works on a phone. The home page now says what Octachrome is, what you need (the MK3 and a Pi 5, with a Pi 4 as the floor) and where it stands, in plain words for someone who has never heard of it. To do: look at it on a phone and a laptop once published; **to publish**, fill in the address of the site and of the repository (`guide/site.json`, or `OCTACHROME_SITE_URL` and the repository variable `OCTACHROME_REPO_URL`) and enable Pages (Settings, Pages, source GitHub Actions; a public repository or a plan that includes Pages), then look at it once on a phone and a laptop.
5. **Eight pieces and the order of the instruments.** Asked on 2026-11-15: eight musical examples to hear what the machine can do, and the MIDI out last. Done: the factory projects are house, boom bap, dub, polyrhythm, acid, electro, ambient and breaks (levels matched, F-F07); PLUG-IN and the standard tracks run drums, KickSyn, FmBass, resonator, sampler, granular, macro, MIDI out; an empty place of the player's in FILE opens a new empty project; a pad that closes FILE or PLUG-IN no longer toggles a step when it is let go. To do: tune the pieces and the presets by ear on the unit.
6. **Navigation with the 4D encoder.** Asked on 2026-11-15: every screen has items that can be reached with the central 4D encoder, global and easy. Decided: a focus that is always visible (a ring and a dot), the encoder turns it, the arrows move it across the screens, push activates or grabs a value, and the tempo is only turned while TEMPO is held. Done (2026-11-15; no dot, only the frame): the mechanism and every view: FILE, PLUG-IN, BROWSER, SAMPLING (with its four values) and composing (the screen buttons 1–8, the tracks A–H, the pads of the grid and the eight values under the screens, which push grabs and the 4D edits), and an opened piece shows its music (F-N01 to F-N10). Not yet tried on the unit. 2026-11-16: the frame of a track hugs its round capsule, and PERFORM works with the 4D (the focus visits the sixteen effects where they are drawn, push engages one and keeps it, turning is its intensity; Brain lists no pads) (F-N11, F-N12). 2026-11-16, after a photo of the unit: the ink over a coloured fill is made to read (`readable`, F-D19) and no ring goes off the screen, the ring of a screen button lying on its own capsule (F-N13). 2026-11-16: the eight sounds of a drum kit can be chosen with the 4D from the right screen (F-N14). The step inspector's own items were added on 2026-11-16 (F-N15).
7. **After that.** Granular (reuses the sampler); KEYBOARD on a drum kit like PAD MODE; a basic mixer; naming and deleting projects; PATTERN, SCENE and ARRANGER; real-MK3 verification of everything marked "not yet tried" above.

## 9. What is left (2026-11-17)

Saved on 2026-11-16, after the batch of section 8, and kept current since. Development goes on first and everything is tried on the MK3 together afterwards.

**Development, in the order proposed**

1. Done (2026-11-16): the step inspector's own items: its components and values reached with the focus and the 4D, like every other view (F-N15).
2. Done (2026-11-16): granular, from the grain scheduler of Clouds (MIT) rewritten in Rust, on the sampler's bank of sounds (F-A21, F-I06), and on its own track F in the standard layout, in the unit, the eight pieces and a new empty project (F-I07). Not tried by ear on the unit yet.
3. Done (2026-11-16): KEYBOARD on a drum kit is PAD MODE (F-K17).
4. Done (2026-11-16): the basic mixer: a level and a pan for each track, turned with VOLUME held and the knobs (SHIFT for the pan), saved with the project (format version 3) (F-A20, F-K18, F-D20). The MIXER page and the master compressor came after (item 11). Sends came after (item 14).
5. Done (2026-11-16): names for projects and presets (by the day, and a text file the player can edit) and ERASE + a tap, twice, to erase one (F-F08, F-F09, F-K19). Naming on the unit itself came after (item 13).
6. Done: PATTERN (item 17), SCENE (item 18) and ARRANGER (item 19).
7. Done (2026-11-16): the sampler's gaps: copied WAVs are loaded when SAMPLING or PLUG-IN opens (F-A24), SHIFT + REC waits for the sound (F-A23), slices on the `loop` knob and `tune` of ± 24, which is also the root note (F-A22). Then (2026-11-17): `hits`, the fifth position of `loop`, cuts the window where the sound hits (F-A26).
8. Done (2026-11-16): SETTINGS in four pages, `theme`, `audio` (output, input, block), `midi` (port, test note, silence) and `light` (steps of backlight and pad level, never more than the supply allows), with the devices asked for live and kept in the settings file (F-C10 to F-C15). Not tried on the unit: the lists of devices of the Mac and the Pi, reopening the audio while it plays, and the brightness steps. Left: REC's count-in. MIDI clock out came after (item 10); MIDI in is not built.
9. Done (2026-11-16): the count-in of SAMPLING (TAP turns it on, REC counts three beats: F-K20) and the cut that moves while the part plays (F-A25). The count-in of the REC that writes notes is still to do (with chords and knob movements).
10. Done (2026-11-17): MIDI clock out: start, 24 clocks to the quarter note and stop on the MIDI port, on by default, with a button on the `midi` page of SETTINGS (F-M03, F-C16). Not tried against real gear. MIDI in came after (item 15). Left: a start that waits for the bar when it is turned on while playing (the song position pointer out is not sent: the unit always starts from the top).
11. Done (2026-11-17): the MIXER button opens the mixer on both screens (four channels, the levels and pans with the knobs; the compressor of the master on the right: threshold, ratio, attack, release, make-up and mix), saved with the project (format version 4) (F-K21, F-A27). Not tried by ear on the unit. The side-chain came after (item 16).
12. Done (2026-11-17): the master EQ, the third page of MIXER (button 7): a low shelf, a middle band with its frequency and a high shelf of ±15 dB, before the compressor, saved with the project (format version 5) (F-K22, F-A28). Not tried by ear on the unit.
13. Done (2026-11-17): naming on the unit: SELECT held and a tap on one of your own places in FILE or BROWSER opens NAME, a text screen of twelve places where the 4D chooses the letter, the arrows and the pads move the cursor and push saves (F-N16, F-N17). Not tried on the unit. Left: naming a sound that is not a recording, and a longer name; recordings came after (item 16).
14. Done (2026-11-17): sends: each track sends part of itself (after level and pan) to a ping-pong delay in step with the tempo and to a reverb, with the knobs under the left screen when `delay` or `reverb` is lit, and a page `sends` for the effects (time, feedback, size, tone); saved with the project (format version 6) (F-K23, F-A29). Not tried by ear on the unit. More effects on the sends are left; the send taken before the fader came after (item 16).
15. Done (2026-11-17): MIDI in: an input chosen on the `midi` page of SETTINGS (none by default) plays the selected track or, with `in: by channel`, the track of each channel (1 to 8 are A to H); REC writes what comes in; controllers 7, 10 and 91 mix the track (F-M04, F-C17). Not tried with a real keyboard, and the Linux branch has not been compiled yet. Left: aftertouch. Learning controls, the outside clock, program changes and pitch bend came after (item 16).
16. Done (2026-11-17): the small ones: the compressor can listen to one track (a side-chain, the third value of `output`: F-K24, F-A30) and each send can be taken before the fader (the page `taps`, button 8 again); MIDI in has program changes (the eight of PLUG-IN), the pitch wheel (±2 semitones; FM bass, macro, resonator, sampler, granular and the MIDI out), the outside clock (tempo, start, continue, stop and the song position pointer, kept in step by the pulses' own times: F-M06) and learned controls (any controller to any of the eight knobs of the instrument: F-M07); SAMPLING names its recordings (F-N18). The project is format version 7. Not tried with real gear: the clock was checked against simulated pulses stamped as the input thread stamps them, not a real sequencer, and a start or stop is acted on at the next frame; the Linux branch of the timestamps has not been compiled. Left: a start that waits for the bar, aftertouch, naming a sound that is not a recording.
17. Done (2026-11-17): PATTERN: sixteen patterns for each track (steps, length, division and kit); the pads of PATTERN show one (at once) and play it from the next loop of the track, SHIFT + a tap at once, hold + tap copies, ERASE + tap empties; saved with the project (format version 8) (F-S15, F-K25). Not tried on the unit. Left: the pattern is not shown while composing (only in PATTERN), and SCENE and ARRANGER, which come next in that order (a scene is the pattern of each track and the mutes; the song is a chain of scenes with repeats, and goes round at its end).

18. Done (2026-11-17): SCENE: sixteen scenes in the project, each the pattern every track shows and the mutes; a tap on a pad brings one back (each track at the start of its own next loop, the mutes at once), SHIFT + a tap stores, ERASE + a tap empties; saved with the project (format version 9) (F-S16, F-K26). Not tried on the unit. Left: ARRANGER, the song as a chain of scenes with repeats that goes round at its end.

19. Done (2026-11-17): ARRANGER: a song of up to sixteen steps, each a scene for 1 to 16 bars; when it is followed (knob 7) the sequencer plays the steps one after another and goes back to the first at the end; saved with the project (format version 10) (F-S17, F-K27). Also fixed: a pad tap in a view other than composing could open the step inspector when the mode was STEP (the hold was counted from time zero). Not tried on the unit. Left then: the mutes that a scene sets while the song plays were not drawn, a step could not be inserted, and the top bar did not say the pattern. Done after (item 20).

20. Done (2026-11-17): the three that item 19 left: the top bar of composing says the shown pattern and the step of the song that plays (`p2>3 s3`), the mutes a playing song's scene sets are drawn and lit (`EngineStatus::muted`), and SHIFT + a tap in ARRANGER puts a step in front of the one tapped (F-K27). Not tried on the unit.

21. Done (2026-11-17): the unit starts empty (`demo::start_rig`: the eight standard instruments and nothing in the steps) and a piece is loaded from FILE; the manual and the twin follow (F-I07). The simulator (`octachrome sim`) and the bench still use the demo project.
22. Done (2026-11-17): the harmony of each scene (project format 11): a scene stores the key, scale, mode and chords of the Brain; launching it brings them back at once, and while the song is followed the scene of the step brings its own to the tracks linked to the Brain, counted from the start of the step (F-S16, F-S18; the twin too). Known limit: while the song plays, the Brain pages draw the project's own chords (the chord that sounds is the right one: top bar and rings follow it), as with the mutes. Next: house as a complete song.
23. Done (2026-11-17): house is a whole song of about two minutes (60 bars at 124): seven scenes (intro, groove, main, break, build, drop, outro) in a song that is followed from the start, five patterns for the drums and more for the others, other chords in the break and the build, and every instrument that sounds takes part (F-F08; the twin too). With it, MUTE works while a song plays: a track is silent if its own mute is on or its scene silences it. The other seven pieces are still one pattern each; they are next, one at a time, as Javi listens to house first.
24. Done (2026-11-17): the song can be steered and read: a tap on a step while the song plays jumps there at the next bar (SHIFT + tap: at once), a step moves (hold a pad and tap another, or knob 4), and a step is drawn as two lines (`scene 2` / `4 bars`) with a bar that grows while it plays and `playing step N · bar X of Y` on the right screen (F-K27; the twin too). Not tried on the unit.
25. Done (2026-11-17): house is composed again as a deep Chicago house (122 bpm, 67 bars) and balanced by measurement: each track was rendered alone and the first version was found shrill (the granular and the sampler were the loudest, in the highest octaves) with the chords buried (the quietest) and a kit that was all hats. Now the chords are an electric piano (macro `fm`) and a soft pad (sampler bell), the hook a plucked string, the cloud a low granular, and the harmony only minor sevenths (Am7, Dm7, Em7) because the instruments stack a minor seventh. A bass that never sits on a kick, accents and ghost notes, rhythms that change from bar to bar (Every), a two-part build, a bar of breath before the drop, fills, and a second drop; eleven scenes in twelve song steps (F-F08, F-F10; the twin too, without the locks). **Not yet heard by Javi**: only measured. The other seven pieces are still one pattern each.
26. Done (2026-11-18): the sampler's sounds play the note they are given (F-A31). They were made on whatever pitch each had (the bell on G4, the glass on E5, the pluck on A3, the pad on G3) and played as if they were on C, so every piece that used them was out of the key; the bell's partials (2.76 and 5.4 times the first) clashed in a chord, and the voice's vibrato grew wider as the note went on. Now the five with a pitch are made on a C, the bell is tuned to play chords, the voice's vibrato is steady, the granular follows (its grains are cut from them), the `bells` preset of the granular no longer tunes up by 4.8 semitones, and house's cloud is an octave down instead of a fraction of one (the twin too).
27. Done (2026-11-18): the second piece is a whole melodic techno song in place of boom bap (F-F11; the twin too, without the locks). Asked by Javi: the seven pieces after house redone from nothing, in current styles of electronic music, one at a time, each heard by him before the next. The plan, in this order: techno, then UK garage, drum & bass, trance, amapiano, lo-fi and ambient, in place of dub, polyrhythm, acid, electro, breaks and ambient. Each piece brings its own knobs and kit (`KNOBS` and `KITS` in `factory.rs`), set by rendering every track alone and measuring it. **Not yet heard by Javi.**
28. Done (2026-11-18): the other six pieces are whole songs too, made from nothing in current styles, as Javi asked after hearing techno (he found it right and asked for the rest): garage (UK 2-step), dnb (liquid drum & bass), trance, amapiano (the KickSyn as the log drum), ambient and lo-fi, in place of dub, polyrhythm, acid, electro, ambient and breaks (F-F13 to F-F18). Each was voiced by rendering every track alone and measuring it, which found instruments out of the key (the resonator's bells ring on partials outside it, a snare on D#) and too loud or shrill (a reese at -14 dB, a phase distortion at 3.6 kHz). The twin now plays the pieces from a copy written from `factory.rs` and checked by a test (F-F12), instead of a port by hand; with it, the twin's blank project no longer keeps the patterns, scenes and song of house. **Not yet heard by Javi.**
29. Done (2026-11-18): the project is called **Octachrome** (it was OP-8). Chosen with Javi after a search of GitHub and the web for collisions: `Prism-8` was free but is lost among Prism 8 (GraphPad, WPF) and a crowded audio brand, `Eightfold` has a VCV Rack module and a company, `Chroma-8` collides with the ARP Chroma and Arturia's Chroma; *octa* (eight) + *chrome* (colour) is the eight-colour mark. Everything moved: the repository, the crates (`octachrome-*`), the command, the environment variables (`OCTACHROME_*`), the twin (`octachrome-twin.html`), the brand page and the social image, the skill and the documents. What a player saved under the old name is kept (F-F19, F-F20): `~/.config/op8` moves to `~/.config/octachrome`, projects are `.ocp` (the old `.op8p` and its magic still open), the twin moves its browser keys. A trademark search in an official register is still to do before the name is announced.
30. Done (2026-11-18): the twin and the manual draw the real panel of the MK3, measured on a photo of Javi's own unit (the manual's map had the left block, the pads, the knobs and the groups out of place; the twin grouped its buttons by function). One source, `guide/mk3.js`; the twin places every part from a copy of it, checked by a test (F-F21), with the keys the machine does not use yet dark; on a phone it scrolls sideways at a readable size. **Not yet seen by Javi.**

**To try on the unit, all together**

- The audio input and SAMPLING; BROWSER and the factory presets; the eight pieces; the focus of the 4D everywhere (PERFORM included, and the 4 % step of its intensity turn); ink over colour and the rings at the edges (F-D19, F-N13); the sounds of a kit chosen with the 4D (F-N14); the themes.
- Tune the presets and the pieces by ear, with what is heard on the unit.

**Outside the code**

- The CI of GitHub does not start (billing of the account, seen 2026-11-16): the last pull requests were merged after running `cargo test --workspace` locally.
- Publishing the site: done on 2026-11-18 (the repository is public and Pages serves the site). Left: the social preview image in the repository settings.
- Releases and upkeep: `CHANGELOG.md`, `release.yml`, `audit.yml`, `dependabot.yml` and `rust-toolchain.toml` are written but **not run yet** (CI is down, see above): the first tag `v0.1.0` is the test of `release.yml`. Left: a GIF or video of the twin and the unit, a recorded song, and a picture of the real unit in the README.
