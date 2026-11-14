# OP-8 — Features and Test Cases

The list of what OP-8 has to do, with the test case that verifies it. Each feature has an ID; the test file carries the same ID in a comment (`// F-S01`), so `grep -r "F-S01" crates/op8/tests` finds the case.

How to read each row:

- **What must happen:** the observable behaviour, in the user's language.
- **How it is verified:** what the bench measures (see [architecture.md §9](architecture.md)).
- **Status:** `✓` passes, `✗` fails (with the reason in the notes column), `R` needs manual review.

The buttons and their implementation status are in [buttons.md](buttons.md). This list is of **verifiable behaviour**, not of design.

## T · Transport (`tests/transport.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-T01 | PLAY starts the sequencer and the audio, and PLAY again pauses it | `status.playing` changes; there is signal after PLAY and silence after the pause | ✓ |
| F-T02 | STOP stops the sequencer; the audio falls to silence | `playing == false` and `silent` in the last stretch | ✓ |
| F-T03 | RESTART goes back to step 1 and keeps playing | the playhead of track A is 0 after RESTART | ✓ |
| F-T04 | Tracks of different length drift apart (polymeter) and the playhead never exceeds its length | track C (length 7): playhead always in 0..=6 and reaches 6 | ✓ |
| F-T05 | TAP works the tempo out from the taps (the average gap of the last ones), and a pause of more than two seconds starts over | `project.bpm` after taps at known times | ✓ |
| F-T06 | SHIFT + TAP turns the metronome on and off: it clicks on every beat of a running sequencer and is silent when off | peak of the audio | ✓ |

## S · Sequencer (`tests/sequencer.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-S01 | In STEP, tapping a pad toggles its step | `steps[n].active` toggles; the LED goes from level 1 to 3 | ✓ |
| F-S02 | Holding a pad for more than 300 ms opens the step inspector and does not toggle it | `inspect_step == Some(n)`; the step does not change on release | ✓ |
| F-S03 | ERASE + pad erases the step | with ERASE held, the step ends up inactive | ✓ |
| F-S04 | Pads past the track length are off | track of length 7: pad of step 8 with its LED off | ✓ |
| F-S05 | Tapping a group A–H selects the track; its LED is brighter than the others | `ui.track` and the group LED level | ✓ |
| F-S06 | Knob 1 in STEP changes the track length | `tracks[t].length` goes up and down, limited to 1–16 | ✓ |
| F-S07 | MUTE + group mutes the track | `tracks[t].muted` toggles | ✓ |
| F-S08 | A muted track emits no notes | track E (MIDI) muted: no new notes after the mute | ✓ |
| F-S09 | A drum-kit track has eight lanes: a step plays one note per lane that is on (several sounds in one step), `SetKit` turns a track into a kit (existing notes become lanes by pitch class) and `ToggleLane` switches one lane ([instruments.md §7](instruments.md)) | `op8-seq` unit tests | ✓ |
| F-S12 | Shifting a track turns its steps round inside its length (they wrap round the end of the bar; those beyond the length stay put) | `Command::ShiftTrack` on the project | ✓ |
| F-S13 | The players: a chord player sounds the whole chord on the note of a step, together or strummed over the step; an arpeggio plays its notes one after another inside the step, in order (up, down, up-down, random) and carries on round the chord across steps; the chord stacks over 1–3 octaves and stays inside the MIDI range; a drum kit ignores the players | note events of the sequencer; `op8-seq` unit tests | ✓ |
| F-S14 | The players page (button 3): knobs 1–4 set `play`, `chord`, `rate`/`strum` and `octaves` and stop at the ends of their lists; a drum kit's track takes none of it; with a chord on, a step sounds three notes at once, and with an eight-note arpeggio one step sounds eight; both pages (players, bar) draw something | `project().tracks[..].players`, `status().note_count`, lit pixels | ✓ |

## B · Brain (`tests/brain.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-B01 | CHORDS switches to Brain mode; the left screen changes | `ui.mode == Brain`; the left screen hash differs | ✓ |
| F-B02 | Holding a pad in Brain plays a live chord and releases it on lift | `brain.live` is `Some` during the press and `None` after | ✓ |
| F-B03 | SHIFT + group connects or disconnects the track from the Brain | `brain_link` toggles | ✓ |
| F-B04 | Knob 1 in Brain changes the key, wrapping from 0 to 11 | `brain.key` goes up a semitone per detent | ✓ |
| F-B07 | A CHORDS pad sounds the chord (all its notes) on the selected track's instrument while it is held, and still sets the live chord for the Brain-linked tracks; a drum kit stays silent ([buttons.md](buttons.md)) | note count of the track | ✓ |

## P · Punch-in FX (`tests/punch.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-P01 | PERFORM switches to Punch mode | `ui.mode == Punch` | ✓ |
| F-P02 | A pad in Punch engages the effect while held and turns it off on release | `ui.punch` is `Some` during the press and `None` after | ✓ |
| F-P03 | There is always a way back to the sequencer: STEP leaves Punch and Brain, and pressing PERFORM or CHORDS again while in that mode goes back to STEP | `ui.mode` after the presses | ✓ |
| F-P04 | PAD MODE: on a drum kit pads 1–8 play the eight sounds (and are heard with the sequence stopped) and pads 9–16 are free; on any other instrument the pads climb the Brain's scale from its key; pressing the button again goes back to STEP ([buttons.md](buttons.md)) | note count of the track and `pad_note` | ✓ |
| F-P05 | KEYBOARD: the pads are a chromatic keyboard from C3, the 4D encoder ◀ ▶ moves the octave, a held pad sounds, and in settings the pads do nothing | note count and `pad_note` | ✓ |

## K · Knobs (`tests/knobs.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-K01 | Knob 5 changes parameter 1 of the selected track's instrument | `handle.param(track, 1)` changes with the sign of the turn | ✓ |
| F-K02 | With page 6 open, knobs 5–8 change parameters 5–8 | `right_page == 1`; `param(track, 4)` changes | ✓ |
| F-K03 | Touching a knob shows the large overlay on the right screen | `knob_touch[i]` true; the screen hash changes | ✓ |
| F-K04 | Each instrument exposes at most 8 parameters | `parameters().len() <= 8` for all cartridges (`builtin_names`) | ✓ |
| F-K09 | A kit step has no note: on a drum track the first knob of the step inspector does nothing and its cell is empty; a plain-pitch track keeps its note knob ([instruments.md §7](instruments.md)) | the step is unchanged after the knob | ✓ |
| F-K10 | On a drum kit the pads are the steps of the lane chosen on the right (`sound`): a tap switches that lane on or off in the step and leaves the other lanes as they are, so several sounds can share a step | `steps[..].lanes` after the taps | ✓ |
| F-K11 | Holding TEMPO, SWING or VOLUME and turning the 4D encoder turns that one (VOLUME is the master volume: half of it, quieter) and the left screen shows it big while it is held | `project` and `ui.master` | ✓ |
| F-K12 | FIXED VEL makes the pads and REC use the same velocity, whatever the strike | the velocity REC writes | ✓ |
| F-K13 | The bar page (button 4): knobs 1–4 are `length`, `division`, `shift` and `rec rounds`; the same button, or STEP / button 1, goes back to the steps; a shift keeps the steps inside the length | `project()`, `ui().left_page`, `ui().quantize` | ✓ |

## D · Screens (`tests/screens.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-D01 | Both screens show content on the first frame | more than 1000 non-black pixels on each screen | ✓ |
| F-D02 | The background is absolute black | more than 40 % of the pixels are `#000000` | ✓ |
| F-D03 | With the scene idle, the screen reports only a small part of its pixels as changed per frame | in 1 s with no input, under 5 % of a full frame per frame | ✓ (the original spec asked for zero pixels; changed because the screen animates by design, see [design.md §6](design.md)) |
| F-D04 | STEP and PERFORM look different | different hash between STEP and PERFORM; the mode changes | ✓ (returning to STEP cannot be compared by hash: the screen animates and never repeats the same frame) |
| F-D05 | The sending to the MK3 is exact: what is encoded is what the reference reader reconstructs, for any band of rows | `bands_round_trip` in `crates/op8-hw` (bands of 1 row, full screen and the middle; it does not write outside the band) | ✓ |
| F-D06 | A flat screen takes a few bytes and noise is not inflated | `flat_screens_are_tiny_and_noise_is_not_inflated`: 32 bytes for a black screen; the same as raw for noise | ✓ |
| F-D08 | The optional delta sending (`OP8_DISPLAY_DELTA=1`): a few changed pixels cost a small transfer, the screen ends up showing the new frame, and a pair is sent even if only one of its pixels changed | `a_small_change_costs_a_small_transfer_and_still_round_trips` and `changed_pairs_are_sent_even_when_only_one_pixel_differs` in `crates/op8-hw` | ✓ (not looked at on the real screens) |
| F-D09 | Each theme defines the eight brand colours and ends in its white; glossy and matte follow the rainbow order (red, orange, yellow, green, blue, indigo, violet), chrome is one range drifting one way ([design.md §2.3](design.md)) | `op8-gfx` unit test | ✓ |
| F-D10 | Each screen names what its buttons do, in a capsule centred on the button: `step`, `components`, `players`, `bar` (left) and `shape`, `sound`, `fx`, `mod` (right, in that order); `shape` only for an instrument with a second page, and a button with no function shows nothing; the open page is underlined; in settings the strip is empty ([design.md §4](design.md)) | pixels of the top 18 rows around each button; the open page has the `fg` outline | ✓ |
| F-D11 | On the step grid an active step and an empty one are far apart in brightness (the active at least 5 times brighter) | pixel luminance of both cells | ✓ |
| F-D12 | Screen button 1 goes back to the step view and 2 opens the components; both, and 3 to 8, do nothing in settings | `ui.mode` and `ui.inspect_step` | ✓ |
| F-D13 | The text in a capsule is centred in it by its ink, within a pixel horizontally and 2.5 px vertically ([design.md §3](design.md)) | pixels of the strip | ✓ |
| F-D14 | The LED of each of the eight screen buttons agrees with what the screens say: no label, no light; a label, dim; the open page, full; `shape` only for an instrument with a second page; all dark in settings ([design.md §4](design.md)) | `leds().slots` of `Screen(1..8)` against `screen_buttons` | ✓ |
| F-D15 | All eight screen buttons are named in the strip and lit; the fx and mod pages (right) draw, each of the six effects has its own picture, and the page dots show the pages that exist | pixel hashes and LED slots | ✓ |
| F-D07 | On the real MK3, the two animated screens look complete and in step | **manual review** with photo and video; `OP8_DISPLAY_STATS=1` gives the figures | R |

## L · LEDs (`tests/leds.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-L01 | PLAY lights green when playing and dim when stopped | palette `Green`, level 3 vs 1 | ✓ |
| F-L02 | The step pads show the track colour: active bright in the track colour, inactive a dim white (F-L07) | level 3 with the colour of `track_led_colors` (glossy) | ✓ |
| F-L07 | An empty step lights its pad as a dim white (level 1), not a dim colour, so it stays apart from an active pad under any cap | palette `White`, level 1 | ✓ |
| F-L08 | In PAD MODE and KEYBOARD the pads that play are lit in the track colour, a held pad is white, and on the keyboard every C is brighter | pad LED bytes | ✓ |
| F-L09 | Every button follows one rule: dark if it does nothing in the context, a very faint white if it does something, a strong white while it is pressed or has the focus (the mode in use, CHANNEL or SETTINGS, SHIFT held…); in settings MUTE, ERASE and the screen buttons are dark and the transport still works ([design.md §2.6](design.md)) | LED slots of the buttons | ✓ |

## M · MIDI (`tests/midi.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-M01 | Track C emits well-formed note-on and note-off on a single channel; also CC74 (filter) when a parameter changes | status byte `0x8n`/`0x9n`/`0xBn`, data 0..127, a single channel, controller 74 for the CCs | ✓ |
| F-M02 | No lost events during the demo | `dropped_events == 0` after 7 s of demo | ✓ |

## A · Audio (`tests/audio.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-A01 | Silence before PLAY | RMS of the first 0.2 s of the demo without PLAY below −90 dBFS | ✓ |
| F-A02 | The demo sounds | RMS between 2 and 7 s above −30 dBFS | ✓ |
| F-A03 | No digital clipping | absolute peak below 0.99 across the whole demo | ✓ |
| F-A04 | An empty project makes no noise | RMS of 1 s of playback with no notes below −90 dBFS | ✓ |
| F-A05 | The same script gives exactly the same audio | two runs of the demo produce identical samples | ✓ |
| F-A06 | The audio stream comes back by itself when its device fails. The sequencer's clock runs in the audio callback, so without this PLAY lights up and nothing moves | `Keeper` in `crates/op8/src/audio.rs`: restarts after a failure, retries about once a second while the device is missing, reports one failure once, and resumes when the device returns | ✓ |
| F-A07 | On the real MK3, pulling the USB cable and plugging it back: screens, audio and PLAY recover without restarting the app | **manual review**, with `OP8_LOG_EVENTS=1` | R |
| F-A08 | drums: each of the eight sounds makes sound on its own note, and the note picks the sound by its pitch class (C kick, C# snare, D closed hat, D# open hat, E tom, F clap, F# rim, G crash; above G they wrap round); only the struck sound wakes ([instruments.md §7](instruments.md)) | `op8-cartridges` unit tests (`drums.rs`) | ✓ |
| F-A09 | drums: the knobs edit the chosen sound (the `sound` knob), and each sound keeps its own values, also through save and load | `op8-cartridges` unit tests | ✓ |
| F-A10 | drums: silent without notes, and all eight at once stay within full scale | `op8-cartridges` unit tests | ✓ |
| F-A11 | A choice is heard at once, without playing the sequence: turning a knob of the selected instrument or `sound`, turning the note of a step, and tapping a drum step all play the note (`EngineCommand::Audition`) | peak of the track's audio with the sequence stopped | ✓ |
| F-A12 | A choice is auditioned once, not over and over: the `sound` knob plays each new sound once, and a continuous knob plays nothing while it moves and one note when it has been still for 250 ms | note count of the track | ✓ |
| F-A13 | resonator: both kinds (string, bells) ring when struck; four notes ring together and a fifth takes the oldest voice; silent without notes and bounded with every knob at maximum ([instruments.md §7](instruments.md)) | `op8-cartridges` unit tests (`resonator.rs`) | ✓ |
| F-A16 | sampler: the eight sounds that come with it exist, are all different, and are made the same way every time; a note plays its sound once at its own pitch (an octave up takes half the time); `start` and `length` cut it; one shot ignores the note-off while a loop holds until it and fades over `release`; eight voices and a ninth takes the oldest; the player's WAV files (any rate, 8–32 bit or float, mono or stereo) fill the other eight places in the order of their names, and what is not a sound is skipped; an empty place is silent; the picture shows the wave and the voices; a WAV is read and brought to 48 kHz, and what is not a WAV is refused | `op8-cartridges` unit tests (`sampler.rs`, `wav.rs`) | ✓ |
| F-A14 | The effect of a track (button 7): off leaves the sound as it is; delay and reverb leave a tail after the sound and no mix leaves none; a low-pass takes out the highs; every effect stays finite and bounded with its knobs at 0, half and full; knob 5 chooses the effect (and stops at the ends), knobs 6–8 move its three knobs, and the delay fills the gap after a kick; it belongs to one track and is saved with it | `op8-engine` unit tests (`track_fx.rs`), the audio of a bench between two kicks, `project().tracks[..].fx` | ✓ |
| F-A15 | The LFO of a track (button 8): it swings a knob of the instrument around where the player left it, touches no other knob, and puts it back when it is turned off; on the volume it dips the track from full volume and never raises it; the page's knobs set `shape`, `rate`, `depth` and `moves`, and the footer names the knob it moves | `op8-engine` unit tests, `project().tracks[..].lfo`, `ui().lfo_target` | ✓ |

## G · Scripts (`tests/script.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-G01 | The demo script parses and ends at 6.8 s | `end_ms() == 6800` | ✓ |
| F-G02 | A script error gives the line number | message with `line N` for each malformed case | ✓ |
| F-G03 | The `shot` captures are generated with their name | PNG files created in `OP8_BENCH_DIR` | ✓ |
| F-G04 | Every recipe of the manual (`guide/manual.html`) works as written: following its buttons in order gives what it promises (a drum beat, a bass that follows the chords, a recorded melody, a ratchet, a chord and an arpeggio, an effect, an LFO, two lengths drifting apart, PERFORM, PLUG-IN, FILE, TAP and the metronome, the theme) | `crates/op8/tests/manual.rs` on the bench; its captures are the manual's pictures | ✓ |

## C · Settings (`tests/settings.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-C01 | SETTINGS takes both screens, lights its LED, dims CHANNEL's, and the pads go dark | both screen hashes change; LED levels; every pad LED off | ✓ |
| F-C02 | CHANNEL goes back to composing and keeps the pad mode; pressing SETTINGS again does too | `ui.main`, `ui.mode`, LED levels | ✓ |
| F-C03 | In settings the 4D encoder and ◀ ▶ change the theme, wrapping, and the screens change; outside settings the encoder is still the tempo | `theme_id()`, the right-screen hash, `bpm` | ✓ |
| F-C04 | STEP, CHORDS and PERFORM leave settings and choose their pad mode | `ui.main` and `ui.mode` | ✓ |
| F-C05 | In settings the pads, knobs and groups do nothing, and PLAY still works | the step, the parameter and the track are unchanged; `playing` | ✓ |
| F-C06 | The settings file: every theme survives a round trip, a damaged file falls back to the default, names match the command line, saving creates the folder | unit tests in `crates/op8/src/settings.rs` | ✓ |
| F-C07 | A theme change is saved to the settings file, and only when a path is set (never in the simulator or the bench) | the file's content after each change | ✓ |
| F-C08 | On the real MK3: the SETTINGS button reaches the app and its LED lights, the 4D encoder changes the theme on both screens, the choice survives a restart, and `-t` overrides it | **manual review**, with `OP8_LOG_EVENTS=1` | R |
| F-C09 | The theme changes the pad and group LEDs, not only the screens: track B's pad and group key take that theme's colour from `track_led_colors`, and the three themes do not light the pads alike | `crates/op8/tests/settings.rs` | ✓ |

## I · Instruments (`tests/plugin.rs`)

| ID | Behaviour | How it is verified | Status |
| :-- | :-- | :-- | :-- |
| F-I01 | PLUG-IN shows the sixteen places on both screens (the factory instruments from the most guttural to the most refined, then eight for installed ones), marks the instrument of the selected track, and a pad puts that instrument on the track and goes back to composing; PLUG-IN again leaves | the instrument of the track | ✓ |
| F-I02 | An instrument that is not built yet, and an empty place, say so and change nothing | the instrument is unchanged | ✓ |
| F-I03 | The MIDI out is one bridge that owns the MIDI queue: choosing it moves it from the track that has it | the instruments of both tracks | ✓ |
| F-I04 | Leaving the drums turns a kit track's lanes back into notes, so the pattern is not lost | the steps of the track | ✓ |
| F-I05 | The sampler can be chosen in PLUG-IN, has two pages (`sample`, `start`, `length`, `tune` and `attack`, `release`, `loop`, `level`), plays a sound from a step, and the right screen draws its wave | `instrument(track)`, the track's audio and note count, `ui().has_second` | ✓ |

## F · Files (`tests/files.rs`, `crates/op8-seq`, `crates/op8`)

| ID | Behaviour | How it is verified | Status |
| :-- | :-- | :-- | :-- |
| F-F01 | A project round-trips through bytes (`op8_seq::io`), and garbage, a truncated file or another version is refused | unit tests | ✓ |
| F-F02 | A project file (the project and each track's instrument name and state) round-trips, and a damaged or other-version file is refused | unit test (`projectfile.rs`) | ✓ |
| F-F03 | The eight factory projects load and differ from each other | unit test (`factory.rs`) | ✓ |
| F-F04 | FILE shows the sixteen slots (the factory projects in the top two rows) on both screens; a pad opens the project, with its instruments, and goes back to composing; FILE again leaves | the project in the engine after the tap | ✓ |
| F-F05 | SHIFT + a pad of the player's slots saves into it (a file in the projects folder), and a tap opens it again with the tempo and the instruments' knobs | the files and the project after reopening | ✓ |
| F-F06 | An empty slot says so and does not load; SHIFT on a factory slot does not overwrite it; a damaged file is refused and the project stays as it was | the project is unchanged | ✓ |
| F-F07 | The players, the effect and the LFO of every track are saved with the project and come back with it; a project saved before they existed (version 1) still opens, with them off | save, open another project, open it again; `op8-seq` unit test of a version 1 file | ✓ |

## R · Recording (`tests/rec.rs`)

| ID | Behaviour | How it is verified | Status |
| :-- | :-- | :-- | :-- |
| F-R01 | REC arms and disarms (strong white while armed, faint otherwise, a red mark on the left screen), and does nothing in settings | `ui.rec` | ✓ |
| F-R02 | A note played in PAD MODE or KEYBOARD while the sequencer runs is written at the nearest step (early in a step it stays, late in it goes to the next) with its note and velocity | the step after the strikes | ✓ |
| F-R03 | On a drum kit REC switches on the lane of the sound played, several sounds can share a step, and the track becomes a kit | `steps[..].lanes` | ✓ |
| F-R04 | Nothing is written when REC is not armed, when the sequencer is stopped, or for CHORDS pads | the steps stay empty | ✓ |
| F-R05 | REC writes where the bar page says: `nearest` (late in a step it goes to the next), `down` (the step that is sounding) or `up` (the next one) | the step REC writes into | ✓ |

## H · Hardware link (`crates/op8-hw`, `crates/op8`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-H01 | The optional screen byte-rate limit (`OP8_DISPLAY_KBPS`, off by default) keeps sustained sending under the rate it is given and spreads a burst out. It is not needed: the resets were the backlight, not the traffic ([roadmap.md](roadmap.md)) | `Pacer` unit tests in `crates/op8-hw/src/mk3/display.rs`: a burst passes, the rest is spread, the long-run average stays under the limit, and 0 means no limit | ✓ |
| F-H02 | After a failure the app waits before reconnecting, and waits longer while failures keep coming (4, 8, 16, then 30 s), starting over after a steady minute. Reconnecting at once made the MK3 fail again: the screens and the sound switched on and off in a loop | `Backoff` unit tests in `crates/op8/src/backoff.rs` | ✓ |
| F-H04 | The app reads the current the host grants the MK3 and picks a plan: below 500 mA limited (screens 50), from 500 mA full (screens 80), unknown counts as limited. It reads the MK3's own entry and never another device's | `parse_ioreg` and `plan_for` tests in `crates/op8-hw/src/mk3/power.rs`, against the shape of the real `ioreg` output | ✓ (the 500 mA threshold is provisional, see F-H06) |
| F-H05 | On a limited supply the LEDs are capped before they are written: colour LEDs (pads, groups, PLAY…) at level 2 keeping their colour, the others at 60 % but never rounded down to off; unlit LEDs stay unlit; a full supply changes nothing | `Plan::limit` tests in `crates/op8-hw/src/mk3/power.rs` | ✓ |
| F-H06 | With the MK3 plugged straight into the Mac: `power:` shows what the host grants, and the plan it picks holds five minutes of PLAY without a reset. If it picks full, brightness 80 must hold | **manual review**, `scripts/run.sh -l`; adjust `FULL_POWER_MA` from the value seen | R |
| F-H07 | The theme asks for light within what the supply allows: glossy takes all the backlight the plan gives, chrome less, matte least; matte caps the pad LEDs at level 2; no theme gets more than the plan ([design.md §2.5](design.md)) | `op8-hw` unit test (`power.rs`) | ✓ |
| F-H03 | On the real MK3, five minutes of playing, turning knobs and changing modes and themes with no reset and no reconnect | **manual review**: `OP8_LOG_EVENTS=1` and `OP8_DISPLAY_STATS=1`; the log has no `reconnecting` line | R |

## Manual review (cannot be automated)

- That the sound is pleasant. The bench measures presence and level, not timbre.
- That the screens are legible at a distance. The bench sees pixels, not reading.
- That the LED colours match the real MK3. Only red and orange are confirmed.
- That the physical MK3 responds to every control. That is `hw-test` and `run --script` with the MK3 connected.
