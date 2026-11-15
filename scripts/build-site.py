#!/usr/bin/env python3
"""Builds the public site into site/ from guide/ (the pages and their parts), twin/ and docs/buttons.md.

    python3 scripts/build-site.py            # then: python3 scripts/check-site.py

The site is a few static files: no server, no network at run time, no build step in the browser.
guide/site.json holds the placeholders for the day it is published (the address of the site and of the
repository); the environment variables OP8_SITE_URL and OP8_REPO_URL override them (the Pages workflow sets them).
"""
import json, os, pathlib, re, shutil

root = pathlib.Path(__file__).resolve().parent.parent
guide, out = root / "guide", root / "site"
cfg = json.loads((guide / "site.json").read_text())
site_url = (os.environ.get("OP8_SITE_URL") or cfg["site_url"]).rstrip("/")
repo_url = (os.environ.get("OP8_REPO_URL") or cfg["repo_url"]).rstrip("/")
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

# ---- the parts every page shares -----------------------------------------------------------------------------------
NAV = [("index.html", "Home"), ("manual.html", "Manual"), ("twin/op8-twin.html", "Try it")]

def header(page):
    cur = ' aria-current="page"'
    links = "".join(f'<a href="{h}"{cur if h == page else ""}>{t}</a>' for h, t in NAV)
    links += f'<a href="{repo_url}">Source</a>'
    return ('<a class="skip" href="#main">Skip to the content</a>\n'
            '<header class="site"><a class="home" href="index.html"><img src="brand/op8-mark.svg" alt=""><b>OP-8</b></a>'
            f'<nav aria-label="Site">{links}</nav></header>')

def footer():
    docs = " · ".join(f'<a href="{docs_url}/{f}.md">{t}</a>' for f, t in
                      [("architecture", "How it is built"), ("buttons", "Every button"), ("instruments", "Instruments"), ("design", "Design")])
    return ('<footer class="site">\n'
            f'<p><b>{cfg["name"]}</b> · {cfg["tagline"]}.</p>\n'
            '<p><a href="index.html">Home</a> · <a href="manual.html">Manual</a> · <a href="twin/op8-twin.html">Try it in the browser</a> · '
            f'<a href="{repo_url}">Source</a></p>\n<p>{docs}</p>\n'
            '<small>OP-8 is an independent project. Maschine is a trademark of Native Instruments and OP-1 and OP-XY of Teenage Engineering; '
            'OP-8 is not affiliated with either company.</small>\n</footer>')

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

(out / "index.html").write_text(render((guide / "index.tpl.html").read_text(), "index.html"))
(out / "manual.html").write_text(render((guide / "manual.tpl.html").read_text(), "manual.html", button_rows()))
# the brand page stays apart: not in the menu, not in the sitemap, and search engines are asked to leave it
brand = (guide / "brand.html").read_text()
if 'name="robots"' not in brand:
    brand = brand.replace("<title>", '<meta name="robots" content="noindex">\n<meta name="description" content="The OP-8 mark: eight buttons, four and four, in the order of the rainbow, the eighth white.">\n<title>', 1)
(out / "brand.html").write_text(brand)
(out / "404.html").write_text(render((guide / "404.tpl.html").read_text(), "404.html"))
pages = ["", "manual.html"]
(out / "sitemap.xml").write_text('<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
    + "".join(f"  <url><loc>{site_url}/{p}</loc></url>\n" for p in pages) + "</urlset>\n")
(out / "robots.txt").write_text(f"User-agent: *\nAllow: /\nDisallow: /brand.html\nSitemap: {site_url}/sitemap.xml\n")
print(f"site/ built: {len(list(out.rglob('*')))} files; site_url={site_url} repo_url={repo_url}")
