#!/usr/bin/env python3
"""Builds the public site into site/ from guide/ (the pages and their parts), twin/ and docs/buttons.md.

    python3 scripts/build-site.py            # then: python3 scripts/check-site.py

The site is a few static files: no server, no network at run time, no build step in the browser.
guide/site.json holds the placeholders for the day it is published (the address of the site and of the
repository); the environment variables OCTACHROME_SITE_URL and OCTACHROME_REPO_URL override them (the Pages workflow sets them).
"""
import json, os, pathlib, re, shutil

root = pathlib.Path(__file__).resolve().parent.parent
guide, out = root / "guide", root / "site"
cfg = json.loads((guide / "site.json").read_text())
site_url = (os.environ.get("OCTACHROME_SITE_URL") or cfg["site_url"]).rstrip("/")
repo_url = (os.environ.get("OCTACHROME_REPO_URL") or cfg["repo_url"]).rstrip("/")
docs_url = repo_url + "/blob/main/docs"

# ---- the button map, for the manual's reference list (docs/buttons.md is the source) -------------------------------
def button_rows():
    rows, section = [], ""
    for line in (root / "docs/buttons.md").read_text().splitlines():
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
    return rows

def md_inline(text):
    """The inline Markdown of the documents (**bold**, `code`) as HTML, with everything else escaped."""
    text = text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    text = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", text)
    return re.sub(r"`([^`]+)`", r"<code>\1</code>", text)

def piece_rows():
    """The manual's table of the eight pieces (docs/instruments.md §9 is the source)."""
    text = (root / "docs/instruments.md").read_text()
    sec = text[text.index("## 9. The eight pieces"):]
    rows = []
    for line in sec.splitlines():
        c = [x.strip() for x in line.strip().strip("|").split("|")]
        if line.startswith("|") and len(c) == 4 and c[0].isdigit():
            name = c[1].replace("**", "")
            rows.append(f"  <tr><td>{name}</td><td><b>{c[2].replace(', heavy swing', '')}</b> · {md_inline(c[3])}</td></tr>")
    return "\n".join(rows)

def preset_rows():
    """The manual's table of the presets that come with the instruments (crates/octachrome-cartridges/presets.txt)."""
    by = {}
    for line in (root / "crates/octachrome-cartridges/presets.txt").read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            p = [x.strip() for x in line.split("|")]
            by.setdefault(p[0], []).append(p[1])
    names = [("drums", "drums"), ("KickSyn", "KickSyn"), ("FmBass", "FmBass"), ("resonator", "resonator"), ("sampler", "sampler"), ("macro", "macro")]
    return "\n".join(f"  <tr><td>{label}</td><td>{', '.join(by.get(key, []))}</td></tr>" for label, key in names)

# ---- the parts every page shares -----------------------------------------------------------------------------------
NAV = [("index.html", "Home"), ("manual.html", "Manual"), ("twin/octachrome-twin.html", "Try it")]

def header(page):
    cur = ' aria-current="page"'
    links = "".join(f'<a href="{h}"{cur if h == page else ""}>{t}</a>' for h, t in NAV)
    links += f'<a href="{repo_url}">Source</a>'
    return ('<a class="skip" href="#main">Skip to the content</a>\n'
            '<header class="site"><a class="home" href="index.html"><img src="brand/octachrome-mark.svg" alt=""><b>Octachrome</b></a>'
            f'<nav aria-label="Site">{links}</nav></header>')

def footer():
    docs = " · ".join(f'<a href="{docs_url}/{f}.md">{t}</a>' for f, t in
                      [("architecture", "How it is built"), ("buttons", "Every button"), ("instruments", "Instruments"), ("design", "Design")])
    return ('<footer class="site">\n'
            f'<p><b>{cfg["name"]}</b> · {cfg["tagline"]}.</p>\n'
            '<p><a href="index.html">Home</a> · <a href="manual.html">Manual</a> · <a href="twin/octachrome-twin.html">Try it in the browser</a> · '
            f'<a href="{repo_url}">Source</a></p>\n<p>{docs}</p>\n'
            '<small>Octachrome is an independent project. Maschine is a trademark of Native Instruments and OP-1 and OP-XY of Teenage Engineering; '
            'Octachrome is not affiliated with either company.</small>\n</footer>')

def social(html, page):
    title = re.search(r"<title>(.*?)</title>", html, re.S).group(1).strip()
    desc = re.search(r'<meta name="description" content="(.*?)"', html, re.S)
    desc = desc.group(1) if desc else cfg["description"]
    url = f"{site_url}/{'' if page == 'index.html' else page}"
    img = f"{site_url}/og.png"
    return "\n".join([
        f'<link rel="canonical" href="{url}">',
        '<meta name="theme-color" content="#000000">',
        f'<meta property="og:site_name" content="{cfg["name"]}">', '<meta property="og:type" content="website">',
        f'<meta property="og:title" content="{title}">', f'<meta property="og:description" content="{desc}">',
        f'<meta property="og:url" content="{url}">', f'<meta property="og:image" content="{img}">',
        '<meta property="og:image:width" content="1200"><meta property="og:image:height" content="630">',
        '<meta name="twitter:card" content="summary_large_image">',
        f'<meta name="twitter:title" content="{title}">', f'<meta name="twitter:description" content="{desc}">',
        f'<meta name="twitter:image" content="{img}">'])

def render(text, page, data=None):
    text = text.replace("<!--SOCIAL-->", social(text, page)).replace("<!--HEADER-->", header(page)).replace("<!--FOOTER-->", footer())
    if data is not None:
        text = text.replace("__DATA__", json.dumps(data, ensure_ascii=False))
    rows = data if data is not None else button_rows()
    for key, n in (("N_Y", sum(r["t"] == "y" for r in rows)), ("N_P", sum(r["t"] == "p" for r in rows)),
                   ("N_N", sum(r["t"] == "n" for r in rows)), ("N_ALL", len(rows))):
        text = text.replace("{{" + key + "}}", str(n))
    text = text.replace("__PRESET_ROWS__", preset_rows()).replace("__PIECE_ROWS__", piece_rows())
    return text.replace("{{DOCS}}", docs_url).replace("{{REPO}}", repo_url).replace("{{SITE}}", site_url)

# ---- build ------------------------------------------------------------------------------------------------------------
if out.exists():
    shutil.rmtree(out)
out.mkdir()
for d in ("fonts", "img", "brand"):
    shutil.copytree(guide / d, out / d)
for f in ("site.css", "mk3.js", "themes.js", "unit-map.js", "og.png"):
    if (guide / f).exists():
        shutil.copy(guide / f, out / f)
shutil.copytree(root / "twin", out / "twin", ignore=shutil.ignore_patterns("*.md", ".*"))

# The twin is one self-contained file (it also runs from disk), so the site's menu is added to the published copy only.
twin_page = out / "twin" / "octachrome-twin.html"
cur = ' aria-current="page"'
twin_links = "".join(f'<a href="../{h}"{cur if h.startswith("twin/") else ""}>{t}</a>' for h, t in NAV) + f'<a href="{repo_url}">Source</a>'
twin_bar = (
    '<style>header.site{position:sticky;top:0;z-index:50;display:flex;align-items:center;justify-content:space-between;gap:16px;'
    'padding:10px 18px;margin:0 0 14px;background:rgba(13,14,16,.92);border-bottom:1px solid #454953;font:400 15px/1 Jost,system-ui,sans-serif}'
    'header.site a{color:#8e918f;text-decoration:none}header.site a:hover,header.site a[aria-current=page]{color:#f4efe6}'
    'header.site .home{display:flex;align-items:center;gap:10px;color:#f4efe6;letter-spacing:.06em}header.site .home img{width:34px;height:auto;display:block}'
    'header.site nav{display:flex;gap:6px 20px;flex-wrap:wrap;justify-content:flex-end}'
    '@media(max-width:560px){header.site{padding:8px 12px;font-size:14px}header.site .home b{display:none}}</style>\n'
    '<header class="site"><a class="home" href="../index.html"><img src="../brand/octachrome-mark.svg" alt=""><b>Octachrome</b></a>'
    f'<nav aria-label="Site">{twin_links}</nav></header>\n')
twin_html = twin_page.read_text()
assert '<div class="wrap">' in twin_html
twin_page.write_text(twin_html.replace('<div class="wrap">', twin_bar + '<div class="wrap">', 1))

(out / "index.html").write_text(render((guide / "index.tpl.html").read_text(), "index.html"))
(out / "manual.html").write_text(render((guide / "manual.tpl.html").read_text(), "manual.html", button_rows()))
# the brand page stays apart: not in the menu, not in the sitemap, and search engines are asked to leave it
brand = (guide / "brand.html").read_text()
if 'name="robots"' not in brand:
    brand = brand.replace("<title>", '<meta name="robots" content="noindex">\n<meta name="description" content="The Octachrome mark: eight buttons, four and four, in the order of the rainbow, the eighth white.">\n<title>', 1)
(out / "brand.html").write_text(brand)
(out / "404.html").write_text(render((guide / "404.tpl.html").read_text(), "404.html"))
pages = ["", "manual.html"]
(out / "sitemap.xml").write_text('<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
    + "".join(f"  <url><loc>{site_url}/{p}</loc></url>\n" for p in pages) + "</urlset>\n")
(out / "robots.txt").write_text(f"User-agent: *\nAllow: /\nDisallow: /brand.html\nSitemap: {site_url}/sitemap.xml\n")
print(f"site/ built: {len(list(out.rglob('*')))} files; site_url={site_url} repo_url={repo_url}")
