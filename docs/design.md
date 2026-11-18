# Octachrome — UX/UI Treatise

This document rules over everything Octachrome shows and how it is operated. If the code or the Twin disagree with this treatise, the code is wrong. If something is needed that the treatise does not cover, it is written here first, approved, and only then programmed.

The direction is the language of Teenage Engineering (OP-1, OP-1 field, OP-XY): colour tied to the physical control, drawings instead of text, absolute black. We take the ideas, never their graphics, icons or typeface. It starts from the "Impeccable" system of Flip Tracker+ (`jsrxar77/flip-tracker`, its `DESIGN.md`), adapted to the Maschine MK3.

The living reference is **Octachrome Twin** (`twin/octachrome-twin.html`). Every new design is tried there before it is ported to `crates/octachrome-ui`.

---

## The 13 rules

1. **Eight encoders, eight colours.** Each encoder has its own fixed colour. That colour paints on screen, and only, what the encoder moves.
2. **1–4 time, 5–8 sound.** Encoders 1 to 4 drive the sequencer and are shown on the left screen. Encoders 5 to 8 drive the instrument of the selected track and are shown on the right screen. They never cross.
3. **Grey is context.** Whatever cannot be moved right now is grey.
4. **Absolute black.** The background is always `#000`.
5. **Drawing before text.** Every view has a drawing that moves with the sound or the value. Text is only the short name of the parameter and its value.
6. **Calm until activated.** What is inactive is a thin line; what is active (a step being edited, the selected track, a touched encoder) lights up.
7. **Touching is asking.** Touching an encoder (capacitive sensor) shows, large and in its colour, what it is and how much it is worth. Releasing it makes it disappear. There are no instructions on screen.
8. **Lowercase and no code.** `decay`, `fmbass`, `126ms`. No brackets, underscores, IDs or `0x`.
9. **One typeface.** Jost, anti-aliased, on the fixed scale of section 3. Never pixelated terminal fonts.
10. **The track is a dot.** The track colour (A–H) identifies the track on the physical pads and LEDs, and on screen only as a dot, ring or pad fill. It never competes with the encoder colours.
11. **Buttons above, knobs below.** The top strip of each screen shows what each button above it does, in a capsule centred on the button; the footer shows what each knob below it does, in the column of the knob. Button labels are white or grey, never in a knob colour. A button with no function shows nothing.
12. **Each button means one idea.** What a MK3 button does in Octachrome resembles what its OP-XY equivalent does and what its silkscreen says. Shift gives the secondary function, never a different idea.
13. **Few parameters.** An instrument shows at most 8 parameters: 4 of sound (button 5, the first page) and 4 of shape (button 6, the second). The track's effect and modulation are the same for every instrument (4 + 4). If an engine has more, the cartridge picks the 8 that matter.

---

## 1. Control map

What each MK3 button does is in its own document: **[buttons.md](buttons.md)**. The instruments, which are the centre of Octachrome, are in **[instruments.md](instruments.md)**.

## 2. Colour

Three themes, three characters, in this order since 2026-11-16: **chrome** (the default of the unit and the twin, and the theme of the pictures of the manual since 2026-11-17; the page of the guide and the manual keeps matte colours) is monochrome: one range (violet drifting to blue, strong to soft) with white last; **matte** is softer and chalky and **glossy** is strong and bright. Every theme has exactly **eight** colours, always in an order that recalls the rainbow, and they are chosen for what the MK3's LEDs can show: 16 hues in 3 levels on the pads and group keys, one colour on every other button. The logic is the same in all three; only the palette changes. The hex values are ours, not measured from the devices.

### 2.1 Encoder colours

| Encoder | matte | glossy | chrome (default) |
| :-- | :-- | :-- | :-- |
| 1 | `#3B8EBA` steel blue | `#0099FF` blue | `#D633FF` violet-magenta |
| 2 | `#D9822B` ochre | `#00E56A` green | `#A442FF` violet |
| 3 | `#A9AFB8` grey | `#F2F4F8` white | `#8652FF` soft violet |
| 4 | `#F2652A` orange | `#FF4F00` orange | `#7361FF` periwinkle |
| 5 | `#7FB58A` sage | `#8B6CFF` violet | `#707AFF` cornflower |
| 6 | `#E3C77A` sand | `#FFD21F` yellow | `#809DFF` sky |
| 7 | `#A88BC7` lilac | `#2ED3E6` cyan | `#8FBCFF` pale blue |
| 8 | `#D9677A` dusty rose | `#FF4FA0` pink | `#EEF0F5` white |

The first four follow the encoder order of each reference device (original OP-1: blue, green, white, orange; OP-1 field: blue, ochre, grey, orange). The four instrument colours are chosen so they are not confused with the first four at a glance.

### 2.2 Base colours

| Token | matte | glossy | chrome | Use |
| :-- | :-- | :-- | :-- | :-- |
| `bg` | `#000000` | `#000000` | `#000000` | Background |
| `fg` | `#F4EFE6` | `#F2F4F8` | `#EEF0F5` | Text and main strokes |
| `dim` | `#8E918F` | `#7E8794` | `#80858F` | Context, not editable now |
| `faint` | `#262A30` | `#232831` | `#24262C` | Guides, passive borders, empty steps |
| `panel` | `#12161D` | `#0D1118` | `#0F1014` | Surface of keys and capsules |
| `rec` | `#E63946` | `#FF3B30` | `#FF2244` | Only REC, erase and peaks |

The track colours (A–H) are part of each theme (`Theme.tracks` in `crates/octachrome-gfx/src/theme.rs`; `track_color(i)` in `octachrome-ui`). The pad and group LEDs take the same colours from the MK3's palette (`track_led_colors` in `crates/octachrome/src/controller.rs`): glossy is the rainbow, matte its softer hues, chrome one range from magenta to blue, then white. The palette order is still to be checked on the unit.

### 2.3 Brand mark

The mark of Octachrome is **eight buttons, one per colour of the rainbow, the eighth white**. Everything in Octachrome is built on eight, and the eighth colour is white because it is the **octave** (*8va*): in music the eighth note is the first one again, higher. The mark is the only picture that stands for the product (guide, twin, icon, README). The themes are named glossy, matte and chrome (they were op-1, op-1 field and op-xy; the old names still open an old settings file).

| Button | matte | glossy | chrome |
| :-- | :-- | :-- | :-- |
| 1 red | `#D9534F` | `#FF2D2D` | `#D633FF` |
| 2 orange | `#D9822B` | `#FF8A1F` | `#A442FF` |
| 3 yellow | `#E3C77A` | `#FFD21F` | `#8652FF` |
| 4 green | `#7FB58A` | `#00E56A` | `#7361FF` |
| 5 blue | `#3B8EBA` | `#0099FF` | `#707AFF` |
| 6 indigo | `#5F6BB5` | `#4B4BFF` | `#809DFF` |
| 7 violet | `#A88BC7` | `#B15CFF` | `#8FBCFF` |
| 8 white | `#F4EFE6` | `#F2F4F8` | `#EEF0F5` |

Rules of the mark:

- Always eight buttons, always in a fixed order that recalls the rainbow. In glossy and matte it is **red, orange, yellow, green, blue, indigo, violet, white**. In chrome it is one range, from the strongest violet drifting to blue, then white. The order never changes, in any size or use.
- The eight are laid out **as the MK3 lays out eight** (§2.4): the eight group keys, **four and four** (the main mark), or the **eight knobs in a row** (the long mark). Reading order is left to right, then the second row. Those two are the mark. The **infinity logo** is also allowed for the logo and the wordmark, with the same eight colours in the same order: an 8 on its side, eight keys along it, for Octachrome read as OP-∞ (`guide/brand.html`). A picture of the unit itself never uses it (§2.4).
- Always on absolute black. Never fewer than eight, never another colour, never outlined, never a gradient.
- The colours come from the active theme (`Theme.brand` in `crates/octachrome-gfx/src/theme.rs`). A theme may shift the range (softer in matte), but it always has **eight**: in glossy and matte the seven colours of the rainbow in their order, in chrome seven steps of one range, and in all of them the theme's own white last. A test checks that each button is the colour of its place (F-D09).
- The keys keep the proportion of the unit's group keys and rounded corners; the knobs are circles. A single button may light in turn, like a playhead; nothing else moves. In the guide the eight can be played as a major scale, the white being the octave.
- The wordmark is `Octachrome` in Jost Light. The mark can stand alone or above the wordmark; the wordmark never carries the colours.
- The infinity logo is drawn from the same colours (`Theme.brand`); it has no master file yet.
- The master files are `guide/brand/octachrome-mark.svg` (4 + 4) and `guide/brand/octachrome-mark-knobs.svg` (a row). Their geometry is `MK3.mark` in `guide/mk3.js`.

### 2.4 Pictures of the unit

Every picture of the MK3 (the guide's map, the manual, the brand page, the twin, any future image) **follows the real front panel**, so the picture can be used to find the control in your hands.

- The master geometry is `guide/mk3.js`. **Where the positions come from:** they were measured (2026-11-18) on a top-down photo of Javi's own MK3, in photo pixels with the unit's top-left corner at (74, 299) and its size 838 × 793, then scaled into a 1000-wide box; the proportions of the unit are 1.06 : 1. The left block is, from the top, CHANNEL/PLUG-IN (small), ARRANGER/MIXER and BROWSER/SAMPLING (large), PAGE ◀ ▶, FILE/SETTINGS and AUTO/MACRO (small). A Maschine+ has the same keys but one wide screen and other proportions: it is not a reference. Drawings take their positions from this file; nobody places a button by eye.
- **The manual's map and the twin show the same unit.** The twin (`twin/octachrome-twin.html`) places every part in a box with the unit's proportions from `UNIT_PANEL`, a copy of `guide/mk3.js` that `tests/twin_panel.rs` keeps the same (F-F21; `OCTACHROME_WRITE_TWIN=1 cargo test -p octachrome --test twin_panel` rewrites it). The keys of the unit that Octachrome does not use yet (EVENTS, VARIATION, DUPLICATE, SOLO, AUTO, MACRO, PAGE ◀ ▶, NOTE REPEAT, LOCK, PITCH, MOD, NOTES, FOLLOW) are there too, dark and without a function, so the panel is the unit's. On a narrow screen the twin does not squeeze the unit: it keeps a readable width and scrolls sideways.
- The manual opens with a map of the unit drawn from it (`guide/unit-map.js`): every part is numbered and named, so the rest of the manual can say "button 5", "knob 7", "pad 12" or "group G". The manual, its pictures, the guide and the twin use the matte theme, the default; the other two are one press of the 4D encoder away in settings.
- Top to bottom: the eight screen buttons, the two screens, the eight knobs under them. On the left, six rows of two buttons (CHANNEL to MACRO), the 4D encoder with VOLUME, SWING and TEMPO beside it, then PITCH, MOD, PERFORM, NOTES, the Smart Strip, the group keys A–H as two rows of four, and the transport at the bottom. In the middle, the pad-mode buttons above the pads and SCENE to MUTE in a column to their left. On the right, the 4×4 pads.
- A group of **eight** is drawn as the eight knobs in a row or as the eight group keys, four and four. A group of **sixteen** is the 4×4 pads. Never another arrangement.
- The same rule applies to what the player sees in Octachrome: the 8 factory and 8 user instruments, and the 8 factory and 8 user projects, are the 16 pads ([instruments.md §7](instruments.md)).

### 2.5 Light

A theme is also how much **light** the machine gives, not only its colours. The theme asks for a share of what the supply allows ([architecture.md §10](architecture.md)), and never for more:

| | Screens' backlight | Pad and group LEDs | Other buttons and strip |
| :-- | :-- | :-- | :-- |
| glossy | all that the supply allows | level 3 | full |
| matte | 60 % | level 2 | 70 % |
| chrome | 80 % (to be tuned on the unit) | level 3 | 85 % |

The values are `Theme.light` in `crates/octachrome-gfx/src/theme.rs`; the supply's ceiling is the power plan in `crates/octachrome-hw/src/mk3/power.rs` (F-H07). `guide/themes.html` shows the three on the whole unit. The monochrome theme is still open: its colours and its light are provisional until it is seen on the MK3.

### 2.6 Light of the buttons

Every button follows one rule, the same for all of them (`crates/octachrome/src/lights.rs`, one table):

- **Dark:** it does nothing in the current context (REC, TAP… until they are built; MUTE and ERASE in settings).
- **Very faint white:** it does something here. It can be pressed, but it is not in use.
- **Strong white:** it is pressed or held, or it has the focus: the pad mode in use, CHANNEL while composing and SETTINGS in settings, the page that is open, SHIFT, MUTE or ERASE while held, STOP while stopped and PLAY while playing.

**The rule, from Javi (2026-11-17): a button is lit if and only if it has a function in the current context.** Never leave a working button dark, and never light one that does nothing. A toggle (a pad mode, PATTERN, SCENE, ARRANGER, MIXER…) goes to strong white when it is pressed and back down to faint white when it is pressed again, as KEYBOARD and CHORDS do. The 4D's four sides and SELECT follow it too: up, down, left and right are lit wherever the 4D moves something (the tracks, the buttons and the values while composing, the cells of a view, the theme in settings, the octave on the keyboard, the cursor of a name), SELECT in FILE, BROWSER and SAMPLING, where it names things. The push of the 4D has no LED on the MK3, so it is never lit. Every new function therefore comes with its light: the button goes in `lights::all()` and in `lights::light`, and `F-L09` checks it (rest, pressed, pressed again). The twin lights the same way.

The buttons whose LED has a colour (the groups A–H, PLAY) keep it and follow the same three levels. The faint white is about 10 % of the full brightness, which also keeps the current down ([architecture.md §10](architecture.md)). The eight screen buttons use the same rule through `screen_buttons` (§4). In the twin a faint button has a faint label and a strong one is filled.

## 3. Typography

The screens are a real 480×272, so the type has to be readable at 11–15 px. It is rendered with anti-aliasing (TrueType).

**Typeface: Jost** (OFL), chosen by Javi on 2026-11-12 and tuned to resemble the OP-XY screen: light weights, small labels with some air between letters, and large numbers in the thinnest weight. One family for everything. Teenage Engineering does not publish which typeface the OP-XY uses, so the tuning is by eye against what is seen on the device, not a copy.

| Level | px | Weight | Spacing | Use |
| :-- | :-- | :-- | :-- | :-- |
| `s` | 11 | 400 | +0.6 px | Parameter name under the dial |
| `lab` | 15 | 400 | +0.5 px | Name of a screen button, in the strip (14 px for the name under a knob) |
| `m` | 15 | 400 | +0.3 px | Value under the dial when a long value does not fit at `l` |
| `l` | 20 | 400 | +0.2 px | Inspector values; the value in the footer under each knob |
| `mid` | 21 | 400 | +0.2 px | Instrument, chord, track letter |
| `big` | 34 | 300 | 0 | Step number in the inspector |
| `huge` | 62 | 300 | −1 px | Value when an encoder is touched |

**Centring, always.** Text inside a container (a capsule, a pill, a key, a card) is centred as the eye does, never by its font metrics: the *ink* between the first and the last glyph is centred horizontally (side bearings and trailing letter spacing do not count), and the *x-height* is centred vertically, so the baselines of different labels line up. In Rust that is `text_centered` in `crates/octachrome-gfx/src/font.rs`; use it, not `text_v` with `Align::Center`, whenever text sits in a container. A test measures it (F-D13).

**FlipMono is not used:** the `FlipMono.ttf` file of Flip Tracker+ is Apple's SF Mono, whose licence only allows it on Apple devices. It cannot be distributed on a Raspberry Pi.

In Rust, Jost (300 and 400, `assets/fonts/`) is embedded with `include_bytes!` and rasterised with `fontdue` (MIT/Apache); each glyph is cached and alpha-blended onto the RGB565 framebuffer (`crates/octachrome-gfx/src/font.rs`).

## 3b. Where each thing lives in the code

| What | File |
| :-- | :-- |
| Themes, encoder colours, brand colours and light | `crates/octachrome-gfx/src/theme.rs` (`theme()`, `knob(i)`, `Theme.brand`) |
| Brand mark (master files) | `guide/brand/octachrome-mark.svg`, `guide/brand/octachrome-mark-knobs.svg` |
| MK3 front-panel geometry (every picture of the unit) | `guide/mk3.js` |
| Brand colours in the twin | `THEMES[...].brand` in `twin/octachrome-twin.html` |
| Typeface and type scale | `crates/octachrome-gfx/src/font.rs` |
| Anti-aliased lines, arcs, circles and boxes | `crates/octachrome-gfx/src/paint.rs` |
| Dial footer and touch overlay | `crates/octachrome-ui/src/theme.rs` |
| Left screen | `crates/octachrome-ui/src/left.rs` |
| Right screen (frame) | `crates/octachrome-ui/src/right.rs` |
| Drawing of each instrument | `draw_ui` of each cartridge in `crates/octachrome-cartridges/src/` |

The theme is chosen with `octachrome run -t matte|glossy|chrome` or `OCTACHROME_THEME`.

## 4. Screen structure

```
┌────────────────────────────────────────────────┐
│ (step)    (components)                         │  button strip, 24 px: capsules
│ ● name                           ● ● (pages)   │  top bar: items centred at y = 37
│ +                                            + │
│               animated drawing                 │  drawing area
│ +                                            + │
│ ◔ value    ◔ value    ◔ value    ◔ value       │  footer, 4 × 120 px, each column
│   name       name       name       name        │  above its physical encoder
└────────────────────────────────────────────────┘
```

- **Button strip (24 px, y 0..24):** the name of what each of the four buttons above the screen does, `lab` (15 px), inside a capsule (22 px high, 14 px of padding each side, drawn on whole pixels as a 1 px band between two filled shapes so the outline is the same width all the way round, the text centred by the height of its lowercase letters, `panel` fill) **centred on the physical button**. The buttons sit about 140 px apart, centred at x = 24, 164, 304 and 444 on each screen (`BUTTON_CX` in `crates/octachrome-ui/src/theme.rs`, measured on the unit; a capsule never leaves the screen). The open page has an `fg` outline and `fg` text, the others a `faint` outline and `dim` text. A button with no function shows nothing, and in settings the strip is empty. The strip sits above the header items, which are centred at y = 37, leaving a few pixels of air between them. Left: `step` (button 1: back to the step view), `components` (2: the inspector), `players` (3) and `bar` (4). Right: `sound` (5, the first page), `shape` (6, only for an instrument with a second page), `fx` (7) and `mod` (8), in that order. **Choosing a track, or an instrument, puts the screens on their first pages: the steps on the left (button 1) and `sound` on the right (button 5), so the two buttons that are lit are always 1 and 5, and what matters most of an instrument lives on its first page.** **A label and its button's LED always agree** (one table, `screen_buttons` in `crates/octachrome-ui/src/lib.rs`): a button with no function has no label and a dark LED; with a function it has a label and a dim LED; the open page has the bright label and the full LED. In settings all eight are dark.
- **Left (fixed): time.** Tempo, the Brain chord, polymeter rings (one per track, the selected one in front, all of them between the header and the footer: centre (118, 137), outer radius 82), a mirror of the 16 pads. Footer with encoders 1–4. On a drum kit track the polymeter rings give way to the eight lanes by sixteen steps (the chosen lane lit, the playhead marked) and the mirror shows the chosen lane; a kit step has no note, so its inspector has no note cell. On the mirror an active step is the track colour at 75–100 % (by velocity) and an empty step is a nearly black cell of the track colour (10 %) with a `faint` outline, so on and off are far apart; on the pads an active step is the track colour at the highest level the theme and supply allow and an empty step is a dim white (level 1), so they stay apart even when the levels are capped.
- **PAD MODE and KEYBOARD (left):** the sixteen pads drawn as they are on the unit (pad 1 bottom-left), each with the note or the drum sound it plays (`Small`), lit while held; on the left, what the pads are (the key and scale, `kit`, or the keyboard's octave `C3`). The footer shows `octave` (KEYBOARD) and `tempo`.
- **TEMPO, SWING or VOLUME held:** the drawing area is cleared and shows the name (`mid`, `dim`) and the value (`huge`, `fg`) centred, for as long as the button is held; the footer stays.
- **REC armed:** a `rec`-colour dot after the tempo in the header of the left screen, and the REC button strong red.
- **Right: the instrument of the selected track.** Each cartridge draws its animated illustration using colours 5–8 for what those encoders move. Footer with encoders 5–8. The page dots at the top right show the pages that exist (sound, shape if the instrument has one, fx, mod) and the open one is bright.

### Pages of buttons 3, 4, 7 and 8

Each one is a picture that explains what its four knobs do (rule 3), in the colours of those encoders, and each is drawn on the same area and footer as the page it replaces. The picture never carries text beyond a single line of title and, where there is nothing set yet, one line saying what to do (rule 11).

- **Players (left, button 3).** Two octaves of piano keys from C3, drawn as keys (white keys `panel`, black keys `bg`, a `faint` outline). The notes of the chord the players make on the note of the track's first step are filled in the track colour; with an arpeggio, one of them is `fg` and walks through the notes in the order the arpeggio plays them. Title: the mode and the chord (`up · minor`), or `off: each step plays its note`, or `a drum kit plays its lanes`. Footer: `play`, `chord`, `rate` (notes per step) or `strum`, `octaves`.
- **Bar (left, button 4).** The sixteen steps of the track as one bar of cells: the ones inside the length are filled (the track colour if the step is on, `panel` if not), the ones beyond it are `bg`; the steps that start a beat are taller (as many as the division puts in a beat); a dot under the bar is the playhead. Title: `n steps of 1/16 = x beats`. Footer: `length` (with its ring of dots), `division`, `shift` (it has no dial: it is turned, not set) and `rec rounds`.
- **Fx (right, button 7).** A moving picture of the effect, in the colour of encoder 5 and `fg`: **delay** a sound and its echoes (their gap is the time, their height the feedback, their brightness the mix, and one lights up in turn); **reverb** rings spreading out from a point (how far is the size, how sharp the tone, how bright the mix); **chorus** a wave and a copy of it drifting apart (how far is the depth, how fast the rate); **filter** the response curve (a low-pass bends down on the right, a high-pass on the left, the peak is the resonance) with a line at the cutoff; **drive** the curve that flattens a wave and a dot that rides it. With no effect: a crossed-out ring and `turn the first knob to choose an effect`. Footer: `type` and the effect's three knobs, named for that effect (`time feedback mix`, `size tone mix`, `rate depth mix`, `sweep resonance mix`, `drive tone mix`).
- **Mod (right, button 8).** The wave of the LFO across the area (two cycles) with a dot running along it at the LFO's own rate; the height of the wave is the depth. Title: `moves volume`, `moves index`, or `moves nothing` (and then one line: `choose what it moves with the last knob`). Footer: `shape`, `rate` (in Hz), `depth`, `moves`.

### Projects view (FILE)

FILE is a main mode like settings: both screens, no footer, no strip. **Left:** the sixteen slots as the pads are (reading order, top-left first), each a rounded cell with the project's name (`m`); the top two rows are the factory projects (`panel` fill) and the bottom two are the player's (`faint` fill, or an outline and `empty` where there is nothing); the slot last touched has a 2 px `fg` outline; the title is `open`, or `save` while SHIFT is held. **Right:** `projects`, what a tap does and what SHIFT + tap does, and below a line saying what just happened. **Names and erasing** (Javi, 2026-11-16): what the unit saves is named by the day and the tempo (`16 nov 122`; a preset by the day), in a small text file next to it (`user-1.name`) that the player can write or change with any editor; a place without one is called `user N`. ERASE held and a tap on one of the player's places **marks it**: the cell turns red (`rec`, with the ink that reads on it, `readable`) and says `erase?`, and the right screen says `erase <name>? touch it again`; a second tap on the same place erases the file and its name; any other tap lets it be. The factory places and the empty ones are never touched, and the right screen says so. The same in BROWSER. **Armed** (SHIFT + REC, 2026-11-16): the title reads `waiting for a sound`, the dot beats at half the speed it has while it records, nothing is kept, the level bar follows the source and has a small `fg` mark at the level that starts the recording (−36 dB, on the same square-root scale as the bar); REC is lit. The pads and the `source` knob wait, as while it records. The line under the wave also says `loaded N sounds from the folder` when SAMPLING opens and the folder had new files. The pad LEDs are white: level 2 for the factory slots, 3 for the player's that hold a project, 1 for the empty ones. The play mark stays on the left.

### Instruments view (PLUG-IN)

Like FILE (above): both screens, no footer, no strip. **Left:** the sixteen places as the pads are, with the same cells; the top two rows are the factory instruments in the order of [instruments.md §7](instruments.md), the instrument of the selected track has the 2 px `fg` outline, one that is not built yet has an outline only, and the bottom two rows are the places for installed instruments (`empty`). **Right:** `plug-in`, what a tap does, and what just happened. The pad LEDs are white: level 3 for the track's instrument, 2 for the others that can be chosen, off for the rest.

### Patterns view (PATTERN)

Decided with Javi on 2026-11-17. Like FILE and PLUG-IN: both screens, no footer, no strip. **Left:** the title `patterns · C` (the selected track) and the sixteen pads as sixteen cells with their numbers, one tone for all (there is nothing that comes with the machine here): a cell with something in it has a `panel` fill, the pattern that is shown (and edited) the 2 px `fg` outline, and the one that is playing, when it is another because the change waits for the start of the track's next loop, a small play mark in `fg` at the right of its cell. **While anything plays, the cell of the pattern that plays has a bar of `fg` along its bottom that grows with the steps of its loop** (decided 2026-11-17: the person must see where it is). **While a song is followed, what is shown follows what plays** (each track shows the pattern it is playing, changing at the start of its own loop, so the steps, this view, the pads and the top bar move by themselves from section to section), and a tap on a pad says `the song decides what plays` and changes nothing: to choose, stop or turn the song off. **Right:** the same title and three lines, `tap a pad` (shows it, plays from the next loop), `shift + tap` (plays it at once) and `hold + tap another` (copies it; erase + tap empties), and the last thing that happened as a note. **Pads:** white, level 3 on the one that is shown, 2 on one with something in it, 1 on an empty one, and the one that is playing, if it is another, in the colour of the track at level 2. A tap acts when the pad is let go, so that a pad can be held while another is tapped. **What a pattern is:** the steps, the length, the division and the kit of a track; the instrument, the effect, the LFO and the mix are the track's. **When it changes:** what is shown changes at once, so it can be edited at once; what plays changes when the track's loop starts again (the pattern then begins from its first step, also if its length or division are not the same), or on the next tick with SHIFT; a stopped sequencer plays what is shown.

### Scenes view (SCENE)

Decided with Javi on 2026-11-17. Like PATTERN: both screens, no footer, no strip. **Left:** the title `scenes` and the sixteen pads as sixteen cells with their numbers: a scene that has been stored has a `panel` fill and the one launched or stored last the 2 px `fg` outline. **Right:** the same title and three lines, `tap a pad` (every track goes to the pattern it had, from its next loop; the mutes at once), `shift + tap` (stores the patterns and mutes that are now) and `erase + tap` (empties it), and the last thing that happened as a note. **Pads:** white, level 3 on the one launched or stored last, 2 on a stored one, 1 on an empty one. **What a scene is:** which pattern each of the eight tracks shows and which tracks are muted; nothing of the sound. **When it changes:** each track changes when its own loop starts again (as a tap in PATTERN does), so tracks of different lengths come in as each one finishes; the mutes change at once. A scene that was never stored says so and changes nothing.

### Song view (ARRANGER)

Decided with Javi on 2026-11-17. Like SCENE, with the four knobs of the right screen as SAMPLING has them. **Left:** the title `song` and the sixteen pads as the sixteen steps of the song: a step that exists is a cell with a `panel` fill and two lines of text, `scene 2` and `4 bars`, with a bar along the bottom that grows through the bars while that step plays, a free one shows its number only, and the step that plays has the 2 px `fg` outline (while the song is followed and the sequencer plays). **Right:** `song`, the step that is chosen (`step 3 of 5`, `scene 2 for 4 bars`, or `step 4 is free`), what a tap does (it chooses the step; while the song plays it goes there at the next bar, `shift + tap` at once, stopped `shift + tap` puts a step in front; hold a pad and tap another to move it; `erase + tap` takes it out), `playing step N · bar X of Y` when no note is shown, and `move step N` under knob 4 (moves the chosen step), the last thing that happened as a note, and the four knobs under the screen in their colours: `scene` (knob 5), `bars` (6), `song` `on` or `off` (7, turn right for on, left for off) and `steps`, the length (not a knob). **Pads:** white, level 3 for the step that plays, 2 for one that exists, 1 for a free one. **What the song is:** up to sixteen steps, each a scene and 1 to 16 bars; when it is followed, the sequencer enters each step as the bars of the one before are over (every track changes at the start of its own next loop; the mutes at once) and after the last one goes back to the first. The song does not change what is shown while composing (the pattern that is shown): it changes what *plays*, as a waiting pattern does; but a track is drawn (the track dots, the rings, the mixer) and lit on its button as muted while it plays when its own mute is on or its scene silences it (MUTE still works; it cannot bring back what the scene silences). **The top bar of composing** (decided 2026-11-17) has, after the tempo, a small `dim` tag: `p2`, the pattern of the selected track that is shown, or `p2>3` when another one plays; and ` s3` after it, the step of the song that plays.

### Sampling view (SAMPLING)

Like FILE: both screens, no footer, no strip. **Left:** `sampling` and the sixteen pads as cells: the top two rows are `user 1` to `user 8`, the places of the player's own sounds (`panel` fill where one already has a sound), the bottom two rows say `listen` (filled once there is a take). **Right:** a dot and `sampling` (the dot pulses and the title reads `recording` while it records) and the length of the take at the right of the bar; the wave across the screen with the part that is kept lit in `fg` on a `panel` field and its two ends marked in the colours of knobs 6 and 7; a dot that goes along the line while there is no take (the screen moves while it waits); a level bar under the wave; one line saying what just happened; and under it the four knobs of the take, name in `dim` and value in the knob's colour: `source`, `start`, `end`, `fade` (knobs 5 to 8). The pad LEDs are white: level 3 for a place that has a sound, 1 for an empty one, level 2 for `listen` once there is a take. REC is lit while it records. **Count-in** (Javi, 2026-11-16): TAP turns it on; the bar then says `count-in` in `dim` beside the length; when REC is pressed the title reads `get ready`, the dot pulses and the beat that is left (3, 2, 1) is drawn in `fg` over the middle of the wave, in the largest size; TAP and REC are lit while it counts. **The cut while it plays:** the two marks and the values of knobs 6 and 7 move as always and what is heard follows them.

### Presets view (BROWSER)

Like FILE and PLUG-IN: both screens, no footer, no strip. **Left:** the sixteen places as the pads are; the top two rows are the presets that come with the instrument (`panel` fill and their names; an instrument with fewer than eight leaves the rest `empty`) and the bottom two are the player's (`user 1` to `user 8`, with a fill where there is one); the one last touched has the 2 px `fg` outline; the title is `presets`, or `save preset` while SHIFT is held. **Right:** `browser` and the instrument's name, what a tap and SHIFT + tap do, and a line saying what just happened. The pad LEDs are white: level 3 for the last one touched, 2 for a preset that exists, 1 for an empty place of the player's.

### Navigation with the 4D encoder

Decided with Javi on 2026-11-15: **every view that has things to choose has a focus, always, and the 4D encoder works on it**, so the unit can be played without reaching for the pads.

- **Items.** A view lists what can be reached as items (a box on a screen), in `crates/octachrome-ui/src/nav.rs`; the same list draws the focus ring and answers the encoder, so what can be reached and what is drawn cannot drift apart. In FILE, PLUG-IN, BROWSER and SAMPLING the items are the sixteen places (reading order, the order of the pads), and SAMPLING also has its four values (`source`, `start`, `end`, `fade`) under the right screen. **Composing** lists, in this order: the screen buttons that have a name on the left (1–4), the eight tracks (the dots of the header), the sixteen pads of the grid that mirrors the pads (when the left screen shows it: not under the players and bar pages), the four values of the left footer, then the screen buttons of the right screen (5–8, `shape` only if the instrument has a second page), **the eight sounds of a drum kit** (drawn by the drums on the right screen, on its `sound` and `shape` pages: pushing one chooses it, as the `sound` knob does, and plays it; other instruments have none) and the four values of the right footer: forty things, forty-eight on a drum kit (Javi, 2026-11-16). The pads are the ones the left screen **draws**, wherever it draws them: the 37 px grid of the steps and of the notes (KEYBOARD, PAD MODE), the sixteen larger effect places of PERFORM (108 × 36 px, `punch_rect`), and none in Brain, which draws no pads. **The step inspector** (Javi, 2026-11-16) lists the same things but for the grid: in its place the components of the step and the `+` that adds one (`Comp`, three to a row where the inspector draws them). The focus opens on the first; on a component the focus *is* the component cursor of knob 4; push grabs it (the frame turns `fg` and thick) and turning changes its kind, as knob 4 does (taking it to `none` removes it); push lets go. The four values of the left footer (note, velocity, gate, component) are reached and grabbed like any value. The inspector is left with button 1 (`step`) or another page, the way it is left with the buttons.
- **The ring.** The focused item has a **frame** 3 px outside it (2.5 px, in the colour of knob 1), and nothing else: no dot (Javi, 2026-11-15). A **track** is a small round capsule, so its frame is a **circle that hugs its edge** (radius 8 px, 10 px for the chosen track), not a box (Javi, 2026-11-16). It does not pulse or move by itself, so a still screen still sends nothing to the MK3. A **grabbed** value has a thicker (3.5 px) `fg` frame. **The ring is never cut off:** a screen button is a capsule at the very top of the screen, so its ring is drawn on the capsule's own edge (same size as the capsule, which is as wide as its label and no wider), and any other box has its ring pushed inward where 3 px outside would leave the screen (`nav::ring_box`). Javi, 2026-11-16. The ring is always on something: opening a view puts it where you were (the last place touched, the instrument the track has), on the track you are working on when composing, or on the first item. The cell last touched keeps its own 2 px `fg` outline inside the ring.
- **Turn** the 4D: the focus goes to the next or previous item in the list, wrapping; the right screen's items come after the left screen's. With a value grabbed, it turns the value.
- **Arrows ◀ ▲ ▼ ▶:** the focus goes to the nearest item that way, the right screen being to the right of the left one; nothing happens when there is nothing that way. An arrow also lets a grabbed value go.
- **Push:** does what tapping the item does: a place or a pad of the grid is that pad (a tap on a step toggles it; in the other pad modes it plays or chooses), a screen button is that button, a track is its group button; SHIFT does what SHIFT does to them (SHIFT + push on a place saves where SHIFT + a pad saves, on a track it joins or leaves the Brain, on a pad of the grid in the step view it opens that step's inspector). On a value it grabs it: turning changes the knob under it, push lets it go.
- **PERFORM** (Javi, 2026-11-16): an effect is played *while it is held*, which a push cannot do, so **push engages the effect and keeps it** (the pad lights as with a finger, and the frame turns `fg` and thick, like a grabbed value). While one sounds, **turning is its intensity** (what the pressure of the pad is; it starts at half), and the arrows still move the focus without letting it go. Pushing another effect takes its place; pushing the same one lets it go; leaving PERFORM lets it go too.
- **The tempo** is no longer turned by the 4D by itself: hold TEMPO and turn (SWING and VOLUME work the same way). In settings the 4D turns through the themes, and in the step inspector the focus is on its components (above).
- **Where it applies:** everywhere there is something to choose: FILE, PLUG-IN, BROWSER, SAMPLING, MIXER and composing (settings keeps its own: the encoder turns through the themes). With KEYBOARD ◀ ▶ stay the octave (except on a drum kit, where KEYBOARD is PAD MODE: the eight sounds, no octave); the turn and ▲ ▼ move the focus.

### Mix view (VOLUME held)

Decided with Javi on 2026-11-16: while VOLUME is held, the eight knobs are the **mix** of the eight tracks (knob 1 is track A … knob 8 is track H, left to right across both screens), and SHIFT turns them into the pans. It takes the area of the left screen between the strip and the footer, and replaces the footers of both screens, because while VOLUME is held the knobs are not the knobs of the page.

- **Left, in the area:** the word `mix` (`pan` while SHIFT is held) in `dim` at the top left, and the master volume (which the 4D encoder turns) at the top right in `fg`, with `master` under it. Eight columns 56 px apart (centres at x = 44 … 436): a **fader** 20 × 100 px (a `faint` slot, filled from the bottom in the colour of the track; `faint` if the track is muted; at 40 % of its colour while SHIFT is held), under it the **pan**: a 44 px `faint` line with a tick at the centre and a 5 px marker (`dim`, or `fg` while SHIFT is held), and under that the letter of the track (`fg` for the track that is open, `dim` for the others).
- **Footers:** four cells under each screen, each the letter of a track in its colour and its value in `fg`: the level as a percentage, or while SHIFT is held `C`, `L` and a number, or `R` and a number. Knobs 1 to 4 are under the left screen (A to D) and 5 to 8 under the right (E to H).
- **No focus ring** is drawn while VOLUME is held: the 4D turns the master volume, and nothing is to be chosen.
- **The engine:** a track's level plays as `level / 127` (never above 1; the default, 102, is the 0.8 every track had before) and its pan as a balance: the centre (64) is unity on both sides and the far side fades to nothing at the edge, so a hard pan is one speaker only. The mix is kept in the project (`Track::level`, `Track::pan`; format version 3).

### Mixer view (MIXER)

Decided with Javi on 2026-11-17: MIXER is a main mode that takes both screens, like SETTINGS (no footer; the strip names the pages). **Left:** the title `mixer` with the play mark, and **four channels** in the four columns of the knobs under the screen (120 px each): the letter of the track in its colour (and `mute` beside it in `dim` if it is muted), a **fader** (an 18 × 96 px `faint` slot, filled from the bottom in the colour of the track; `dim` if it is muted), under it the **pan** (a 72 px `faint` line, a tick at the centre and an `fg` dot) and, in the place of the footer, the name `level` in `dim` (`pan` while SHIFT is held) and the value in the colour of its knob: a percentage, or `c`, `l` and a number, or `r` and a number. The buttons 1 and 2 are `a–d` and `e–h`: which four channels are shown; the buttons 3 and 4 are the two sends, `delay` and `reverb` (Javi, 2026-11-17): while one is lit the faders show how much of each channel goes to it (filled in `fg`, not in the colour of the track), the name under the screen is `delay` or `reverb` and the knobs set it; SHIFT still turns them into the pans, whatever is lit; pressing the same button again goes back to the levels. **Right:** the title `compressor` and the compressor of the master drawn as a **curve** in a 150 px `panel` square (what comes in across, from −48 dB to 0, what comes out up), with the straight line of a sound that is left alone in `faint`, the curve in `fg` (it includes the make-up and the mix, as they sound), a small mark under it for the threshold in the colour of knob 5, `in` and `out` in `dim`, and a dot in `fg` where the sound is now; at its right a bar from the left, `reduction`, that fills in `fg` as the compressor brings the sound down (0 to 24 dB) with the number under it, and, while the compressor does nothing, a line in `dim` that says so (`it does nothing yet: lower the threshold`). Under the screen the values of the page, name in `dim`, value in the colour of its knob: page `comp` (button 5) has `threshold`, `ratio`, `attack` and `release`; page `output` (button 6) has `make-up`, `mix` and `side-chain` (Javi, 2026-11-17: `the mix`, or `track A` to `track H`, the one the compressor listens to instead of the mix: when that track is loud the whole mix comes down), and the fourth knob shows nothing (rule 11). **The sends page** (button 8, `sends`): two 200 × 130 px `panel` fields side by side, `delay` and `reverb` in `dim` at their top left. The delay is drawn as the taps it makes: vertical bars from the left, in `fg` for the first and `dim` for the repeats, each lower by the feedback and further apart for a longer time. The reverb is drawn as the way it dies away: a curve in `fg` that falls longer for a bigger room and is thinner and lower for a duller tone. Under the screen the four values, all used: `time` (`1/16`, `1/8`, `1/8.`, `1/4`, `1/4.`, `1/2` of a quarter note at the tempo), `feedback`, `size` and `tone`, in the colours of their knobs. **The taps page** (button 8 again, labelled `taps` while it is open; button 8 once more goes back to `sends`): two 200 × 130 px `panel` fields side by side, `delay` and `reverb`, each drawing the path of a track to the mix as a `dim` line with the fader as a `faint` box on it and, in `fg`, a short line that goes down off the path to the effect, **before the box or after it**; under the screen the two values, `after the fader` or `before the fader`, in the colours of their knobs (the other two show nothing). The 4D reaches the four levels and the values of the page (a ring on each, push grabs it, as everywhere); the pads and the group buttons do nothing here. **The engine:** the compressor sits after the performance effects and before the metronome and the master volume; it listens to the louder of the two channels, has a 6 dB knee and acts on the gain in dB. Threshold −40 to 0 dB, ratio 1:1 to 20:1 (finer at the low end), attack 0.1 to 100 ms, release 20 ms to 1 s, make-up 0 to +24 dB, mix 0 to 100 %. The defaults do nothing, so a project sounds as it did until the threshold is lowered. The six settings are the project's (format version 4). **The EQ page** (button 7, `eq`; Javi, 2026-11-17): the screen is the response of the three bands together, drawn across it in `fg` on a `panel` field 420 × 130 px, with the frequency on a logarithmic scale from 20 Hz to 20 kHz (a `faint` line and a `dim` number at 100, 1k and 10k) and ±15 dB high (a `faint` line at 0); a dot in the colour of its knob where each band is set (low at 100 Hz, mid at its frequency, high at 8 kHz) and a small mark under the box, in the colour of knob 7, at the middle band's frequency; while the three gains are at zero a line in `dim` says `flat: it does nothing yet`. Under the screen the four values, all four used: `low`, `mid`, `freq` and `high`, name in `dim` and value in the colour of its knob (gains as `+3 dB`, the frequency in Hz or kHz). The bands are a low shelf at 100 Hz, a peak at 200 Hz to 5 kHz (Q 0.9) and a high shelf at 8 kHz, each −15 to +15 dB, with 128 as no gain; the EQ goes before the compressor, and the project keeps its four settings (format version 5). **The sends in the engine:** each track sends a share of what it plays, taken after its level and pan, to a ping-pong delay that keeps time with the tempo and to a Freeverb reverb; what they give back joins the mix before the performance effects, the EQ and the compressor. With nothing sent they give back exact silence. The project keeps each track's two amounts (0 to 127) and the four settings (format version 6); a send is taken after the level and the pan of the track unless its tap says before them, and the compressor's side-chain is the sound of one track after its level and pan (format version 7 keeps the side-chain and the two taps).

### Naming (NAME)

Decided with Javi on 2026-11-17: what the player saves can be named on the unit. With **SELECT held**, a tap on one of the player's own places in FILE or BROWSER that has something in it opens NAME (both screens, no footer, no strip); a place that comes with the machine or one that is empty says why on the line of the view it came from and nothing opens. **Left:** the title `name` with what is being named in `dim` (`project user 3`), the name as **twelve places in a row** (34 × 52 px; the one with the cursor has a `panel` fill and a 2 px `fg` outline, the others a 1 px `faint` one; the letter in `fg` on the cursor and in `dim` elsewhere), and under it the last row of the pad grid as four outlined cells, `clear`, `delete`, `insert` and `save` (`save` in `fg`). **Right:** the title `letter` and `place 4 of 12`; across the middle **a strip of nine characters** with the one under the cursor large in a `panel` box with a 2 px outline in the colour of knob 1, four either side of it (the two nearest in `dim`, the others in `faint`); under it the name as it will be saved (`dim`, `(no name: user N)`, when it is empty) and two lines of hints. **Controls:** the 4D turns the letter under the cursor through a space, the small letters, the capitals, the digits and `- . + #` (round; with SHIFT ten at a time); the arrows ◀ ▶ and the twelve pads of the top three rows (in reading order) move the cursor; the last four pads are `clear`, `delete` (what follows comes one place to the left), `insert` (it goes one place to the right) and `save`; **push saves** the name next to the file, a name with nothing in it takes the name away, and either goes back to where it came from with the result on that view's line; FILE or BROWSER cancels. The pad LEDs are white: level 3 on the place with the cursor, 1 on the other eleven, 2 on the four actions. A name is at most twelve characters (the unit shows fourteen of a name written by hand). The recordings of SAMPLING are named the same way (Javi, 2026-11-17): SELECT held and a pad of a place with a recording opens NAME (`recording user 3`), and the cell of SAMPLING shows the name in place of `user 3`; a place with no recording says so on the line of SAMPLING.

### Settings view

Settings is a **main mode** (SETTINGS button; see [buttons.md](buttons.md)): it takes **both** screens, and the rule that the left is time and the right is sound is suspended while it is open. Composing (CHANNEL) is the other main mode. **Four pages**, decided with Javi on 2026-11-16, chosen with the four screen buttons over the left screen, which are named in the strip like any other (`theme`, `audio`, `midi`, `light`; the open one is the lit one; the buttons over the right screen do nothing here and name nothing). Each page uses both screens: the left shows the choices, the right shows what they mean. The left screen has the strip on top and its title and transport mark under it (a dot and the page name at the height of the header; the right screen has the title at the top).

- **`theme`** (opens first): **left**, one theme card for each theme, side by side, in the order chrome, matte, glossy; the active one has a 2 px `fg` outline. **Right**, the eight encoder colours as dials, in the active theme, and under them the eight track colours. The 4D turns through the themes, wrapping, and ◀ ▶ do too; the theme applies at once, to everything on screen, so the two screens are their own preview. No confirm step; the choice is saved as it is made. Chrome is the theme the unit starts in (Javi, 2026-11-16).
- **`audio`:** **left**, the audio outputs, one to a row of 456 × 24 px (at most eight, `automatic` first): the chosen one has an `fg` dot and a `panel` fill, the one that is open is tagged `in use` at the right (`automatic` shows the name of what it resolved to). **Right** (`input`), the inputs the same way (at most four), then `audio block` with three buttons (64, 128, 256 frames, the chosen one outlined and filled) and under them what the stream says it has (`48 kHz · 2 ch · buffer 128`). A device that is not there or cannot be opened falls back to the automatic one and the line at the bottom says so.
- **`midi`:** **left**, the MIDI output ports the same way. **Right** (Javi, 2026-11-17), titled `midi in`: the MIDI inputs as rows like the other lists (`off` first, the one that is open tagged `in use`), and under them two rows of buttons (108 × 30 px: four, then three): `play a note` (a note on channel 1 that ends by itself, to hear whether what is plugged in answers), `silence all` (all sound off and all notes off on the sixteen channels), `clock out: on` or `clock out: off` and `in: selected` or `in: by channel` (the last two lit with a `panel` fill and a thick `fg` outline while they are on); the second row has `clock in: on` or `clock in: off` (the tempo, start and stop come from the input), `learn` and `forget N` (Javi, 2026-11-17: `learn` says `turn a knob` and then `move a control`; the knob is one of the four under the right screen of an instrument, the control turns that parameter from then on, and `forget` takes all N away), and under them a line in `dim` that says what a note that comes in plays (`notes play the selected track` or `channels 1 to 8 play the tracks A to H`).
- **`light`:** **left**, `screen`: four steps of backlight (25, 50, 75 and 100 %), each a box with a bar that fills up to its share and the number, and `pads`: three levels of the pad LEDs, each a box with three squares lit up to the level and the number; the chosen ones outlined in `fg`. **Right**, what the supply allows (the line `power:` says) and that the screen and the pads are never brighter than that: a step is a **share of what the supply allows and of what the theme asks, never more** (`octachrome::settings::effective_light`), because a backlight that is too bright resets a MK3 that is fed too little. The step applies at once.
- **Choosing.** On the pages with choices the focus (the frame) is on the first choice; the 4D turns it along them (left screen first, then the right), its arrows move it across the screens, and a push chooses: a device is asked for and opened at once, kept in the settings file, and the line at the bottom of the right screen says what happened. The pads, knobs and groups do nothing here.
- **Nothing is drawn for a control that does nothing here** (rule 11): no footer, no page dots. The pads go dark.

## 5. Components

- **Dial:** 270° arc, 3 px; background `faint`, value in the encoder colour, a dot at the tip. Value in `fg` and name in the encoder colour beside it.
- **Ink over colour** (Javi, 2026-11-16): text or a drawing over a coloured fill (an effect sounding in PERFORM, the chord that plays in Brain) must read on the unit's own screen, where a saturated mid-tone such as a track's orange or red washes dark ink out. The fill goes through `octachrome_gfx::readable`: it is darkened until the theme's `fg` has a contrast of at least 4.5:1 over it (the fill of an effect still grows with its intensity, up to that limit); only a fill light enough for `bg` ink to have that contrast keeps its colour. Never write `bg` ink on a track colour by hand.
- **Pad on screen:** 6 px corners; empty = `faint` dot; with a note = fill in the track colour; being edited = 2 px `fg` border.
- **Capsule:** radius = height / 2, LED dot on the left, text without brackets, minimum margin of 8 px.
- **Calibration crosses:** 3 px `+` in the corners of each technical view.
- **Touch overlay:** darkens the drawing area; name in the encoder colour, value in `huge`, a thin 3 px bar.
- **Theme card:** 136 × 160 px, 8 px corners, filled with *its own* theme's `panel` colour. Inside: that theme's eight brand colours as small keys, four and four (the mark); a bar for the screen backlight it asks for; three squares for the level of the pad LEDs ([§2.5](#25-light)); and its name in `m`, lowercase, below. A card is always drawn in its own colours, never the active theme's, so all three can be compared at once. The active card has a 2 px `fg` outline and an `fg` name; the others a 1 px `faint` outline and a `dim` name.

## 6. Motion

60 FPS in the Twin; on the device, up to 60 FPS with only the changed rows sent (see [architecture.md §7](architecture.md)). Values are interpolated instead of jumping. Every note triggers a flash that decays, and the drawings respond to the audio level of their track. Both screens animate; neither is ever assumed to be still.

## 7. How this treatise is changed

1. The change is proposed in the Twin.
2. Javi approves it.
3. This document is updated in the same PR as the code.
