# OP-8 — UX/UI Treatise

This document rules over everything OP-8 shows and how it is operated. If the code or the Twin disagree with this treatise, the code is wrong. If something is needed that the treatise does not cover, it is written here first, approved, and only then programmed.

The direction is the language of Teenage Engineering (OP-1, OP-1 field, OP-XY): colour tied to the physical control, drawings instead of text, absolute black. We take the ideas, never their graphics, icons or typeface. It starts from the "Impeccable" system of Flip Tracker+ (`jsrxar77/flip-tracker`, its `DESIGN.md`), adapted to the Maschine MK3.

The living reference is **OP-8 Twin** (`twin/op8-twin.html`). Every new design is tried there before it is ported to `crates/op8-ui`.

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
12. **Each button means one idea.** What a MK3 button does in OP-8 resembles what its OP-XY equivalent does and what its silkscreen says. Shift gives the secondary function, never a different idea.
13. **Few parameters.** An instrument shows at most 8 parameters: 4 of sound (button 5, the first page) and 4 of shape (button 6, the second). The track's effect and modulation are the same for every instrument (4 + 4). If an engine has more, the cartridge picks the 8 that matter.

---

## 1. Control map

What each MK3 button does is in its own document: **[buttons.md](buttons.md)**. The instruments, which are the centre of OP-8, are in **[instruments.md](instruments.md)**.

## 2. Colour

Three themes, three characters, in this order: **matte** (the default) is softer and chalky, **glossy** is strong and bright, and **chrome** is monochrome: one range (violet drifting to blue, strong to soft) with white last. Every theme has exactly **eight** colours, always in an order that recalls the rainbow, and they are chosen for what the MK3's LEDs can show: 16 hues in 3 levels on the pads and group keys, one colour on every other button. The logic is the same in all three; only the palette changes. The hex values are ours, not measured from the devices.

### 2.1 Encoder colours

| Encoder | matte (default) | glossy | chrome |
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

The track colours (A–H) are part of each theme (`Theme.tracks` in `crates/op8-gfx/src/theme.rs`; `track_color(i)` in `op8-ui`). The pad and group LEDs take the same colours from the MK3's palette (`track_led_colors` in `crates/op8/src/controller.rs`): glossy is the rainbow, matte its softer hues, chrome one range from magenta to blue, then white. The palette order is still to be checked on the unit.

### 2.3 Brand mark

The mark of OP-8 is **eight buttons, one per colour of the rainbow, the eighth white**. Everything in OP-8 is built on eight, and the eighth colour is white because it is the **octave** (*8va*): in music the eighth note is the first one again, higher. The mark is the only picture that stands for the product (guide, twin, icon, README). The themes are named glossy, matte and chrome (they were op-1, op-1 field and op-xy; the old names still open an old settings file).

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
- The eight are laid out **as the MK3 lays out eight** (§2.4): the eight group keys, **four and four** (the main mark), or the **eight knobs in a row** (the long mark). Reading order is left to right, then the second row. Those two are the mark. The **infinity logo** is also allowed for the logo and the wordmark, with the same eight colours in the same order: an 8 on its side, eight keys along it, for OP-8 read as OP-∞ (`guide/brand.html`). A picture of the unit itself never uses it (§2.4).
- Always on absolute black. Never fewer than eight, never another colour, never outlined, never a gradient.
- The colours come from the active theme (`Theme.brand` in `crates/op8-gfx/src/theme.rs`). A theme may shift the range (softer in matte), but it always has **eight**: in glossy and matte the seven colours of the rainbow in their order, in chrome seven steps of one range, and in all of them the theme's own white last. A test checks that each button is the colour of its place (F-D09).
- The keys keep the proportion of the unit's group keys and rounded corners; the knobs are circles. A single button may light in turn, like a playhead; nothing else moves. In the guide the eight can be played as a major scale, the white being the octave.
- The wordmark is `OP-8` in Jost Light. The mark can stand alone or above the wordmark; the wordmark never carries the colours.
- The infinity logo is drawn from the same colours (`Theme.brand`); it has no master file yet.
- The master files are `guide/brand/op8-mark.svg` (4 + 4) and `guide/brand/op8-mark-knobs.svg` (a row). Their geometry is `MK3.mark` in `guide/mk3.js`.

### 2.4 Pictures of the unit

Every picture of the MK3 (the guide's map, the manual, the brand page, the twin, any future image) **follows the real front panel**, so the picture can be used to find the control in your hands.

- The master geometry is `guide/mk3.js`, scaled from the unit's front view. Drawings take their positions from it; nobody places a button by eye.
- The manual opens with a map of the unit drawn from it (`guide/unit-map.js`): every part is numbered and named, so the rest of the manual can say "button 5", "knob 7", "pad 12" or "group G". The manual, its pictures, the guide and the twin use the matte theme, the default; the other two are one press of the 4D encoder away in settings.
- Top to bottom: the eight screen buttons, the two screens, the eight knobs under them. On the left, six rows of two buttons (CHANNEL to MACRO), the 4D encoder with VOLUME, SWING and TEMPO beside it, then PITCH, MOD, PERFORM, NOTES, the Smart Strip, the group keys A–H as two rows of four, and the transport at the bottom. In the middle, the pad-mode buttons above the pads and SCENE to MUTE in a column to their left. On the right, the 4×4 pads.
- A group of **eight** is drawn as the eight knobs in a row or as the eight group keys, four and four. A group of **sixteen** is the 4×4 pads. Never another arrangement.
- The same rule applies to what the player sees in OP-8: the 8 factory and 8 user instruments, and the 8 factory and 8 user projects, are the 16 pads ([instruments.md §7](instruments.md)).

### 2.5 Light

A theme is also how much **light** the machine gives, not only its colours. The theme asks for a share of what the supply allows ([architecture.md §10](architecture.md)), and never for more:

| | Screens' backlight | Pad and group LEDs | Other buttons and strip |
| :-- | :-- | :-- | :-- |
| glossy | all that the supply allows | level 3 | full |
| matte | 60 % | level 2 | 70 % |
| chrome | 80 % (to be tuned on the unit) | level 3 | 85 % |

The values are `Theme.light` in `crates/op8-gfx/src/theme.rs`; the supply's ceiling is the power plan in `crates/op8-hw/src/mk3/power.rs` (F-H07). `guide/themes.html` shows the three on the whole unit. The monochrome theme is still open: its colours and its light are provisional until it is seen on the MK3.

### 2.6 Light of the buttons

Every button follows one rule, the same for all of them (`crates/op8/src/lights.rs`, one table):

- **Dark:** it does nothing in the current context (REC, TAP… until they are built; MUTE and ERASE in settings).
- **Very faint white:** it does something here. It can be pressed, but it is not in use.
- **Strong white:** it is pressed or held, or it has the focus: the pad mode in use, CHANNEL while composing and SETTINGS in settings, the page that is open, SHIFT, MUTE or ERASE while held, STOP while stopped and PLAY while playing.

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

**Centring, always.** Text inside a container (a capsule, a pill, a key, a card) is centred as the eye does, never by its font metrics: the *ink* between the first and the last glyph is centred horizontally (side bearings and trailing letter spacing do not count), and the *x-height* is centred vertically, so the baselines of different labels line up. In Rust that is `text_centered` in `crates/op8-gfx/src/font.rs`; use it, not `text_v` with `Align::Center`, whenever text sits in a container. A test measures it (F-D13).

**FlipMono is not used:** the `FlipMono.ttf` file of Flip Tracker+ is Apple's SF Mono, whose licence only allows it on Apple devices. It cannot be distributed on a Raspberry Pi.

In Rust, Jost (300 and 400, `assets/fonts/`) is embedded with `include_bytes!` and rasterised with `fontdue` (MIT/Apache); each glyph is cached and alpha-blended onto the RGB565 framebuffer (`crates/op8-gfx/src/font.rs`).

## 3b. Where each thing lives in the code

| What | File |
| :-- | :-- |
| Themes, encoder colours, brand colours and light | `crates/op8-gfx/src/theme.rs` (`theme()`, `knob(i)`, `Theme.brand`) |
| Brand mark (master files) | `guide/brand/op8-mark.svg`, `guide/brand/op8-mark-knobs.svg` |
| MK3 front-panel geometry (every picture of the unit) | `guide/mk3.js` |
| Brand colours in the twin | `THEMES[...].brand` in `twin/op8-twin.html` |
| Typeface and type scale | `crates/op8-gfx/src/font.rs` |
| Anti-aliased lines, arcs, circles and boxes | `crates/op8-gfx/src/paint.rs` |
| Dial footer and touch overlay | `crates/op8-ui/src/theme.rs` |
| Left screen | `crates/op8-ui/src/left.rs` |
| Right screen (frame) | `crates/op8-ui/src/right.rs` |
| Drawing of each instrument | `draw_ui` of each cartridge in `crates/op8-cartridges/src/` |

The theme is chosen with `op8 run -t matte|glossy|chrome` or `OP8_THEME`.

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

- **Button strip (24 px, y 0..24):** the name of what each of the four buttons above the screen does, `lab` (15 px), inside a capsule (22 px high, 14 px of padding each side, drawn on whole pixels as a 1 px band between two filled shapes so the outline is the same width all the way round, the text centred by the height of its lowercase letters, `panel` fill) **centred on the physical button**. The buttons sit about 140 px apart, centred at x = 24, 164, 304 and 444 on each screen (`BUTTON_CX` in `crates/op8-ui/src/theme.rs`, measured on the unit; a capsule never leaves the screen). The open page has an `fg` outline and `fg` text, the others a `faint` outline and `dim` text. A button with no function shows nothing, and in settings the strip is empty. The strip sits above the header items, which are centred at y = 37, leaving a few pixels of air between them. Left: `step` (button 1: back to the step view), `components` (2: the inspector), `players` (3) and `bar` (4). Right: `sound` (5, the first page), `shape` (6, only for an instrument with a second page), `fx` (7) and `mod` (8), in that order. **Choosing a track, or an instrument, puts the screens on their first pages: the steps on the left (button 1) and `sound` on the right (button 5), so the two buttons that are lit are always 1 and 5, and what matters most of an instrument lives on its first page.** **A label and its button's LED always agree** (one table, `screen_buttons` in `crates/op8-ui/src/lib.rs`): a button with no function has no label and a dark LED; with a function it has a label and a dim LED; the open page has the bright label and the full LED. In settings all eight are dark.
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

FILE is a main mode like settings: both screens, no footer, no strip. **Left:** the sixteen slots as the pads are (reading order, top-left first), each a rounded cell with the project's name (`m`); the top two rows are the factory projects (`panel` fill) and the bottom two are the player's (`faint` fill, or an outline and `empty` where there is nothing); the slot last touched has a 2 px `fg` outline; the title is `open`, or `save` while SHIFT is held. **Right:** `projects`, what a tap does and what SHIFT + tap does, and below a line saying what just happened. The pad LEDs are white: level 2 for the factory slots, 3 for the player's that hold a project, 1 for the empty ones. The play mark stays on the left.

### Instruments view (PLUG-IN)

Like FILE (above): both screens, no footer, no strip. **Left:** the sixteen places as the pads are, with the same cells; the top two rows are the factory instruments in the order of [instruments.md §7](instruments.md), the instrument of the selected track has the 2 px `fg` outline, one that is not built yet has an outline only, and the bottom two rows are the places for installed instruments (`empty`). **Right:** `plug-in`, what a tap does, and what just happened. The pad LEDs are white: level 3 for the track's instrument, 2 for the others that can be chosen, off for the rest.

### Sampling view (SAMPLING)

Like FILE: both screens, no footer, no strip. **Left:** `sampling` and the sixteen pads as cells: the top two rows are `user 1` to `user 8`, the places of the player's own sounds (`panel` fill where one already has a sound), the bottom two rows say `listen` (filled once there is a take). **Right:** a dot and `sampling` (the dot pulses and the title reads `recording` while it records) and the length of the take at the right of the bar; the wave across the screen with the part that is kept lit in `fg` on a `panel` field and its two ends marked in the colours of knobs 6 and 7; a dot that goes along the line while there is no take (the screen moves while it waits); a level bar under the wave; one line saying what just happened; and under it the four knobs of the take, name in `dim` and value in the knob's colour: `source`, `start`, `end`, `fade` (knobs 5 to 8). The pad LEDs are white: level 3 for a place that has a sound, 1 for an empty one, level 2 for `listen` once there is a take. REC is lit while it records.

### Presets view (BROWSER)

Like FILE and PLUG-IN: both screens, no footer, no strip. **Left:** the sixteen places as the pads are; the top two rows are the presets that come with the instrument (`panel` fill and their names; an instrument with fewer than eight leaves the rest `empty`) and the bottom two are the player's (`user 1` to `user 8`, with a fill where there is one); the one last touched has the 2 px `fg` outline; the title is `presets`, or `save preset` while SHIFT is held. **Right:** `browser` and the instrument's name, what a tap and SHIFT + tap do, and a line saying what just happened. The pad LEDs are white: level 3 for the last one touched, 2 for a preset that exists, 1 for an empty place of the player's.

### Navigation with the 4D encoder

Decided with Javi on 2026-11-15: **every view that has things to choose has a focus, always, and the 4D encoder works on it**, so the unit can be played without reaching for the pads.

- **Items.** A view lists what can be reached as items (a box on a screen), in `crates/op8-ui/src/nav.rs`; the same list draws the focus ring and answers the encoder, so what can be reached and what is drawn cannot drift apart. In FILE, PLUG-IN, BROWSER and SAMPLING the items are the sixteen places (reading order, the order of the pads), and SAMPLING also has its four values (`source`, `start`, `end`, `fade`) under the right screen. **Composing** lists, in this order: the screen buttons that have a name on the left (1–4), the eight tracks (the dots of the header), the sixteen pads of the grid that mirrors the pads (when the left screen shows it: not under the players and bar pages), the four values of the left footer, then the screen buttons of the right screen (5–8, `shape` only if the instrument has a second page) and the four values of the right footer: forty things. The pads are the ones the left screen **draws**, wherever it draws them: the 37 px grid of the steps and of the notes (KEYBOARD, PAD MODE), the sixteen larger effect places of PERFORM (108 × 36 px, `punch_rect`), and none in Brain, which draws no pads. The step inspector lists nothing: the 4D keeps moving its component cursor, and a push closes it.
- **The ring.** The focused item has a **frame** 3 px outside it (2.5 px, in the colour of knob 1), and nothing else: no dot (Javi, 2026-11-15). A **track** is a small round capsule, so its frame is a **circle that hugs its edge** (radius 8 px, 10 px for the chosen track), not a box (Javi, 2026-11-16). It does not pulse or move by itself, so a still screen still sends nothing to the MK3. A **grabbed** value has a thicker (3.5 px) `fg` frame. **The ring is never cut off:** a screen button is a capsule at the very top of the screen, so its ring is drawn on the capsule's own edge (same size as the capsule, which is as wide as its label and no wider), and any other box has its ring pushed inward where 3 px outside would leave the screen (`nav::ring_box`). Javi, 2026-11-16. The ring is always on something: opening a view puts it where you were (the last place touched, the instrument the track has), on the track you are working on when composing, or on the first item. The cell last touched keeps its own 2 px `fg` outline inside the ring.
- **Turn** the 4D: the focus goes to the next or previous item in the list, wrapping; the right screen's items come after the left screen's. With a value grabbed, it turns the value.
- **Arrows ◀ ▲ ▼ ▶:** the focus goes to the nearest item that way, the right screen being to the right of the left one; nothing happens when there is nothing that way. An arrow also lets a grabbed value go.
- **Push:** does what tapping the item does: a place or a pad of the grid is that pad (a tap on a step toggles it; in the other pad modes it plays or chooses), a screen button is that button, a track is its group button; SHIFT does what SHIFT does to them (SHIFT + push on a place saves where SHIFT + a pad saves, on a track it joins or leaves the Brain, on a pad of the grid in the step view it opens that step's inspector, and push closes it). On a value it grabs it: turning changes the knob under it, push lets it go.
- **PERFORM** (Javi, 2026-11-16): an effect is played *while it is held*, which a push cannot do, so **push engages the effect and keeps it** (the pad lights as with a finger, and the frame turns `fg` and thick, like a grabbed value). While one sounds, **turning is its intensity** (what the pressure of the pad is; it starts at half), and the arrows still move the focus without letting it go. Pushing another effect takes its place; pushing the same one lets it go; leaving PERFORM lets it go too.
- **The tempo** is no longer turned by the 4D by itself: hold TEMPO and turn (SWING and VOLUME work the same way). In settings the 4D turns through the themes, and in the step inspector it moves the component cursor, as before.
- **Where it applies:** everywhere there is something to choose: FILE, PLUG-IN, BROWSER, SAMPLING and composing (settings keeps its own: the encoder turns through the themes). With KEYBOARD ◀ ▶ stay the octave; the turn and ▲ ▼ move the focus.

### Settings view

Settings is a **main mode** (SETTINGS button; see [buttons.md](buttons.md)): it takes **both** screens, and the rule that the left is time and the right is sound is suspended while it is open. Composing (CHANNEL) is the other main mode. Each setting is a page that uses both screens: the left shows the choices, the right shows what they mean. Today there is one page, the theme.

- **Top bar** of both screens: a `fg` dot and the lowercase title (`settings` on the left, the page name on the right). The left screen keeps a small transport mark at its right edge (a play triangle, or a dim square when stopped), because PLAY still works.
- **Left, the theme:** one theme card for each theme, side by side; the active one has a 2 px `fg` outline.
- **Right, the theme:** the eight encoder colours as dials, in the active theme, and under them the eight track colours.
- **The theme applies at once**, to everything on screen, so the two screens are their own preview. There is no confirm step; the choice is saved as it is made.
- **Nothing is drawn for a control that does nothing here** (rule 11): no footer, no page dots while there is a single page. The pads go dark.
- The 4D encoder (turn, or ◀ ▶) changes the value, wrapping from the last to the first.

## 5. Components

- **Dial:** 270° arc, 3 px; background `faint`, value in the encoder colour, a dot at the tip. Value in `fg` and name in the encoder colour beside it.
- **Ink over colour** (Javi, 2026-11-16): text or a drawing over a coloured fill (an effect sounding in PERFORM, the chord that plays in Brain) must read on the unit's own screen, where a saturated mid-tone such as a track's orange or red washes dark ink out. The fill goes through `op8_gfx::readable`: it is darkened until the theme's `fg` has a contrast of at least 4.5:1 over it (the fill of an effect still grows with its intensity, up to that limit); only a fill light enough for `bg` ink to have that contrast keeps its colour. Never write `bg` ink on a track colour by hand.
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
