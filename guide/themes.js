/* The three themes, ONE definition for the guide and the brand page (docs/design.md §2).
   Mirrors crates/op8-gfx/src/theme.rs: keep the two in step.
   OP8.cur is shared between pages through localStorage, so choosing a theme in one changes the other. */
(function () {
  const RB = ["#FF2D2D","#FF8A1F","#FFD21F","#00E56A","#0099FF","#4B4BFF","#B15CFF","#F2F4F8"];
  const CH = ["#D633FF","#A442FF","#8652FF","#7361FF","#707AFF","#809DFF","#8FBCFF","#EEF0F5"];
  const THEMES = {
    glossy: { fg:"#F2F4F8", dim:"#7E8794", faint:"#232831", panel:"#0D1118", off:"#1A1D23", rec:"#FF3B30",
      knobs:["#0099FF","#00E56A","#F2F4F8","#FF4F00","#8B6CFF","#FFD21F","#2ED3E6","#FF4FA0"], brand:RB,
      tracks:["#FF3B30","#FF9500","#FFD60A","#34C759","#30D5C8","#0A84FF","#BF5AF2","#FF2D92"],
      light:{ bl:1, level:3, mono:1 }, cap:"Glossy · as bright as the supply allows",
      note:"Strong colours. Screens at all the backlight the supply allows, pad and group LEDs at level 3, the other buttons at full." },
    matte: { fg:"#F4EFE6", dim:"#8E918F", faint:"#262A30", panel:"#12161D", off:"#1E2127", rec:"#E63946",
      knobs:["#3B8EBA","#D9822B","#A9AFB8","#F2652A","#7FB58A","#E3C77A","#A88BC7","#D9677A"],
      brand:["#D9534F","#D9822B","#E3C77A","#7FB58A","#3B8EBA","#5F6BB5","#A88BC7","#F4EFE6"],
      tracks:["#D9534F","#D9822B","#E3C77A","#7FB58A","#6FB3B8","#3B8EBA","#A88BC7","#D9677A"],
      light:{ bl:.6, level:2, mono:.7 }, cap:"Matte · soft",
      note:"Chalky colours. Screens at 60 % of the backlight, pad and group LEDs at level 2, the other buttons at 70 %." },
    chrome: { fg:"#EEF0F5", dim:"#80858F", faint:"#24262C", panel:"#0F1014", off:"#1A1B20", rec:"#FF2244",
      knobs:CH, brand:CH,
      tracks:["#E83DFF","#D633FF","#A442FF","#8652FF","#7361FF","#5C8DFF","#8FBCFF","#EEF0F5"],
      light:{ bl:.8, level:3, mono:.85 }, cap:"Chrome · one range",
      note:"Monochrome: magenta-violet to blue, strong to soft, then white. Screens at 80 % of the backlight, pads and groups at level 3. To be tuned on the unit." },
  };
  const ORDER = ["chrome", "glossy", "matte"], KEY = "op8-theme", subs = [];
  let cur = "chrome";
  try { const s = localStorage.getItem(KEY); if (THEMES[s]) cur = s; } catch (e) {}
  const OP8 = { THEMES, ORDER, get cur() { return cur; },
    set(n) { if (!THEMES[n]) return; cur = n; try { localStorage.setItem(KEY, n); } catch (e) {} subs.forEach(f => f(n)); },
    on(f) { subs.push(f); } };
  addEventListener("storage", e => { if (e.key === KEY && THEMES[e.newValue]) { cur = e.newValue; subs.forEach(f => f(cur)); } });
  /* a tab row bound to the shared theme */
  OP8.tabs = function (box) {
    box.innerHTML = "";
    ORDER.forEach(n => { const b = document.createElement("button"); b.textContent = n; b.dataset.t = n; b.onclick = () => OP8.set(n); box.append(b); });
    const sync = n => [...box.children].forEach(b => b.classList.toggle("on", b.dataset.t === n));
    OP8.on(sync); sync(cur);
  };
  /* the whole unit in the current theme: screens, knobs, pad LEDs (needs mk3.js) */
  OP8.mount = function (svg, capEl) {
    svg.setAttribute("viewBox", `0 0 ${MK3.W} ${MK3.H}`);
    const NS = "http://www.w3.org/2000/svg", mk = (n, a, p) => { const e = document.createElementNS(NS, n); for (const k in a) e.setAttribute(k, a[k]); (p || svg).append(e); return e; };
    const [bx, by, bw, bh] = MK3.body, body = mk("rect", { x: bx, y: by, width: bw, height: bh, rx: 34, stroke: "#232831" });
    const sg = mk("g", {}), L = MK3.screens[0], R = MK3.screens[1];
    MK3.screens.forEach(r => mk("rect", { x: r[0], y: r[1], width: r[2], height: r[3], rx: 6, fill: "#000", stroke: "#232831" }, sg));
    const px = (L[2] - 20) / 16, py = (L[3] - 20) / 8, cells = [];
    for (let r = 0; r < 8; r++) for (let c = 0; c < 16; c++) cells.push(mk("rect", { x: L[0] + 10 + c * px, y: L[1] + 10 + r * py, width: px - 4, height: py - 4, rx: 2 }, sg));
    const wave = mk("path", { fill: "none", "stroke-width": 4, "stroke-linecap": "round" }, sg);
    const knobs = MK3.knobs.map(([cx, cy, r]) => ({ g: mk("circle", { cx, cy, r, fill: "#000", "stroke-width": 3.5 }), d: mk("circle", { cx, cy, r: 3 }) }));
    const [ex, ey, er] = MK3.encoder; mk("circle", { cx: ex, cy: ey, r: er, fill: "#000", stroke: "#7E8794", "stroke-width": 2 }); mk("circle", { cx: ex, cy: ey, r: er * .55, fill: "#232831" });
    const st = MK3.strip; mk("rect", { x: st[0], y: st[1], width: st[2], height: st[3], rx: st[3] / 2, fill: "#000", stroke: "#232831" });
    const btn = MK3.BTN.map(b => { const r = mk("rect", { x: b[1], y: b[2], width: b[3], height: b[4], rx: 4 }); mk("text", { x: b[1] + b[3] / 2, y: b[2] + b[4] / 2 + 3, "text-anchor": "middle", "font-size": b[0].length > 7 ? 7.5 : 9, fill: "#C9CED8", style: "pointer-events:none" }).textContent = b[0]; return { r, g: /^[A-H]$/.test(b[0]) ? b[0].charCodeAt(0) - 65 : -1 }; });
    const pads = MK3.pads.map(p => mk("rect", { x: p[0], y: p[1], width: p[2], height: p[3], rx: 8 }));
    const ACTIVE = [0, 3, 4, 7, 8, 10, 12, 15], LV = [0, .38, .68, 1];
    const caption = () => { if (capEl) capEl.innerHTML = `${THEMES[cur].cap}<small>${THEMES[cur].note}</small>`; };
    OP8.on(caption); caption();
    (function frame(ms) {
      const t = ms / 1000, T = THEMES[cur], step = Math.floor(t * 6) % 16, lv = LV[T.light.level];
      body.setAttribute("fill", T.panel); sg.style.filter = `brightness(${.35 + .65 * T.light.bl})`;
      btn.forEach(({ r, g }) => { r.setAttribute("fill", g >= 0 ? T.tracks[g] : T.faint); r.style.opacity = g >= 0 ? lv : .55 + .45 * T.light.mono; });
      cells.forEach((c, i) => { const r = Math.floor(i / 16), cc = i % 16, on = (r * 5 + cc * 3) % 7 < 2; c.setAttribute("fill", cc === step ? "#F2F4F8" : on ? T.tracks[r] : T.off); });
      const y = x => R[1] + R[3] * .45 + Math.sin((x - R[0]) / 22 + t * 2.5) * R[3] * .3; let d = "";
      for (let x = R[0] + 10; x <= R[0] + R[2] - 10; x += 4) d += (d ? "L" : "M") + x + " " + y(x);
      wave.setAttribute("d", d); wave.setAttribute("stroke", T.knobs[0]);
      knobs.forEach((k, i) => { k.g.setAttribute("stroke", T.knobs[i]); k.d.setAttribute("fill", T.knobs[i]); });
      pads.forEach((p, i) => { const on = ACTIVE.includes(i), head = i === step; p.setAttribute("fill", head ? "#F2F4F8" : on ? T.tracks[i % 8] : T.off); p.style.opacity = head || on ? lv : 1; });
      requestAnimationFrame(frame);
    })(0);
  };
  window.OP8 = OP8;
})();
