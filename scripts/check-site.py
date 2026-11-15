#!/usr/bin/env python3
"""Checks site/ (built by scripts/build-site.py) the way a visitor and a search engine would meet it.

Errors (exit 1): a link, image, script or stylesheet that points at a file that is not there; an in-site #anchor that
does not exist; a page without a title, a description or exactly one <h1>; an image without alt text; a template
marker left unreplaced; a reference to a file outside the site. Warnings (exit 0): the placeholders that are still to be
filled in before publishing (the address of the site and of the repository).
"""
import pathlib, re, sys
from html.parser import HTMLParser

root = pathlib.Path(__file__).resolve().parent.parent
site = root / "site"
errors, warnings = [], []


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.refs, self.ids, self.h1, self.imgs_no_alt, self.title, self.desc = [], set(), 0, [], "", None
        self._in_title = False

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if a.get("id"):
            self.ids.add(a["id"])
        if tag == "h1":
            self.h1 += 1
        if tag == "title":
            self._in_title = True
        if tag == "meta" and a.get("name") == "description":
            self.desc = a.get("content")
        if tag == "img" and a.get("alt") is None:
            self.imgs_no_alt.append(a.get("src"))
        for k in ("href", "src"):
            if a.get(k) and tag in ("a", "link", "script", "img", "source"):
                self.refs.append(a[k])

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False

    def handle_data(self, data):
        if self._in_title:
            self.title += data


pages = {}
for f in sorted(site.rglob("*.html")):
    if "twin" in f.relative_to(site).parts:
        continue  # the twin is one file with its own world; it is only linked
    text = f.read_text()
    p = Page()
    p.feed(text)
    pages[f] = (p, text)

for f, (p, text) in pages.items():
    name = str(f.relative_to(site))
    if not p.title.strip():
        errors.append(f"{name}: no <title>")
    if not (p.desc or "").strip():
        errors.append(f"{name}: no meta description")
    if p.h1 != 1 and name != "brand.html":
        errors.append(f"{name}: {p.h1} <h1> (should be one)")
    for src in p.imgs_no_alt:
        errors.append(f"{name}: image without alt: {src}")
    for marker in ("<!--HEADER-->", "<!--FOOTER-->", "<!--SOCIAL-->", "__DATA__", "{{"):
        if marker in text.replace("${{", ""):
            # {{ appears in JavaScript template strings of the guide; only our own markers matter
            if marker == "{{" and not re.search(r"\{\{(DOCS|REPO|SITE)\}\}", text):
                continue
            errors.append(f"{name}: template marker left: {marker}")
    for ref in p.refs:
        if re.match(r"^(https?:|mailto:|data:|tel:|javascript:)", ref) or ref.startswith("//"):
            if "SITE-URL-PENDING" in ref:
                warnings.append("the address of the site is still the placeholder (guide/site.json or OP8_SITE_URL)")
            if "REPO-OWNER-PENDING" in ref:
                warnings.append("the address of the repository is still the placeholder (guide/site.json or OP8_REPO_URL)")
            continue
        target, _, frag = ref.partition("#")
        path = (f.parent / target) if target else f
        path = pathlib.Path(str(path).split("?")[0])
        try:
            path.resolve().relative_to(site.resolve())
        except ValueError:
            errors.append(f"{name}: link leaves the site: {ref}")
            continue
        if path.is_dir():
            path = path / "index.html"
        if not path.exists():
            errors.append(f"{name}: broken link: {ref}")
            continue
        if frag and path.suffix == ".html" and path in pages and frag not in pages[path][0].ids:
            # ids made by JavaScript (the guide builds its sections) are listed in the page as strings
            if f'"{frag}"' not in pages[path][1] and f"'{frag}'" not in pages[path][1] and f'id="{frag}"' not in pages[path][1]:
                errors.append(f"{name}: no #{frag} in {path.relative_to(site)}")

for needed in ("robots.txt", "sitemap.xml", "404.html", "og.png", "brand/op8-mark.svg"):
    if not (site / needed).exists():
        errors.append(f"missing {needed}")
if "brand.html" in (site / "sitemap.xml").read_text():
    errors.append("brand.html is in the sitemap (it stays apart)")

for w in sorted(set(warnings)):
    print("WARN ", w)
for e in errors:
    print("FAIL ", e)
print(f"{len(pages)} pages checked: {len(errors)} errors, {len(set(warnings))} warnings")
sys.exit(1 if errors else 0)
