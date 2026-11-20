#!/usr/bin/env python3
"""Writes THIRD-PARTY-NOTICES.md: every crate Octachrome is built with (name, version, licence and the licence texts that come with it), and the
code and fonts that are copied into the repository. It also refuses a licence that is not permissive (GPL, AGPL, LGPL alone, MPL, unknown).

    python3 scripts/third-party-notices.py            # writes THIRD-PARTY-NOTICES.md
    python3 scripts/third-party-notices.py --check    # fails if it is out of date or a licence is not allowed (CI)

Run it after changing Cargo.lock. The release packages carry the file (docs/architecture.md, "Licences").
"""
import hashlib, json, pathlib, re, subprocess, sys

root = pathlib.Path(__file__).resolve().parent.parent
out = root / "THIRD-PARTY-NOTICES.md"

meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--locked"], cwd=root, check=True, capture_output=True, text=True).stdout)
ws = set(meta["workspace_members"])
OK = {"MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unlicense", "CC0-1.0", "Unicode-3.0", "Apache-2.0 WITH LLVM-exception"}

def terms(expr):
    """The alternatives of an SPDX expression: each is a list of licences that must all hold."""
    expr = expr.replace("/", " OR ").replace("(", " ").replace(")", " ")
    return [[x.strip() for x in alt.split(" AND ")] for alt in re.split(r"\s+OR\s+", expr)]

problems, rows, texts = [], [], {}
for p in sorted((p for p in meta["packages"] if p["id"] not in ws), key=lambda p: (p["name"], p["version"])):
    lic = (p.get("license") or "").strip()
    if not lic:
        problems.append(f"{p['name']} {p['version']}: no licence declared")
        continue
    # one alternative where every licence is allowed is enough: that is the one we use
    if not any(all(l in OK for l in alt) for alt in terms(lic)):
        problems.append(f"{p['name']} {p['version']}: licence not allowed ({lic})")
    d = pathlib.Path(p["manifest_path"]).parent
    found = sorted(f for f in d.iterdir() if f.is_file() and re.match(r"(?i)(licen[sc]e|copying|unlicense|notice)", f.name))
    keys = []
    for f in found:
        t = f.read_text(errors="replace").strip()
        h = hashlib.sha256(re.sub(r"\s+", " ", t).encode()).hexdigest()[:10]
        texts.setdefault(h, [t, []])[1].append(f"{p['name']} {p['version']}")
        keys.append(h)
    rows.append((p["name"], p["version"], lic, p.get("repository") or "", keys))

if problems:
    print("\n".join(problems), file=sys.stderr)
    sys.exit(1)

L = ["# Third-party notices", "",
     "Octachrome is built with the code and the fonts below. Each keeps its own licence; the texts that come with the crates are at the end. "
     "Octachrome's own code is under the licence in `LICENSE`.", "",
     "## Copied into the repository", "",
     "* **Plaits and stmlib** (`crates/octachrome-plaits/vendor`): by Mutable Instruments, copyright Emilie Gillet, MIT licence "
     "(the notice is in every file and in `crates/octachrome-plaits/vendor/LICENSE-stmlib`). Octachrome does not use their module names.",
     "* **Granular instrument**: the way grains are scheduled, shaped and added follows the granular player of Clouds by Mutable Instruments "
     "(copyright Emilie Gillet, MIT licence). It is written anew in Rust.",
     "* **Jost** (`assets/fonts`): copyright the Jost Project Authors, SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).", "",
     "## Rust crates", "", "| Crate | Version | Licence | Source |", "| :-- | :-- | :-- | :-- |"]
for n, v, lic, repo, _ in rows:
    L.append(f"| {n} | {v} | {lic} | {repo} |")
L += ["", "## Licence texts", ""]
ofl = (root / "assets/fonts/OFL.txt")
if ofl.exists():
    L += ["### SIL Open Font License 1.1 (Jost)", "", "```", ofl.read_text().strip(), "```", ""]
vend = root / "crates/octachrome-plaits/vendor/LICENSE-stmlib"
if vend.exists():
    L += ["### MIT licence of Mutable Instruments (stmlib, Plaits)", "", "```", vend.read_text().strip(), "```", ""]
for h, (t, who) in sorted(texts.items(), key=lambda kv: kv[1][1][0]):
    L += [f"### Text shared by {len(who)} crate{'s' if len(who) != 1 else ''}", "", "Crates: " + ", ".join(sorted(set(who))), "", "```", t, "```", ""]
no_text = [f"{n} {v}" for n, v, _, _, k in rows if not k]
if no_text:
    L += ["### Crates that carry no licence file", "", "Their licence is the one declared in the table above: " + ", ".join(no_text), ""]
text = "\n".join(L).rstrip() + "\n"

if "--check" in sys.argv:
    if not out.exists() or out.read_text() != text:
        print("THIRD-PARTY-NOTICES.md is out of date: run python3 scripts/third-party-notices.py", file=sys.stderr)
        sys.exit(1)
    print(f"{len(rows)} crates, all licences allowed, notices up to date")
else:
    out.write_text(text)
    print(f"{out.name}: {len(rows)} crates, {len(texts)} distinct licence texts, {len(no_text)} without a file, {len(text)//1024} KB")
