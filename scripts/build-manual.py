#!/usr/bin/env python3
"""Builds guide/manual.html from docs/buttons.md, so the manual never drifts from the button map.
Run: python3 scripts/build-manual.py"""
import json, re, pathlib
root = pathlib.Path(__file__).resolve().parent.parent
md = (root / "docs/buttons.md").read_text()
rows, section = [], ""
for line in md.splitlines():
    if line.startswith("## "):
        section = re.sub(r"^\d+\.\s*", "", line[3:]).split(" (")[0].strip()
        continue
    if not line.startswith("|") or set(line.replace("|", "").strip()) <= set(":- "):
        continue
    c = [x.strip() for x in line.strip().strip("|").split("|")]
    if c[0] in ("MK3 button", "Button", "Control"):
        continue
    n = len(c)
    what = c[3] if n == 5 else c[1]
    st = c[-1][:1]
    if st not in "✓◐○":
        continue
    rows.append({"s": section, "n": c[0], "w": re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", what).replace("**", ""),
                 "m": c[1] if n == 5 else "", "x": c[2] if n == 5 else "", "t": {"✓": "y", "◐": "p", "○": "n"}[st],
                 "note": c[-1][1:].strip(" ()")})
tpl = (root / "guide/manual.tpl.html").read_text()
(root / "guide/manual.html").write_text(tpl.replace("__DATA__", json.dumps(rows, ensure_ascii=False)))
print(len(rows), "rows")
