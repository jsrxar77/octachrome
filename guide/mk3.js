/* The Maschine MK3 faceplate, drawn the way the unit is laid out.
   ONE source for every picture of the device (guide, manual, brand): see docs/design.md §2.4.
   Coordinates are from the unit's front view, scaled into a 1000 x 960 box. */
(function () {
  const S = 2.2, X = x => (x - 38) * S, Y = y => (y - 6) * S;
  const R = (x, y, w, h) => [X(x), Y(y), w * S, h * S];
  const B = [];                                   // [label, x, y, w, h, status y|p|n, what it does in OP-8]
  const add = (l, r, s, d) => B.push([l, ...r, s, d]);

  // 8 screen buttons, above the screens
  [["step","y","Step grid and polymeter"],["components","p","Step components"],["players","y","Arpeggio and chords"],["bar","y","Length, division, shift and rec rounds"],["sound","y","Instrument page 1: what matters most"],["shape","y","Instrument page 2"],["fx","y","The track's effect"],["mod","y","The track's LFO"]]
    .forEach((b, i) => add(String(i + 1), R(153 + 39.1 * i, 40, 34, 12), b[1], b[0] + ": " + b[2]));
  // left block: two columns, six rows
  [[["CHANNEL","y","Back to composing"],["PLUG-IN","y","Choose the track's instrument: the 16 pads are the 16 places"]],
   [["ARRANGER","y","The song: a chain of scenes, each for some bars"],["MIXER","p","Level, pan and sends of the tracks, the compressor and the EQ of the master"]],
   [["BROWSER","p","Presets of the instrument: the 16 pads are the 16 presets"],["SAMPLING","p","Record a sound and put it in the sampler"]],
   [["PAGE ◀","n","Previous page of the current mode"],["PAGE ▶","n","Next page of the current mode"]],
   [["FILE","y","Open and save projects: the 16 pads are the 16 slots (factory top, yours bottom); SHIFT + a pad saves"],["SETTINGS","p","Both screens become settings: pick the theme with the 4D encoder"]],
   [["AUTO","n","Hold and turn a knob to record automation"],["MACRO","n","Brain, effect sends and inputs/outputs"]]]
    .forEach((row, r) => row.forEach((b, c) => add(b[0], R(c ? 90 : 40, [43, 65, 87, 109, 128, 150][r], c ? 34 : 36, r === 3 ? 12 : 17), b[1], b[2])));
  // volume / swing / tempo block, beside the 4D encoder
  [["VOLUME",131,203,32,"n","Master volume"],["NOTE REPEAT",169,203,34,"n","Repeat the held note"],["SWING",131,221,32,"n","Global swing"],["TEMPO",131,239,32,"n","Tempo"],["LOCK",169,239,34,"n","Snapshot of all the knobs"]]
    .forEach(b => add(b[0], R(b[1], b[2], b[3], 14), b[4], b[5]));
  // pad modes, above the pads
  [["FIXED VEL",232,31,"n","Fixed velocity"],["PAD MODE",275,31,"y","The pads play the instrument: a kit's eight sounds, or the scale"],["KEYBOARD",325,40,"y","Chromatic keyboard; the 4D encoder ◀ ▶ moves the octave"],["CHORDS",373,40,"y","Brain: pads are chord degrees"],["STEP",425,42,"y","Pads are the 16 steps"]]
    .forEach(b => add(b[0], R(b[1], 203, b[2], 12), b[3], b[4]));
  // column left of the pads
  [["SCENE",224,"y","16 scenes on the pads: tap one to bring it back, SHIFT + tap to store"],["PATTERN",245,"y","16 patterns of the track on the pads: tap to play one, hold and tap to copy"],["EVENTS",275,"n","Components of the selected step"],["VARIATION",299,"n","Random variation"],["DUPLICATE",324,"n","Copy a step, pattern or track"],["SELECT",346,"n","Choose without sounding"],["SOLO",369,"n","Isolate a track"],["MUTE",392,"y","Hold + a group to mute the track"]]
    .forEach(b => add(b[0], R(232, b[1], 31, 19), b[2], b[3]));
  // PITCH / MOD / PERFORM / NOTES
  [["PITCH","n","Smart Strip bends pitch"],["MOD","n","Smart Strip sends modulation"],["PERFORM","y","Punch-in FX: pressure is the amount"],["NOTES","n","Smart Strip plays notes"]]
    .forEach((b, i) => add(b[0], R(53 + 39 * i, 265, 32, 12), b[1], b[2]));
  // groups A–H: two rows of four
  "ABCDEFGH".split("").forEach((g, i) => add(g, R(53 + 39 * (i % 4), 323 + 24 * Math.floor(i / 4), 34, 19), "y", "Group " + g + ": choose track " + g + ". Shift joins it to the Brain."));
  // transport
  [["RESTART","y","Back to step 1 and keep playing"],["ERASE","p","Hold + a pad to erase the step"],["TAP","y","Tap tempo; SHIFT + TAP is the metronome"],["FOLLOW","n","Recording quantisation"]]
    .forEach((b, i) => add(b[0], R(53 + 39 * i, 386, 34, 12), b[1], b[2]));
  [["PLAY","y","Play / pause"],["REC","p","Record what you play into the track (notes; automation is still to come)"],["STOP","p","Stop (twice to silence: planned)"],["SHIFT","y","Secondary functions"]]
    .forEach((b, i) => add(b[0], R(53 + 39 * i, 405, 34, 18), b[1], b[2]));

  const pads = [], knobs = [];
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) pads.push(R(275 + 50 * c, 228 + 50 * r, 46, 46));
  for (let i = 0; i < 8; i++) knobs.push([X(168 + 39.4 * i), Y(178), 13 * S]);
  const group = i => R(53 + 39 * (i % 4), 323 + 24 * Math.floor(i / 4), 34, 19);   // A–H: 4 + 4

  window.MK3 = {
    W: 1000, H: 950, body: R(38, 6, 450, 432), BTN: B, pads, knobs, group,
    screens: [R(157, 72, 143, 76), R(318, 72, 134, 76)],
    encoder: [X(88), Y(226), 21 * S], strip: R(53, 288, 150, 17),
    /* the brand mark takes the unit's own arrangements: the eight group keys (4 + 4) or the eight knobs in a row */
    mark: {
      keys: { vb: "0 0 238 78", items: Array.from({ length: 8 }, (_, i) => ["rect", 10 + 56 * (i % 4), 10 + 32 * Math.floor(i / 4), 50, 26, 7]) },
      knobs: { vb: "0 0 314 48", items: Array.from({ length: 8 }, (_, i) => ["circle", 24 + 38 * i, 24, 14]) },
    },
  };
})();
