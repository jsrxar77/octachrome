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
| F-D10 | Each screen names what its buttons do, in a capsule centred on the button: `step`, `components`, `players`, `bar` (left) and `sound`, `shape`, `fx`, `mod` (right, in that order); `shape` only for an instrument with a second page, and a button with no function shows nothing; the open page is underlined; in settings the strip is empty ([design.md §4](design.md)) | pixels of the top 18 rows around each button; the open page has the `fg` outline | ✓ |
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
| F-M03 | MIDI clock out: PLAY sends start (`0xFA`) once and then a clock (`0xF8`) every 1/24 of a quarter note at the project's tempo (the gaps are exact to a sample and follow a change of tempo at once); STOP sends stop (`0xFC`) once and the clocks cease; RESTART sends a new start; nothing is sent while stopped | `Bench::sync_log` (sample time, byte) | ✓ |

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
| F-A17 | SAMPLING records: the engine takes the master (or the audio that comes in) into a buffer made once, from the moment it is asked to the moment it is stopped, and stops by itself at ten seconds; a recording saved as `rec-N.wav` is the sampler's place N whatever else is in the folder (the other WAV files fill the places that are left); a place can be filled while the sampler plays; a pad of the top two rows writes the kept part (faded at both ends, brought to the same level) as `rec-N.wav` through a temporary file, and a sampler that is playing has it at once; it says so when there is no take, while it records, or with no folder | `op8-engine` and `op8-cartridges` unit tests, and the bench (`tests/sampling.rs`) reading the saved file and the track's audio | ✓ |
| F-A18 | The audio input: with SAMPLING on `input`, the program opens the interface's input (the MK3's if it is there, `OP8_AUDIO_INPUT` to choose another), the level moves with what is played into it, and what is recorded is that sound; the stream is opened only then, and it is reopened if the device goes away | Manual review (needs the unit or a microphone; the macOS microphone permission) | R |
| F-A14 | The effect of a track (button 7): off leaves the sound as it is; delay and reverb leave a tail after the sound and no mix leaves none; a low-pass takes out the highs; every effect stays finite and bounded with its knobs at 0, half and full; knob 5 chooses the effect (and stops at the ends), knobs 6–8 move its three knobs, and the delay fills the gap after a kick; it belongs to one track and is saved with it | `op8-engine` unit tests (`track_fx.rs`), the audio of a bench between two kicks, `project().tracks[..].fx` | ✓ |
| F-A15 | The LFO of a track (button 8): it swings a knob of the instrument around where the player left it, touches no other knob, and puts it back when it is turned off; on the volume it dips the track from full volume and never raises it; the page's knobs set `shape`, `rate`, `depth` and `moves`, and the footer names the knob it moves | `op8-engine` unit tests, `project().tracks[..].lfo`, `ui().lfo_target` | ✓ |

## G · Scripts (`tests/script.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-G01 | The demo script parses and ends at 6.8 s | `end_ms() == 6800` | ✓ |
| F-G02 | A script error gives the line number | message with `line N` for each malformed case | ✓ |
| F-G03 | The `shot` captures are generated with their name | PNG files created in `OP8_BENCH_DIR` | ✓ |
| F-G05 | The public site builds (`scripts/build-site.py`) and holds together (`scripts/check-site.py`): the home page, the manual, a 404, `robots.txt`, `sitemap.xml`, the sharing image and the twin are in `site/`; no broken link, anchor or image; every page has a title, a description and one heading; no template marker is left; the brand page is not in the sitemap; the placeholders for the site's address are warnings | `crates/op8/tests/site.rs` runs both scripts | ✓ |
| F-G04 | Every recipe of the manual (`guide/manual.html`) works as written: following its buttons in order gives what it promises (a drum beat, a bass that follows the chords, a recorded melody, a ratchet, a chord and an arpeggio, an effect, an LFO, two lengths drifting apart, PERFORM, PLUG-IN, FILE, TAP and the metronome, the theme, a sound recorded with SAMPLING, a preset saved and loaded in BROWSER, the focus of the 4D, the mix) | `crates/op8/tests/manual.rs` on the bench; its captures are the manual's pictures | ✓ |

## C · Settings (`tests/settings.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-C01 | SETTINGS takes both screens, lights its LED, dims CHANNEL's, and the pads go dark | both screen hashes change; LED levels; every pad LED off | ✓ |
| F-C02 | CHANNEL goes back to composing and keeps the pad mode; pressing SETTINGS again does too | `ui.main`, `ui.mode`, LED levels | ✓ |
| F-C03 | In settings the 4D encoder and ◀ ▶ change the theme, wrapping, and the screens change; outside settings the encoder does not change the theme, and it does not turn the tempo either unless TEMPO is held | `theme_id()`, the right-screen hash, `bpm` | ✓ |
| F-C04 | STEP, CHORDS and PERFORM leave settings and choose their pad mode | `ui.main` and `ui.mode` | ✓ |
| F-C05 | In settings the pads, knobs and groups do nothing, and PLAY still works | the step, the parameter and the track are unchanged; `playing` | ✓ |
| F-C06 | The settings file: every theme survives a round trip, a damaged file falls back to the default, names match the command line, saving creates the folder | unit tests in `crates/op8/src/settings.rs` | ✓ |
| F-C07 | A theme change is saved to the settings file, and only when a path is set (never in the simulator or the bench) | the file's content after each change | ✓ |
| F-C08 | On the real MK3: the SETTINGS button reaches the app and its LED lights, the 4D encoder changes the theme on both screens, the choice survives a restart, and `-t` overrides it | **manual review**, with `OP8_LOG_EVENTS=1` | R |
| F-C09 | The theme changes the pad and group LEDs, not only the screens: track B's pad and group key take that theme's colour from `track_led_colors`, and the three themes do not light the pads alike | `crates/op8/tests/settings.rs` | ✓ |

## I · Instruments (`tests/plugin.rs`)

| ID | Behaviour | How it is verified | Status |
| :-- | :-- | :-- | :-- |
| F-I01 | PLUG-IN shows the sixteen places on both screens (the factory instruments from the most guttural to the most refined with the MIDI out last: drums, KickSyn, FmBass, resonator, sampler, granular, macro, midi out; then eight for installed ones), marks the instrument of the selected track, and a pad puts that instrument on the track and goes back to composing; PLUG-IN again leaves | the instrument of the track | ✓ |
| F-I02 | An empty place says so and changes nothing | the instrument is unchanged | ✓ |
| F-I03 | The MIDI out is one bridge that owns the MIDI queue, on the last track (H) as the standard tracks follow the order of PLUG-IN: choosing it moves it from the track that has it | the instruments of both tracks | ✓ |
| F-I04 | Leaving the drums turns a kit track's lanes back into notes, so the pattern is not lost | the steps of the track | ✓ |
| F-K14 | SAMPLING (the button) takes both screens; REC records the master until REC again and the take has the length of what played; knob 5 chooses the source, 6 and 7 cut the take and cannot cross, 8 sets the fade; the bottom pads play the kept part and it ends by itself; SAMPLING again goes back to composing | `ui().main`, `ui().smp`, the audio the preview made | ✓ |
| F-D17 | Choosing a track puts both screens on their first page, whatever was open: the steps (button 1) and `sound` (button 5) are the lit ones; chrome is the default theme and the first of the list since 2026-11-16 (the settings view and the 4D encoder go chrome, matte, glossy); the guide and the manual are drawn in matte | the LED slots of the screen buttons, `ui().right_page`, `ui().left_page`, and the settings unit test | ✓ |
| F-K15 | BROWSER: SHIFT + a pad of the bottom two rows saves the instrument's knobs as `<presets>/<instrument>/user-N.preset` (through a temporary file); a tap loads them back and the engine keeps them as its own, and plays a note; each instrument has its own; it says so when there is nothing to load, when a damaged file is tapped, when the presets that come with it are written over, and when there is nowhere to save | `ui().browse`, the files, `param(track, i)` and the audio of the audition | ✓ |
| F-K16 | BROWSER's top two rows are the eight presets that come with the instrument, by name: a tap loads every knob the preset names and leaves the rest (the drums keep the chosen sound), plays a note, and each instrument has its own; a track with no instrument has none | `ui().browse`, `param(track, i)` and the audio | ✓ |
| F-A19 | The presets that come with the instruments (`presets.txt`): each of the six instruments has eight, every one plays (finite, not silent, not clipped), no two sound the same, the names are true (deep is lower than click, boom rings longer than tight, metal is brighter than sub, gong outlasts muted, the tick is short and the pad goes on), and the twin carries an equal copy of the text | `op8-cartridges` unit tests (`presets.rs`) | ✓ |
| F-N01 | In FILE the focus is always on one place; the 4D encoder turns it along the places (wrapping), its arrows move it across the grid (nothing to the right of the last column), the ring is drawn where it is, and pushing opens that place as tapping its pad does | `ui().focus`, the left screen's hash, the project in the engine | ✓ |
| F-N02 | The 4D encoder does not turn the tempo by itself: only while TEMPO is held | `bpm` | ✓ |
| F-N03 | PLUG-IN starts with the focus on the track's instrument; turning moves it and pushing puts that instrument | `instrument(track)` | ✓ |
| F-N04 | BROWSER: push loads the focused preset; SHIFT + push on the player's saves it | `ui().browse_note`, the file | ✓ |
| F-N05 | SAMPLING: the focus reaches the four values under the right screen; push grabs one, turning changes it (and the focus stays), push lets it go, and an arrow lets it go and moves; turning without grabbing moves the focus, not the value | `ui().focus`, `ui().grabbed`, `ui().smp` | ✓ |
| F-N06 | Opening a piece puts the screens on its music: the first track that has steps, its first page, and on a kit the sound that has steps | `ui().track`, `ui().lane`, `ui().right_page` | ✓ |
| F-N07 | Composing has a focus that visits everything: turning the 4D goes through the eight screen buttons, the eight tracks, the sixteen pads of the grid and the eight values (forty things, once each; a drum kit adds its eight sounds, F-N14) and comes back to where it started; the ring is drawn and the screens change as it moves | `ui().focus`, pixel hashes | ✓ |
| F-N08 | Pushing does what tapping does in composing: a screen button changes the page, a track is chosen, a pad of the grid toggles its step, SHIFT + push on a pad opens the step's inspector and the `step` button closes it | `ui()`, the project | ✓ |
| F-N09 | A value of a footer is grabbed with push, turned (the knob under it changes) and let go; without grabbing, turning moves the focus and the value stays | `param(track, i)`, `ui().focus` | ✓ |
| F-N10 | The arrows cross from the left screen to the right one; with KEYBOARD (on an instrument that is not a drum kit) ◀ ▶ stay the octave and the turn and ▲ ▼ move the focus | `ui().focus`, `ui().octave` | ✓ |
| F-N11 | In PERFORM the focus visits the sixteen effects and its frame is where each one is drawn (the pad that plays it is that effect); Brain, which draws no pads, lists none | `nav_items`, the position `punch_view` draws each slot at | ✓ |
| F-N12 | Pushing an effect of PERFORM engages it and keeps it; turning is its intensity and does not move the focus, the arrows do and the effect goes on; another push takes its place, the same one lets it go, and leaving PERFORM lets it go | `ui().punch`, `ui().focus` | ✓ |
| F-D19 | Ink over a colour always reads: for every theme, every track colour and every intensity of an effect (0 to 100 %), the ink over the fill that `readable` returns has a contrast of at least 4.5:1 | unit test in `op8-ui` | ✓ |
| F-N13 | Every ring of every view is inside the screen, and the ring of a screen button is on the capsule that is drawn for its label | unit test in `op8-ui` over every view, pad mode, page and instrument kind | ✓ |
| F-N14 | A drum kit draws its eight sounds on the right screen and the 4D reaches them: the focus visits them, pushing one chooses it (the right screen shows it large, the lane of the steps is its) and plays it; an instrument that is not a kit lists none | `nav_items`, `ui().lane`, pixel hash | ✓ |
| F-N15 | The step inspector has its own items: the focus opens on the first component, visits the components and the `+`, a component is grabbed with push and turned to change its kind (a `+` grabbed and turned adds one), the footer values (note, velocity, gate) are grabbed like any, and the cursor follows the focus | `ui().focus`, `ui().comp_cursor`, the step's components and velocity | ✓ |
| F-N16 | NAME: SELECT held and a tap on one of the player's own places of FILE that has a project opens NAME (a factory place or an empty one says why and does nothing); the 4D turns the letter under the cursor through the small letters, the capitals, the digits and a few marks (round, SHIFT ten at a time), the arrows and the twelve pads of the top three rows move the cursor, the last four pads clear, delete, insert and save; push saves the name next to the file and goes back to FILE, which shows it; an empty name takes the name away; FILE cancels; the name starts from the one it has | `ui().main`, `ui().naming`, the `.name` file, `ui().files` | ✓ |
| F-N17 | The same for the presets of BROWSER; the right screen shows the letters round the cursor and turns with the encoder, the pad LEDs show the twelve places (the cursor brighter) and the four actions | `ui().naming`, `ui().browse`, the screen and `leds()` | ✓ |
| F-K17 | On a drum kit KEYBOARD is PAD MODE: eight sounds on the pads (the kick on the first), no octave, ◀ ▶ move the focus; on another instrument KEYBOARD stays chromatic (16 pads) | `pad_note`, `ui().octave` | ✓ |
| F-A20 | The mix: a track's level scales its sound (half the level, half the sound; 0 is silence), the pan places it between the speakers (hard left leaves the right speaker silent, hard right the left one, the centre is the same in both) | unit test in `op8-engine` on the audio of a kick | ✓ |
| F-K18 | While VOLUME is held the eight knobs are the levels of the eight tracks (knobs 5–8 those of E–H) and with SHIFT their pans, clamped to 0..127; the other knobs and the master volume are untouched, the knobs go back to their work when VOLUME is let go, and the mix is part of the project file (an older file opens with the mix at its defaults) | `project().tracks[i].level/pan`, `ui().master`, `op8_seq::io` | ✓ |
| F-D20 | While VOLUME is held both screens show the mix (faders and pans on the left, the four tracks of each knob under each screen); SHIFT shows the pans; it is gone when VOLUME is let go | pixel hashes and the pictures | ✓ |
| F-F08 | What the unit saves is named: a project by the day and the tempo, a preset by the day, in a text file next to it that the player can change; a place with no name is `user N`; a name the player wrote is the one shown and is kept when saving again | `ui().files`, the `.name` files | ✓ |
| F-F09 | ERASE held and a pad of FILE erases one of the player's projects on the second touch of the same place (the first marks it and says so); any other touch lets it be; the projects that come with it and the empty places are not touched; the file and its name are gone | `ui().confirm`, `ui().file_note`, the files, the picture | ✓ |
| F-K19 | The same in BROWSER for the presets of an instrument: named by the day, erased on the second touch, those that come with the instrument stay | `ui().browse`, `ui().confirm`, the files | ✓ |
| F-K20 | TAP in SAMPLING turns the count-in on and off (its LED follows): REC then counts three beats of the project's tempo, with a click on each and the number (3, 2, 1) on the right screen, and records after the third; REC during the count, or leaving SAMPLING, cancels it and nothing records; with it off REC starts at once; the pads and the `source` knob wait while it counts | `ui().smp.counting`, `ui().smp.recording`, the audio of the clicks, the screen | ✓ |
| F-K21 | MIXER opens the mixer on both screens and is lit (again goes back); buttons 1 and 2 choose the channels A to D or E to H, whose levels knobs 1 to 4 are (the pans with SHIFT); buttons 5 and 6 choose the compressor's page (`comp`: threshold, ratio, attack, release; `output`: make-up and mix, two values only) whose values knobs 5 to 8 are; the 4D reaches the four levels and the values of the page, grabs one and turns it | `ui().main`, `ui().mixer`, the project's levels and `comp`, `nav_items` | ✓ |
| F-K22 | Button 7 of MIXER is the EQ page: the screen draws the curve of the three bands, knobs 5 to 8 are low, mid, freq and high (all four used), the 4D reaches the four levels and the four values, button 5 goes back to the compressor, and the four settings are saved with the project | `ui().mixer`, the project's `eq`, `nav_items`, the screen | ✓ |
| F-K23 | Buttons 3 and 4 of MIXER are the sends, `delay` and `reverb`: lit, the knobs under the left screen are how much each of the four channels sends, the faders and the values show it (the level stays), SHIFT is still the pan, the same button again goes back to the levels, and it follows the group (E to H); button 8 is the page `sends` (time, feedback, size, tone: four values, the 4D reaches them); the amounts and the settings are saved with the project | `ui().mixer`, the project's `send_a`, `send_b` and `sends`, `nav_items`, the screens | ✓ |
| F-A21 | The granular makes a cloud only while a note is held and fades after: nothing without a note; `position` chooses where in the sound the grains are cut; `density` is regular on one side of the middle, random on the other and only the note's own grain in the middle, with more grains the further from the middle; `size` is how long that grain lasts; held notes and `tune` set the pitch (an octave is twice the crossings); `spread` scatters between the speakers (none: both the same); the same notes make the same cloud, and six notes at the busiest stay finite and bounded; a sound put in a place while it plays is the one heard; an empty place is silent; the drawing shows the grains | unit tests in `granular.rs` on the audio of a tone | ✓ |
| F-I06 | The granular can be chosen in PLUG-IN, plays a cloud from a step, has two pages (`sample, position, size, density` and `tune, texture, spread, level`), has eight presets in BROWSER, and what SAMPLING saves is heard in a granular that is already playing | `instrument(track)`, the track's audio and note count, `ui().has_second`, `ui().browse` | ✓ |
| F-I07 | The standard tracks hold the eight instruments in the order of PLUG-IN, drums first and the MIDI out last (A drums, B KickSyn, C FmBass, D resonator, E sampler, F granular, G macro, H MIDI out): the unit starts so, and so does every project that comes with it and a new empty one | `instrument(track)` on the unit, on each factory project and on a new empty project | ✓ |
| F-A22 | The sampler's `loop` knob has four positions: one shot, loop, slices 8 and slices 16; with slices the window is cut in equal parts and a note plays one at its own pitch (C3 the first, going round after the last, the length of what sounds is a slice); `tune` goes ± 24 semitones; a state saved before (tune ± 12, loop off or on) still sounds the same and is saved as the new version; the picture shows a tick between the slices | unit tests in `sampler.rs` on the audio; `ui` on the bench | ✓ |
| F-A23 | SHIFT + REC in SAMPLING arms the recording: it waits for a sound louder than −36 dB, keeps nothing while it waits, starts ten milliseconds before the sound, REC (or leaving SAMPLING) disarms it, the pads and the source knob wait, and the screen says it is waiting | `status.armed`, `ui().smp`, the take, pixel hashes | ✓ |
| F-A24 | WAV files copied into the samples folder while OP-8 runs are put in the instruments that play when SAMPLING or PLUG-IN opens: only the new or changed ones, with a line that says how many, a place whose file is gone becomes empty, nothing is loaded when nothing changed, and what SAMPLING saved is not loaded again | `ui().smp.note`, `ui().plug_note`, the audio | ✓ |
| F-A25 | The ends of the part (knobs 6 and 7) move while it plays: the end cuts the sound short, the start jumps ahead; nothing happens if it is not playing | the audio after the cut moved | ✓ |
| F-A26 | The sampler's `loop` knob has a fifth position, `hits`: a sound is searched for where it hits (a jump of 9 dB or more in loudness over what was just before, 60 ms apart at least, up to sixteen, the first sample that is loud less a millisecond); a note plays from one hit to the next (C3 the first, going round after the last), keeping its pitch; what is before the first hit is not a slice, a window with no hits is one slice, a steady tone and a murmur have none; the picture shows a tick at each hit; a state saved with four positions means the same position as before | unit tests in `sampler.rs` (four noise bursts: found within 10 ms, the edges, the voices that play); the bench picture | ✓ |
| F-A27 | The compressor on the master: stereo-linked, soft knee, attack and release on the gain in dB, make-up of 0 to +24 dB and a dry/wet mix; the defaults (threshold 0 dB, no make-up) leave the sound untouched, to the sample; a loud sound is brought down by the ratio and a quiet one is left; make-up adds what it says and mix 0 is the sound as it was; a slow attack lets the start through and the release lets go; the engine reports the reduction and the input level for the screen; the six settings are saved with the project (format version 4, and a version 3 file opens with the defaults) | unit tests in `comp.rs` on the audio; `op8-seq` `io` round trip; the bench: the demo is quieter with the compressor set and the screen changes | ✓ |
| F-A28 | The EQ on the master: a low shelf at 100 Hz, a peak of 200 Hz to 5 kHz and a high shelf at 8 kHz, each of −15 to +15 dB (128 is none); flat, the sound passes untouched to the sample and no tail of the filters is left when it goes back to flat; each band changes its own part of the sound by what it says (measured on tones); the curve the screen draws is what the audio does, within 0.8 dB; the four settings are saved with the project (format version 5, and a version 4 file opens flat) | unit tests in `eq.rs` on tones and on the curve; `op8-seq` `io` round trip; the bench: the demo with the bass cut is quieter | ✓ |
| F-A29 | The sends: with nothing sent the effects give back exact silence; the delay repeats in time with the tempo (an eighth is a quarter of a second at 120 bpm, twice that at 60, to a sample), each repeat quieter and alternating sides; the reverb rings, longer in a bigger room and brighter with a brighter tone, and stays stable on a loud sustained tone; a track sent to an effect is heard from it after the track has stopped, and with nothing sent there is silence; the settings mean what the screen says (six note values, feedback to 95 %); everything is saved with the project (format version 6, and a version 5 file opens sending nothing) | unit tests in `sends.rs` on impulses and tones; `op8-seq` `io` round trip; the bench: a kick sent to each effect rings after it stops | ✓ |
| F-C10 | SETTINGS has four pages named by the buttons 1 to 4 over the left screen (`theme`, `audio`, `midi`, `light`; the open one lit, the buttons of the right screen dark); it opens on the theme, where the 4D turns the theme; on the other pages it moves the focus and leaves the theme alone, and audio and midi ask for the devices | `screen_buttons`, `ui().settings`, `requests`, pixel hashes | ✓ |
| F-C11 | The audio page lists the outputs and inputs with `automatic` first, pushing one chooses it, asks for it (`Request::AudioOut`, `AudioIn`) and keeps it in the file; the audio block is chosen the same way (64, 128, 256); what the hardware answers is shown and a page with no devices still works | `settings`, `requests`, the file, `ui().settings` | ✓ |
| F-C12 | The MIDI page lists the ports and chooses one (asked for and kept in the file); `play a note` and `silence all` ask for those and keep nothing | `requests`, the file | ✓ |
| F-C13 | The settings file keeps the devices, the audio block, the backlight step and the pad level beside the theme, writing only what is not the default; "auto" or nothing is automatic, a value that makes no sense is the default; the light that is asked of the surface is the theme's with the steps put on it and never more | `Settings::render` and `parse`, `effective_light` | ✓ |
| F-C14 | The light page has four steps of backlight and three levels of pad LED; pushing one keeps it in the file and the page shows it; it asks nothing of the audio | `settings`, the file, pixel hashes | ✓ |
| F-C15 | The thread that writes to the MIDI port can be told to play a test note (that ends by itself), to silence the port (all sound off and all notes off on the sixteen channels) and to switch to another port, which it silences first; it says which is open | unit test with fake ports | ✓ |
| F-C16 | The `midi` page has `clock out`, on by default: pushed it is off (the label says so, the file keeps `midi_clock=off`, and a unit that was playing sends one stop), pushed again it is on and the clocks go out again (the default is not written) | `ui().settings.midi_clock`, `Bench::sync_log`, the settings file | ✓ |
| F-D18 | BROWSER draws both screens (the sixteen presets on the left, the instrument's name and what the pads do on the right) and changes when something is saved; its button and the pad LEDs follow it | pixel hashes and lit pixels, the LED slots | ✓ |
| F-D16 | SAMPLING draws both screens (the pads on the left, the wave, the level and the four knobs on the right); the right screen moves while nothing records; the pad and button LEDs follow it | pixel hashes and lit pixels, `ui().smp` | ✓ |
| F-I05 | The sampler can be chosen in PLUG-IN, has two pages (`sample`, `start`, `length`, `tune` and `attack`, `release`, `loop`, `level`), plays a sound from a step, and the right screen draws its wave | `instrument(track)`, the track's audio and note count, `ui().has_second` | ✓ |

## F · Files (`tests/files.rs`, `crates/op8-seq`, `crates/op8`)

| ID | Behaviour | How it is verified | Status |
| :-- | :-- | :-- | :-- |
| F-F01 | A project round-trips through bytes (`op8_seq::io`), and garbage, a truncated file or another version is refused | unit tests | ✓ |
| F-F02 | A project file (the project and each track's instrument name and state) round-trips, and a damaged or other-version file is refused | unit test (`projectfile.rs`) | ✓ |
| F-F03 | The eight factory projects load and differ from each other | unit test (`factory.rs`) | ✓ |
| F-F07 | The eight pieces that come with the machine (house, boom bap, dub, polyrhythm, acid, electro, ambient, breaks) open from FILE and play for twelve seconds with sound all the way through (no silent second in the middle), do not clip, are about as loud as each other (within 7 dB) and every track that has steps makes notes | the bench's audio and the note counts (`tests/songs.rs`) | ✓ |
| F-F04 | FILE shows the sixteen slots (the factory projects in the top two rows) on both screens; a pad opens the project, with its instruments, and goes back to composing; FILE again leaves | the project in the engine after the tap | ✓ |
| F-F05 | SHIFT + a pad of the player's slots saves into it (a file in the projects folder), and a tap opens it again with the tempo and the instruments' knobs | the files and the project after reopening | ✓ |
| F-F06 | An empty slot opens a new empty project (the standard instruments, nothing in the steps); SHIFT on a factory slot does not overwrite it; a damaged file is refused and the project stays as it was | the project is unchanged | ✓ |
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
