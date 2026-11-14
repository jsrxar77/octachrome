# OP-8 — What Each Button Does

Part of the [UX/UI Treatise](design.md). The rules that rule here: each button means one idea, similar to its OP-XY equivalent and to what its silkscreen says; Shift gives the secondary function, never a different idea (rule 12). Above each screen you see what the buttons above it do, and below it what the knobs do (rule 11).

The buttons and the sequencer always do the same and do not grow much further. What grows is the instruments ([instruments.md](instruments.md)).

For each MK3 button: its function in the Maschine software (per the [NI manual](https://docs.native-instruments.com/online-guides/maschine-mk3-manual/en/quick-reference)), its equivalent idea on the OP-XY (per the [TE guide](https://teenage.engineering/guides/chrome/layout)), and what it does in OP-8. **Status:** ✓ done, ◐ partial, ○ to do (see [roadmap.md](roadmap.md)).

## 1. Main modes (the equivalents of the OP-XY's instrument, auxiliary, arrange and mix)

The left screen always shows time and the right screen always shows sound. A main mode changes which modules the screen buttons 1–8 offer, like M1–M4 on the OP-XY.

| MK3 button | In Maschine | On the OP-XY | In OP-8 | Status |
| :-- | :-- | :-- | :-- | :-- |
| CHANNEL | Channel properties | instrument | **Instrument**: the default mode, for composing. Sequencer on the left, the track's cartridge on the right. CHANNEL goes back here from settings | ✓ |
| PLUG-IN | Channel plug-ins | instrument (engine) | **Instruments**: a main mode that takes both screens. The 16 pads are the 16 places (the top two rows are the factory instruments from the most guttural to the most refined, the bottom two are for installed ones); a tap puts that instrument on the selected track and goes back to composing. The MIDI out moves from the track that has it. PLUG-IN again leaves ([instruments.md §7](instruments.md)) | ✓ |
| MACRO | Macros | auxiliary | **Auxiliary**: Brain, effect sends and inputs/outputs | ○ |
| ARRANGER | Ideas and song | arrange | **Arrange**: scenes and the song chain | ○ |
| MIXER | Mix | mix | **Mix**: level and pan of the 8 tracks, master EQ and compressor | ○ |
| BROWSER | Browser | (presets) | Presets of the track's cartridge | ○ |
| SAMPLING | Sample editor | sample | Record and trim a sample for a sampler cartridge | ○ |
| FILE · Shift: save | Files | projects | **Projects**: a main mode that takes both screens. The 16 pads are the 16 projects (the top two rows are the factory eight, the bottom two are yours); a tap opens one and goes back to composing, SHIFT + a tap on one of yours saves into it. FILE again leaves. Inside, only the pads, the transport and the pad-mode buttons work | ✓ |
| SETTINGS | Preferences | com | **Settings**: a main mode that takes both screens (see [design.md §4](design.md)). Today it has one setting, the theme; audio, MIDI and brightness come next. Pressing SETTINGS again, CHANNEL, STEP, CHORDS or PERFORM goes back to composing. Inside, the pads, knobs and groups do nothing; PLAY, STOP and RESTART still work | ◐ |
| AUTO · Shift: lock | Record modulation | record + encoder | Hold AUTO and turn a knob to record automation on the track; with Shift it stays fixed | ○ |
| PAGE ◀ ▶ | Parameter page | M1–M4 | Previous and next page of the current mode (the same as the screen buttons) | ○ |

## 2. Screen buttons 1–8 (above the screens)

They are the modules of the current mode, like M1–M4 on the OP-XY. Their name is shown in the button strip along the top of each screen, only for the buttons that do something (rule 11, [design.md §4](design.md)). Their LED follows the same table: dark if the button does nothing, dim if it does something, full if its page is open. The same rule holds for every button ([design.md §2.6](design.md)): dark, a very faint white, or a strong white when pressed or in focus. In Instrument mode:

| Button | Module | On the OP-XY | Status |
| :-- | :-- | :-- | :-- |
| 1 | step: step grid and polymeter | seq | ✓ (back to the step view; named in the strip) |
| 2 | components: step inspector | step components | ◐ (opens the inspector on step 1; holding a pad still opens it on that step) |
| 3 | players: arpeggio and chords over the track | players | ✓ (knobs 1–4: `play` off, chord, up, down, up-down, random; `chord` major, minor, 7th, m7, sus2, sus4, power; `rate` (notes per step of an arpeggio) or `strum` (a chord); `octaves` 1–3. Choosing `chord` starts all together, choosing an arpeggio starts at four notes a step. Not on a drum kit; the same button again goes back to the steps) |
| 4 | bar: length, division and shift of the track | bar | ✓ (knobs 1–4: `length`, `division`, `shift` (turns the steps round the bar) and `rec rounds` (nearest, down, up: which step REC writes to). The same button again goes back. The step view's own knobs 1–2 stay as they were) |
| 5 | shape: page 2 of the cartridge (envelopes) | M2 | ✓ |
| 6 | sound: page 1 of the cartridge | M1 | ✓ |
| 7 | fx: the track's effect | M3 | ✓ (knob 5 `type`: off, delay, reverb, chorus, filter, drive; knobs 6–8 are that effect's three knobs. One effect per track; it is saved with the project) |
| 8 | mod: the track's LFO | M4 | ✓ (knobs 5–8: `shape` sine, triangle, square, saw, random; `rate` 0.05–20 Hz; `depth`; `moves` nothing, the track's volume, or one of the instrument's eight knobs. One LFO per track; saved with the project) |

## 3. Pads and their modes

| MK3 button | In Maschine | On the OP-XY | In OP-8 | Status |
| :-- | :-- | :-- | :-- | :-- |
| STEP | Step sequencer | seq (steps) | Pads = the 16 steps of the track. Holding a pad opens its components | ✓ |
| KEYBOARD | Chromatic pads | keyboard | Pads = a chromatic keyboard from C3 that plays the track's instrument while a pad is held. 4D encoder ◀ ▶ (or knob 1) = octave (the − and + of the OP-XY). Pressing KEYBOARD again goes back to STEP. Not recorded yet (REC) | ✓ |
| CHORDS | Chord mode | players (chords) / brain | **Brain**: chord progression. Pads = degrees; holding one = live chord, and the chord **sounds on the selected track's instrument** for as long as the pad is held (a drum kit has no chords). Pressing CHORDS again goes back to STEP | ✓ |
| PAD MODE | Pads as sounds | (tracks) | The pads play the track's instrument by hand. On a drum kit pads 1–8 are the eight sounds and pads 9–16 are free; on any other instrument the pads climb the Brain's scale from its key. Pressing PAD MODE again goes back to STEP. Not recorded yet (REC) | ✓ |
| PERFORM · Shift: choose | Perform FX on the Smart Strip | punch-in fx | **Punch-in FX**: each pad an effect, pressure is the intensity. Pressing PERFORM again goes back to STEP | ✓ |
| NOTE REPEAT · Shift: arp | Repeat and arpeggiator | players (arp) | Repeat the held note; with Shift, arpeggiator | ○ (today it opens Punch) |
| FIXED VEL | Fixed velocity | — | Toggles a fixed velocity (100): the pads in PAD MODE, KEYBOARD and CHORDS play at it and REC writes it, whatever the strike | ✓ |

## 4. Editing

| MK3 button | In Maschine | On the OP-XY | In OP-8 | Status |
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

| MK3 button | In Maschine | On the OP-XY | In OP-8 | Status |
| :-- | :-- | :-- | :-- | :-- |
| PLAY | Play | play | Play / pause | ✓ |
| STOP | Stop | stop (twice: silence) | Stop; twice silences all notes | ◐ (the second press is missing) |
| REC · Shift: count-in | Record | record | Arms the recording: while the sequencer runs, what you play in PAD MODE or KEYBOARD is written into the track at the nearest step (a drum kit: into the lane of the sound). Chords, automation and the count-in are not recorded yet | ◐ |
| RESTART · Shift: loop | From the start | play (2nd time) | Go back to step 1 and keep playing | ✓ |
| TAP · Shift: metro | Tap tempo | tempo / metronome | Tap tempo: the average gap of the last taps (the first tap after a pause of two seconds only starts counting). **SHIFT + TAP** turns the metronome on and off: a click on every beat while the sequencer runs, higher on the first of each bar | ✓ |
| FOLLOW · Shift: grid | Follow playback | bar | Recording quantisation | ○ |
| TEMPO + 4D encoder | Tempo | tempo | While TEMPO is held, the 4D encoder turns the tempo (Shift = fine) and the left screen shows it big. Without a button held the encoder still turns the tempo | ✓ |
| SWING + 4D encoder | Swing | tempo (swing) | While SWING is held, the 4D encoder turns the global swing and the left screen shows it big | ✓ |
| VOLUME + 4D encoder | Volume | volume | While VOLUME is held, the 4D encoder turns the master volume and the left screen shows it big. (A track's volume with a group held is still to do) | ◐ |
| SHIFT | Secondary functions | shift | Secondary functions (rule 12) | ✓ |

## 6. Other controls

| Control | In OP-8 | Status |
| :-- | :-- | :-- |
| Knobs 1–4 | What the left footer shows (sequencer or the open module) | ✓ |
| Knobs 5–8 | What the right footer shows (cartridge) | ✓ |
| Touching a knob | Large overlay with its parameter | ✓ |
| 4D encoder: turn | Value of the selected element; if none, tempo. In settings: the theme | ◐ |
| 4D encoder: ◀ ▶ | In settings: previous and next theme | ✓ |
| 4D encoder: ▲ ▼ and push | Move through lists and in the inspector; push confirms | ○ |
| Smart Strip | Per PITCH, MOD or PERFORM: pitch bend, modulation or punch-in intensity. Today: master filter | ◐ |
| PITCH / MOD / NOTES | Smart Strip mode | ○ |
