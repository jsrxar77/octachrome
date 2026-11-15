/* The map of the unit at the start of the manual: the faceplate from mk3.js (the one source of every picture of the unit,
   docs/design.md §2.4), every part numbered and named, in the chrome theme. A button's colour is its status in docs/buttons.md
   (the manual passes that table as `rows`). */
(function () {
  window.drawUnitMap = function (host, rows) {
    const M = window.MK3, T = window.OP8.THEMES.chrome, NS = "http://www.w3.org/2000/svg";
    const el = (tag, attrs, parent, text) => { const e = document.createElementNS(NS, tag); for (const k in attrs) e.setAttribute(k, attrs[k]); if (text != null) e.textContent = text; (parent || svg).appendChild(e); return e; };
    const svg = document.createElementNS(NS, "svg");
    svg.setAttribute("viewBox", "0 0 " + M.W + " " + M.H); svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", "The Maschine MK3 front panel with every part numbered");
    const status = label => {
      const r = rows.find(x => x.n.split(" · ")[0].replace(/\s+/g, " ") === label || x.n.split(" · ")[0].replace(/\s+/g, " ") === label.replace("PAGE ◀", "PAGE ◀ ▶").replace("PAGE ▶", "PAGE ◀ ▶"));
      return r ? r.t : null;
    };
    const fill = { y: T.knobs[6], p: T.knobs[2], n: T.off };
    const box = [38, 6, 450, 432];
    el("rect", { x: M.body[0], y: M.body[1], width: M.body[2], height: M.body[3], rx: 26, fill: T.panel, stroke: T.faint, "stroke-width": 2 });
    // screens
    ["LEFT SCREEN", "RIGHT SCREEN"].forEach((t, i) => {
      const [x, y, w, h] = M.screens[i];
      el("rect", { x, y, width: w, height: h, rx: 8, fill: "#000", stroke: T.dim, "stroke-width": 1.5 });
      el("text", { x: x + w / 2, y: y + h / 2 - 4, "text-anchor": "middle", fill: T.fg, "font-size": 16, "letter-spacing": ".08em" }, null, t);
      el("text", { x: x + w / 2, y: y + h / 2 + 18, "text-anchor": "middle", fill: T.dim, "font-size": 13 }, null, i ? "sound · knobs 5–8" : "time · knobs 1–4");
    });
    // buttons
    M.BTN.forEach(([label, x, y, w, h, st]) => {
      const s = status(label) || (/^[1-8]$/.test(label) ? st : st);
      const g = el("g", {});
      el("rect", { x, y, width: w, height: h, rx: 6, fill: T.panel, stroke: fill[s] || T.dim, "stroke-width": 2 }, g);
      el("rect", { x: x + 5, y: y + h - 5, width: w - 10, height: 2.5, rx: 1.2, fill: fill[s] || T.faint }, g);
      const names = ["step", "components", "players", "bar", "sound", "shape", "fx", "mod"];
      const text = /^[1-8]$/.test(label) ? label + " " + names[+label - 1] : label;
      el("text", { x: x + w / 2, y: y + h / 2 + 1, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.fg, "font-size": text.length > 9 ? 10 : 12, "letter-spacing": ".02em" }, g, text);
    });
    // knobs and pads
    M.knobs.forEach(([cx, cy, r], i) => {
      el("circle", { cx, cy, r, fill: T.panel, stroke: T.knobs[i], "stroke-width": 3 });
      el("line", { x1: cx, y1: cy - r * .35, x2: cx, y2: cy - r * .85, stroke: T.knobs[i], "stroke-width": 3, "stroke-linecap": "round" });
      el("text", { x: cx, y: cy + 10, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.fg, "font-size": 15 }, null, String(i + 1));
    });
    M.pads.forEach(([x, y, w, h], i) => {
      el("rect", { x, y, width: w, height: h, rx: 8, fill: T.panel, stroke: T.tracks[i % 8], "stroke-width": 1.6, opacity: .95 });
      el("text", { x: x + w / 2, y: y + h / 2 + 1, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.dim, "font-size": 15 }, null, String(i + 1));
    });
    // encoder and strip
    const [ex, ey, er] = M.encoder;
    el("circle", { cx: ex, cy: ey, r: er, fill: T.panel, stroke: T.fg, "stroke-width": 2.5 });
    el("circle", { cx: ex, cy: ey, r: er * .42, fill: T.faint });
    [["▲", 0, -er * .72], ["▼", 0, er * .72], ["◀", -er * .72, 0], ["▶", er * .72, 0]].forEach(([c, dx, dy]) => el("text", { x: ex + dx, y: ey + dy, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.dim, "font-size": 12 }, null, c));
    el("text", { x: ex, y: ey + 1, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.fg, "font-size": 12 }, null, "4D");
    const [sx, sy, sw, sh] = M.strip;
    el("rect", { x: sx, y: sy, width: sw, height: sh, rx: 8, fill: T.panel, stroke: T.dim, "stroke-width": 1.5 });
    for (let i = 0; i < 25; i++) el("rect", { x: sx + 6 + i * ((sw - 12) / 25), y: sy + 5, width: (sw - 12) / 25 - 2, height: sh - 10, rx: 1.5, fill: T.faint });
    el("text", { x: sx + sw / 2, y: sy + sh / 2 + 1, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.dim, "font-size": 11, "letter-spacing": ".12em" }, null, "SMART STRIP");
    // the regions, numbered
    const byLabel = ls => M.BTN.filter(b => ls.includes(b[0])).map(b => [b[1], b[2], b[3], b[4]]);
    const REG = [
      [1, byLabel(["1", "2", "3", "4", "5", "6", "7", "8"])],
      [2, M.screens],
      [3, M.knobs.map(([cx, cy, r]) => [cx - r, cy - r, 2 * r, 2 * r])],
      [4, M.pads],
      [5, byLabel(["A", "B", "C", "D", "E", "F", "G", "H"])],
      [6, byLabel(["RESTART", "ERASE", "TAP", "FOLLOW", "PLAY", "REC", "STOP", "SHIFT"])],
      [7, byLabel(["FIXED VEL", "PAD MODE", "KEYBOARD", "CHORDS", "STEP"])],
      [8, byLabel(["CHANNEL", "PLUG-IN", "ARRANGER", "MIXER", "BROWSER", "SAMPLING", "PAGE ◀", "PAGE ▶", "FILE", "SETTINGS", "AUTO", "MACRO"])],
      [9, [[ex - er, ey - er, 2 * er, 2 * er]]],
      [10, [M.strip]],
      [11, byLabel(["VOLUME", "NOTE REPEAT", "SWING", "TEMPO", "LOCK"])],
      [12, byLabel(["SCENE", "PATTERN", "EVENTS", "VARIATION", "DUPLICATE", "SELECT", "SOLO", "MUTE"])],
      [13, byLabel(["PITCH", "MOD", "PERFORM", "NOTES"])],
    ];
    REG.forEach(([n, rects]) => {
      const x0 = Math.min(...rects.map(r => r[0])) - 7, y0 = Math.min(...rects.map(r => r[1])) - 7;
      const x1 = Math.max(...rects.map(r => r[0] + r[2])) + 7, y1 = Math.max(...rects.map(r => r[1] + r[3])) + 7;
      const c = T.knobs[(n - 1) % 8 === 7 ? 6 : (n - 1) % 8];
      el("rect", { x: x0, y: y0, width: x1 - x0, height: y1 - y0, rx: 12, fill: "none", stroke: c, "stroke-width": 1.5, "stroke-dasharray": "5 4", opacity: .85 });
      el("circle", { cx: x0 + 2, cy: y0 + 2, r: 12, fill: "#000", stroke: c, "stroke-width": 2 });
      el("text", { x: x0 + 2, y: y0 + 3, "text-anchor": "middle", "dominant-baseline": "middle", fill: T.fg, "font-size": 13, "font-weight": 600 }, null, String(n));
    });
    // crop to the body, with a margin
    const b = M.body; svg.setAttribute("viewBox", [b[0] - 24, b[1] - 24, b[2] + 48, b[3] + 48].join(" "));
    host.appendChild(svg);
  };
})();
