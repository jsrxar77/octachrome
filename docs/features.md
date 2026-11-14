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
| F-T04 | Tracks of different length drift apart (polymeter) and the playhead never exceeds its length | track B (length 7): playhead always in 0..=6 and reaches 6 | ✓ |

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
| F-S08 | A muted track emits no notes | track C (MIDI) muted: no new notes after the mute | ✓ |

## B · Brain (`tests/brain.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-B01 | CHORDS switches to Brain mode; the left screen changes | `ui.mode == Brain`; the left screen hash differs | ✓ |
| F-B02 | Holding a pad in Brain plays a live chord and releases it on lift | `brain.live` is `Some` during the press and `None` after | ✓ |
| F-B03 | SHIFT + group connects or disconnects the track from the Brain | `brain_link` toggles | ✓ |
| F-B04 | Knob 1 in Brain changes the key, wrapping from 0 to 11 | `brain.key` goes up a semitone per detent | ✓ |

## P · Punch-in FX (`tests/punch.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-P01 | PERFORM switches to Punch mode | `ui.mode == Punch` | ✓ |
| F-P02 | A pad in Punch engages the effect while held and turns it off on release | `ui.punch` is `Some` during the press and `None` after | ✓ |
| F-P03 | There is always a way back to the sequencer: STEP leaves Punch and Brain, and pressing PERFORM or CHORDS again while in that mode goes back to STEP | `ui.mode` after the presses | ✓ |

## K · Knobs (`tests/knobs.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-K01 | Knob 5 changes parameter 1 of the selected track's instrument | `handle.param(track, 1)` changes with the sign of the turn | ✓ |
| F-K02 | With page 6 open, knobs 5–8 change parameters 5–8 | `right_page == 1`; `param(track, 4)` changes | ✓ |
| F-K03 | Touching a knob shows the large overlay on the right screen | `knob_touch[i]` true; the screen hash changes | ✓ |
| F-K04 | Each instrument exposes at most 8 parameters | `parameters().len() <= 8` for all cartridges (`builtin_names`) | ✓ |

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
| F-D07 | On the real MK3, the two animated screens look complete and in step | **manual review** with photo and video; `OP8_DISPLAY_STATS=1` gives the figures | R |

## L · LEDs (`tests/leds.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-L01 | PLAY lights green when playing and dim when stopped | palette `Green`, level 3 vs 1 | ✓ |
| F-L02 | The step pads show the track colour: active bright, inactive dim | levels 3 and 1 with the colour of `track_led_colors` (glossy) | ✓ |

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

## G · Scripts (`tests/script.rs`)

| ID | What must happen | How it is verified | Status |
|---|---|---|---|
| F-G01 | The demo script parses and ends at 6.8 s | `end_ms() == 6800` | ✓ |
| F-G02 | A script error gives the line number | message with `line N` for each malformed case | ✓ |
| F-G03 | The `shot` captures are generated with their name | PNG files created in `OP8_BENCH_DIR` | ✓ |

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
| F-C09 | The theme changes the pad and group LEDs, not only the screens: track A's pad and group key take that theme's colour from `track_led_colors`, and the three themes do not light the pads alike | `crates/op8/tests/settings.rs` | ✓ |

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
