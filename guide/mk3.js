/* The Maschine MK3 faceplate, drawn the way the unit is laid out.
   ONE source for every picture of the device (guide, manual, brand): see docs/design.md §2.4.
   Coordinates are MEASURED on a top-down photo of a real unit (docs/design.md §2.4, "Where the positions come from"):
   they are in photo pixels, with the unit's top-left corner at (74, 299) and its size 838 x 793, and are scaled into a 1000-wide box. */
(function () {
  const S = 1.18, X = x => (x - 74) * S, Y = y => (y - 299) * S;
  const R = (x, y, w, h) => [X(x), Y(y), w * S, h * S];
  const B = [];                                   // [label, x, y, w, h, status y|p|n, what it does in Octachrome]
  const add = (l, r, s, d) => B.push([l, ...r, s, d]);

  // 8 screen buttons, above the screens
  [["step","y","Step grid and polymeter"],["components","p","Step components"],["players","y","Arpeggio and chords"],["bar","y","Length, division, shift and rec rounds"],["sound","y","Instrument page 1: what matters most"],["shape","y","Instrument page 2"],["fx","y","The track's effect"],["mod","y","The track's LFO"]]
    .forEach((b, i) => add(String(i + 1), R([286, 361, 434, 507, 581, 655, 727, 801][i], 319, 65, 24), b[1], b[0] + ": " + b[2]));
  // left block: two columns, six rows
  [[["CHANNEL","y","Back to composing"],["PLUG-IN","y","Choose the track's instrument: the 16 pads are the 16 places"]],
   [["ARRANGER","y","The song: a chain of scenes, each for some bars"],["MIXER","y","Level, pan and sends of the tracks, the compressor and the EQ of the master"]],
   [["BROWSER","p","Presets of the instrument: the 16 pads are the 16 presets"],["SAMPLING","p","Record a sound and put it in the sampler"]],
   [["PAGE ◀","n","Previous page of the current mode"],["PAGE ▶","n","Next page of the current mode"]],
   [["FILE","y","Open and save projects: the 16 pads are the 16 slots (factory top, yours bottom); SHIFT + a pad saves"],["SETTINGS","p","Both screens become settings: pick the theme with the 4D encoder"]],
   [["AUTO","n","Hold and turn a knob to record automation"],["MACRO","n","Brain, effect sends and inputs/outputs"]]]
    .forEach((row, r) => row.forEach((b, c) => add(b[0], R(c ? 165 : 90, [321, 361, 408, 455, 485, 518][r], 67, [22, 38, 40, 23, 24, 25][r]), b[1], b[2])));
  // volume / swing / tempo block, beside the 4D encoder
  [["VOLUME",239,625,68,24,"y","Master volume"],["NOTE REPEAT",316,625,67,54,"n","Repeat the held note"],["SWING",239,656,68,23,"y","Global swing"],["TEMPO",239,689,68,24,"y","Tempo"],["LOCK",316,689,67,24,"n","Snapshot of all the knobs"]]
    .forEach(b => add(b[0], R(b[1], b[2], b[3], b[4]), b[5], b[6]));
  // pad modes, above the pads
  [["FIXED VEL",434,66,"y","Fixed velocity"],["PAD MODE",519,89,"y","The pads play the instrument: a kit's eight sounds, or the scale"],["KEYBOARD",616,89,"y","Chromatic keyboard; the 4D encoder ◀ ▶ moves the octave"],["CHORDS",714,89,"y","Brain: pads are chord degrees"],["STEP",812,89,"y","Pads are the 16 steps"]]
    .forEach(b => add(b[0], R(b[1], 624, b[2], 24), b[3], b[4]));
  // column left of the pads
  [["SCENE",224,"y","16 scenes on the pads: tap one to bring it back, SHIFT + tap to store"],["PATTERN",245,"y","16 patterns of the track on the pads: tap to play one, hold and tap to copy"],["EVENTS",275,"n","Components of the selected step"],["VARIATION",299,"n","Random variation"],["DUPLICATE",324,"n","Copy a step, pattern or track"],["SELECT",346,"p","Held in FILE or BROWSER, a tap on one of your own places names it"],["SOLO",369,"n","Isolate a track"],["MUTE",392,"y","Hold + a group to mute the track"]]
    .forEach((b, i) => add(b[0], R(436, [667, 717, 768, 818, 868, 919, 969, 1020][i], 66, 41), b[2], b[3]));
  // PITCH / MOD / PERFORM / NOTES
  [["PITCH","n","Smart Strip bends pitch"],["MOD","n","Smart Strip sends modulation"],["PERFORM","y","Punch-in FX: pressure is the amount"],["NOTES","n","Smart Strip plays notes"]]
    .forEach((b, i) => add(b[0], R([81, 158, 239, 316][i], 746, 69, 24), b[1], b[2]));
  // groups A–H: two rows of four
  "ABCDEFGH".split("").forEach((g, i) => add(g, R([77, 155, 235, 314][i % 4], 861 + 51 * Math.floor(i / 4), 70, 39), "y", "Group " + g + ": choose track " + g + ". Shift joins it to the Brain."));
  // transport
  [["RESTART","y","Back to step 1 and keep playing"],["ERASE","p","Hold + a pad to erase the step"],["TAP","y","Tap tempo; SHIFT + TAP is the metronome"],["FOLLOW","n","Recording quantisation"]]
    .forEach((b, i) => add(b[0], R([77, 155, 235, 314][i], 988, 68, 26), b[1], b[2]));
  [["PLAY","y","Play / pause"],["REC","p","Record what you play into the track (notes; automation is still to come)"],["STOP","p","Stop (twice to silence: planned)"],["SHIFT","y","Secondary functions"]]
    .forEach((b, i) => add(b[0], R([77, 155, 235, 314][i], 1021, 68, 41), b[1], b[2]));

  const pads = [], knobs = [];
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) pads.push(R(519 + 97.7 * c, 668 + 99 * r, 90, 91));
  for (let i = 0; i < 8; i++) knobs.push([X(317 + 75.4 * i), Y(573), 22 * S]);
  const group = i => R([77, 155, 235, 314][i % 4], 861 + 51 * Math.floor(i / 4), 70, 39);   // A–H: 4 + 4

  window.MK3 = {
    W: 1000, H: 940, body: R(74, 299, 838, 793), BTN: B, pads, knobs, group,
    screens: [R(303, 384, 252, 141), R(599, 383, 252, 141)],
    encoder: [X(158), Y(670), 38 * S], strip: R(98, 792, 280, 44),
    /* the brand mark takes the unit's own arrangements: the eight group keys (4 + 4) or the eight knobs in a row */
    mark: {
      keys: { vb: "0 0 238 78", items: Array.from({ length: 8 }, (_, i) => ["rect", 10 + 56 * (i % 4), 10 + 32 * Math.floor(i / 4), 50, 26, 7]) },
      knobs: { vb: "0 0 314 48", items: Array.from({ length: 8 }, (_, i) => ["circle", 24 + 38 * i, 24, 14]) },
    },
  };
})();
