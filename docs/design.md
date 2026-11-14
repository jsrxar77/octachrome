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
11. **Buttons above, knobs below.** The top strip of each screen shows what each button above it does; the footer shows what each knob below it does. Always in the same 120 px column as the physical control. Button labels are white or grey, never in a knob colour. A button with no function shows nothing.
12. **Each button means one idea.** What a MK3 button does in OP-8 resembles what its OP-XY equivalent does and what its silkscreen says. Shift gives the secondary function, never a different idea.
13. **Few parameters.** An instrument shows at most 8 parameters: 4 of sound (button 5) and 4 of shape (button 6). The track's effect and modulation are the same for every instrument (4 + 4). If an engine has more, the cartridge picks the 8 that matter.

---

## 1. Control map

What each MK3 button does is in its own document: **[buttons.md](buttons.md)**. The instruments, which are the centre of OP-8, are in **[instruments.md](instruments.md)**.

## 2. Colour

Three themes, three characters: **glossy** is strong and bright, **matte** is softer and chalky, and **chrome** is monochrome: one range (violet drifting to blue, strong to soft) with white last. Every theme has exactly **eight** colours, always in an order that recalls the rainbow, and they are chosen for what the MK3's LEDs can show: 16 hues in 3 levels on the pads and group keys, one colour on every other button. The logic is the same in all three; only the palette changes. The hex values are ours, not measured from the devices.

### 2.1 Encoder colours

| Encoder | glossy (default) | matte | chrome |
| :-- | :-- | :-- | :-- |
| 1 | `#0099FF` blue | `#3B8EBA` steel blue | `#D633FF` violet-magenta |
| 2 | `#00E56A` green | `#D9822B` ochre | `#A442FF` violet |
| 3 | `#F2F4F8` white | `#A9AFB8` grey | `#8652FF` soft violet |
| 4 | `#FF4F00` orange | `#F2652A` orange | `#7361FF` periwinkle |
| 5 | `#8B6CFF` violet | `#7FB58A` sage | `#707AFF` cornflower |
| 6 | `#FFD21F` yellow | `#E3C77A` sand | `#809DFF` sky |
| 7 | `#2ED3E6` cyan | `#A88BC7` lilac | `#8FBCFF` pale blue |
| 8 | `#FF4FA0` pink | `#D9677A` dusty rose | `#EEF0F5` white |

The first four follow the encoder order of each reference device (original OP-1: blue, green, white, orange; OP-1 field: blue, ochre, grey, orange). The four instrument colours are chosen so they are not confused with the first four at a glance.

### 2.2 Base colours

| Token | glossy | matte | chrome | Use |
| :-- | :-- | :-- | :-- | :-- |
| `bg` | `#000000` | `#000000` | `#000000` | Background |
| `fg` | `#F2F4F8` | `#F4EFE6` | `#EEF0F5` | Text and main strokes |
| `dim` | `#7E8794` | `#8E918F` | `#80858F` | Context, not editable now |
| `faint` | `#232831` | `#262A30` | `#24262C` | Guides, passive borders, empty steps |
| `panel` | `#0D1118` | `#12161D` | `#0F1014` | Surface of keys and capsules |
| `rec` | `#FF3B30` | `#E63946` | `#FF2244` | Only REC, erase and peaks |

The track colours (A–H) are one fixed palette shared by the three themes (`crates/op8-ui/src/theme.rs`, `TRACK_COLORS`); they match the pad LED palette. Whether they should follow the theme is an open question in [roadmap.md](roadmap.md).

### 2.3 Brand mark

The mark of OP-8 is **eight buttons, one per colour of the rainbow, the eighth white**. Everything in OP-8 is built on eight, and the eighth colour is white because it is the **octave** (*8va*): in music the eighth note is the first one again, higher. The mark is the only picture that stands for the product (guide, twin, icon, README). The themes are named glossy, matte and chrome (they were op-1, op-1 field and op-xy; the old names still open an old settings file).

| Button | glossy | matte | chrome |
| :-- | :-- | :-- | :-- |
| 1 red | `#FF2D2D` | `#D9534F` | `#D633FF` |
| 2 orange | `#FF8A1F` | `#D9822B` | `#A442FF` |
| 3 yellow | `#FFD21F` | `#E3C77A` | `#8652FF` |
| 4 green | `#00E56A` | `#7FB58A` | `#7361FF` |
| 5 blue | `#0099FF` | `#3B8EBA` | `#707AFF` |
| 6 indigo | `#4B4BFF` | `#5F6BB5` | `#809DFF` |
| 7 violet | `#B15CFF` | `#A88BC7` | `#8FBCFF` |
| 8 white | `#F2F4F8` | `#F4EFE6` | `#EEF0F5` |

Rules of the mark:

- Always eight buttons, always in a fixed order that recalls the rainbow. In glossy and matte it is **red, orange, yellow, green, blue, indigo, violet, white**. In chrome it is one range, from the strongest violet drifting to blue, then white. The order never changes, in any size or use.
- The eight are laid out **as the MK3 lays out eight** (§2.4): the eight group keys, **four and four** (the main mark), or the **eight knobs in a row** (the long mark). Reading order is left to right, then the second row. Never another arrangement: no loop, no circle, no stairs.
- Always on absolute black. Never fewer than eight, never another colour, never outlined, never a gradient.
- The colours come from the active theme (`Theme.brand` in `crates/op8-gfx/src/theme.rs`). A theme may shift the range (softer in matte), but it always has **eight**: in glossy and matte the seven colours of the rainbow in their order, in chrome seven steps of one range, and in all of them the theme's own white last. A test checks that each button is the colour of its place (F-D09).
- The keys keep the proportion of the unit's group keys and rounded corners; the knobs are circles. A single button may light in turn, like a playhead; nothing else moves. In the guide the eight can be played as a major scale, the white being the octave.
- The wordmark is `OP-8` in Jost Light. The mark can stand alone or above the wordmark; the wordmark never carries the colours.
- The master files are `guide/brand/op8-mark.svg` (4 + 4) and `guide/brand/op8-mark-knobs.svg` (a row). Their geometry is `MK3.mark` in `guide/mk3.js`.

### 2.4 Pictures of the unit

Every picture of the MK3 (the guide's map, the manual, the brand page, the twin, any future image) **follows the real front panel**, so the picture can be used to find the control in your hands.

- The master geometry is `guide/mk3.js`, scaled from the unit's front view. Drawings take their positions from it; nobody places a button by eye.
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

## 3. Typography

The screens are a real 480×272, so the type has to be readable at 11–15 px. It is rendered with anti-aliasing (TrueType).

**Typeface: Jost** (OFL), chosen by Javi on 2026-11-12 and tuned to resemble the OP-XY screen: light weights, small labels with some air between letters, and large numbers in the thinnest weight. One family for everything. Teenage Engineering does not publish which typeface the OP-XY uses, so the tuning is by eye against what is seen on the device, not a copy.

| Level | px | Weight | Spacing | Use |
| :-- | :-- | :-- | :-- | :-- |
| `s` | 11 | 400 | +0.6 px | Parameter name under the dial |
| `m` | 15 | 400 | +0.3 px | Value under the dial |
| `l` | 20 | 400 | +0.2 px | Inspector values |
| `mid` | 21 | 400 | +0.2 px | Instrument, chord, track letter |
| `big` | 34 | 300 | 0 | Step number in the inspector |
| `huge` | 62 | 300 | −1 px | Value when an encoder is touched |

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

The theme is chosen with `op8 run -t glossy|matte|chrome` or `OP8_THEME`.

## 4. Screen structure

```
┌────────────────────────────────────────────────┐
│ ● name                           ● ● (pages)   │  top bar, 32 px
│ +                                            + │
│               animated drawing                 │  drawing area
│ +                                            + │
│ ◔ value    ◔ value    ◔ value    ◔ value       │  footer, 4 × 120 px, each column
│   name       name       name       name        │  above its physical encoder
└────────────────────────────────────────────────┘
```

- **Left (fixed): time.** Tempo, the Brain chord, polymeter rings (one per track, the selected one in front), a mirror of the 16 pads. Footer with encoders 1–4.
- **Right: the instrument of the selected track.** Each cartridge draws its animated illustration using colours 5–8 for what those encoders move. Footer with encoders 5–8.

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
- **Pad on screen:** 6 px corners; empty = `faint` dot; with a note = fill in the track colour; being edited = 2 px `fg` border.
- **Capsule:** radius = height / 2, LED dot on the left, text without brackets, minimum margin of 8 px.
- **Calibration crosses:** 3 px `+` in the corners of each technical view.
- **Touch overlay:** darkens the drawing area; name in the encoder colour, value in `huge`, a thin 3 px bar.
- **Theme card:** 136 × 160 px, 8 px corners, filled with *its own* theme's `panel` colour. Inside: that theme's eight encoder colours as 9 px dots in two rows of four; four chips (`fg`, `dim`, `faint`, `rec`); and its name in `m`, lowercase, below. A card is always drawn in its own colours, never the active theme's, so all three can be compared at once. The active card has a 2 px `fg` outline and an `fg` name; the others a 1 px `faint` outline and a `dim` name.

## 6. Motion

60 FPS in the Twin; on the device, up to 60 FPS with only the changed rows sent (see [architecture.md §7](architecture.md)). Values are interpolated instead of jumping. Every note triggers a flash that decays, and the drawings respond to the audio level of their track. Both screens animate; neither is ever assumed to be still.

## 7. How this treatise is changed

1. The change is proposed in the Twin.
2. Javi approves it.
3. This document is updated in the same PR as the code.
