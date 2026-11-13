# OP-3 — What Each Button Does

Part of the [UX/UI Treatise](design.md). The rules that rule here: each button means one idea, similar to its OP-XY equivalent and to what its silkscreen says; Shift gives the secondary function, never a different idea (rule 12). Above each screen you see what the buttons above it do, and below it what the knobs do (rule 11).

The buttons and the sequencer always do the same and do not grow much further. What grows is the instruments ([instruments.md](instruments.md)).

For each MK3 button: its function in the Maschine software (per the [NI manual](https://docs.native-instruments.com/online-guides/maschine-mk3-manual/en/quick-reference)), its equivalent idea on the OP-XY (per the [TE guide](https://teenage.engineering/guides/op-xy/layout)), and what it does in OP-3. **Status:** ✓ done, ◐ partial, ○ to do (see [roadmap.md](roadmap.md)).

## 1. Main modes (the equivalents of the OP-XY's instrument, auxiliary, arrange and mix)

The left screen always shows time and the right screen always shows sound. A main mode changes which modules the screen buttons 1–8 offer, like M1–M4 on the OP-XY.

| MK3 button | In Maschine | On the OP-XY | In OP-3 | Status |
| :-- | :-- | :-- | :-- | :-- |
| CHANNEL | Channel properties | instrument | **Instrument**: the default mode, for composing. Sequencer on the left, the track's cartridge on the right. CHANNEL goes back here from settings | ✓ |
| PLUG-IN | Channel plug-ins | instrument (engine) | Choose the track's cartridge (cartridge list on the right) | ○ |
| MACRO | Macros | auxiliary | **Auxiliary**: Brain, effect sends and inputs/outputs | ○ |
| ARRANGER | Ideas and song | arrange | **Arrange**: scenes and the song chain | ○ |
| MIXER | Mix | mix | **Mix**: level and pan of the 8 tracks, master EQ and compressor | ○ |
| BROWSER | Browser | (presets) | Presets of the track's cartridge | ○ |
| SAMPLING | Sample editor | sample | Record and trim a sample for a sampler cartridge | ○ |
| FILE · Shift: save | Files | projects | Projects: open, new, copy. Shift + FILE saves | ○ |
| SETTINGS | Preferences | com | **Settings**: a main mode that takes both screens (see [design.md §4](design.md)). Today it has one setting, the theme; audio, MIDI and brightness come next. Pressing SETTINGS again, CHANNEL, STEP, CHORDS or PERFORM goes back to composing. Inside, the pads, knobs and groups do nothing; PLAY, STOP and RESTART still work | ◐ |
| AUTO · Shift: lock | Record modulation | record + encoder | Hold AUTO and turn a knob to record automation on the track; with Shift it stays fixed | ○ |
| PAGE ◀ ▶ | Parameter page | M1–M4 | Previous and next page of the current mode (the same as the screen buttons) | ○ |

## 2. Screen buttons 1–8 (above the screens)

They are the modules of the current mode, like M1–M4 on the OP-XY. Their name is shown in the top strip (rule 11). In Instrument mode:

| Button | Module | On the OP-XY | Status |
| :-- | :-- | :-- | :-- |
| 1 | step: step grid and polymeter | seq | ◐ (view done, button unassigned) |
| 2 | components: step inspector | step components | ◐ (today opened by holding a pad) |
| 3 | players: arpeggio and chords over the track | players | ○ |
| 4 | bar: length, division and quantisation | bar | ◐ (today on knobs 1–2) |
| 5 | sound: page 1 of the cartridge | M1 | ✓ |
| 6 | shape: page 2 of the cartridge (envelopes) | M2 | ✓ |
| 7 | fx: the track's effect | M3 | ○ |
| 8 | mod: the track's LFO | M4 | ○ |

## 3. Pads and their modes

| MK3 button | In Maschine | On the OP-XY | In OP-3 | Status |
| :-- | :-- | :-- | :-- | :-- |
| STEP | Step sequencer | seq (steps) | Pads = the 16 steps of the track. Holding a pad opens its components | ✓ |
| KEYBOARD | Chromatic pads | keyboard | Pads = the track's chromatic keyboard. 4D encoder ◀ ▶ = octave (the − and + of the OP-XY) | ○ |
| CHORDS | Chord mode | players (chords) / brain | **Brain**: chord progression. Pads = degrees; holding one = live chord | ✓ |
| PAD MODE | Pads as sounds | (tracks) | Pads 1–8 trigger the 8 tracks; 9–16 are left for a drum cartridge | ○ |
| PERFORM · Shift: choose | Perform FX on the Smart Strip | punch-in fx | **Punch-in FX**: each pad an effect, pressure is the intensity | ✓ |
| NOTE REPEAT · Shift: arp | Repeat and arpeggiator | players (arp) | Repeat the held note; with Shift, arpeggiator | ○ (today it opens Punch) |
| FIXED VEL | Fixed velocity | — | Fixed velocity when recording or playing | ○ |

## 4. Editing

| MK3 button | In Maschine | On the OP-XY | In OP-3 | Status |
| :-- | :-- | :-- | :-- | :-- |
| Groups A–H | Groups | tracks 1–8 | Choose a track. Shift + group connects or disconnects it from the Brain | ✓ |
| SELECT | Select without triggering | — | Hold SELECT + pad or group to choose without sounding | ○ |
| MUTE | Mute | shift + track | Hold MUTE + group to mute the track | ✓ |
| SOLO | Solo | — | Hold SOLO + group to isolate the track | ○ |
| PATTERN | Patterns | per-track patterns | Pads = 16 patterns of the track | ○ |
| SCENE | Scenes | scenes (arrange) | Pads = 16 scenes | ○ |
| EVENTS | Edit events | step components | Opens the components of the selected step (same as button 2) | ○ |
| VARIATION | Humanise and random | component random | Adds random variation to the track (velocity, chance) | ○ |
| DUPLICATE | Duplicate | copy | Hold + source + destination: copies a step, pattern or track | ○ |
| ERASE · Shift: replace | Erase | — | Hold ERASE + pad to erase the step; + group to empty the track | ◐ (pad yes, group no) |
| LOCK · Shift: ext | Parameter snapshot | — | Save and recall the state of all the knobs | ○ |

## 5. Transport and tempo

| MK3 button | In Maschine | On the OP-XY | In OP-3 | Status |
| :-- | :-- | :-- | :-- | :-- |
| PLAY | Play | play | Play / pause | ✓ |
| STOP | Stop | stop (twice: silence) | Stop; twice silences all notes | ◐ (the second press is missing) |
| REC · Shift: count-in | Record | record | Record notes played on the pads and automation | ○ |
| RESTART · Shift: loop | From the start | play (2nd time) | Go back to step 1 and keep playing | ✓ |
| TAP · Shift: metro | Tap tempo | tempo / metronome | Tap tempo; Shift toggles the metronome | ○ |
| FOLLOW · Shift: grid | Follow playback | bar | Recording quantisation | ○ |
| TEMPO + 4D encoder | Tempo | tempo | Tempo; Shift = fine | ○ |
| SWING + 4D encoder | Swing | tempo (swing) | Global swing | ○ |
| VOLUME + 4D encoder | Volume | volume | Master volume; holding a group = that track's volume | ○ |
| SHIFT | Secondary functions | shift | Secondary functions (rule 12) | ✓ |

## 6. Other controls

| Control | In OP-3 | Status |
| :-- | :-- | :-- |
| Knobs 1–4 | What the left footer shows (sequencer or the open module) | ✓ |
| Knobs 5–8 | What the right footer shows (cartridge) | ✓ |
| Touching a knob | Large overlay with its parameter | ✓ |
| 4D encoder: turn | Value of the selected element; if none, tempo. In settings: the theme | ◐ |
| 4D encoder: ◀ ▶ | In settings: previous and next theme | ✓ |
| 4D encoder: ▲ ▼ and push | Move through lists and in the inspector; push confirms | ○ |
| Smart Strip | Per PITCH, MOD or PERFORM: pitch bend, modulation or punch-in intensity. Today: master filter | ◐ |
| PITCH / MOD / NOTES | Smart Strip mode | ○ |
