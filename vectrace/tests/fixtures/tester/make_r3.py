"""Renders the tester's round 3 lines of `specs/0091-trace-text-recognition` criterion 29.

Each picture holds one line in a face Curvyo does not include (system fonts: Adwaita Sans,
Liberation Mono; Arch Linux paths), black on white. Lines are placed letter by letter on their
ink so the gaps between letters are exact: `em` is the em of Source Sans 3 at the line's letter
height (cap height / 0.66). Writes `r3/<name>.png` and `r3/r3.json` with, per picture, the string
the reader is taken to have read, each character's ink box (a space: the gap it stands in) and
the em. Run from anywhere: `python3 make_r3.py`.
"""

import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent / "r3"
ADWAITA = "/usr/share/fonts/Adwaita/AdwaitaSans-Regular.ttf"
MONO = "/usr/share/fonts/liberation/LiberationMono-Regular.ttf"
SIZE = 100
CAP = 0.66
CASES = []


def face(path, weight=None):
    f = ImageFont.truetype(path, SIZE)
    if weight:
        f.set_variation_by_name(weight)
    return f


def ink(f, c):
    """The ink box of `c` drawn at (0, 0) with `f`: left, top, right, bottom (PIL's `getbbox`
    gives the advance across, not the ink)."""
    image = Image.new("L", (4 * SIZE, 3 * SIZE), 0)
    ImageDraw.Draw(image).text((SIZE, SIZE), c, font=f, fill=255)
    l, t, r, b = image.getbbox()
    return l - SIZE, t - SIZE, r - SIZE, b - SIZE


def em_of(f):
    box = ink(f, "H")
    return (box[3] - box[1]) / CAP


def placed(name, f, string, gap, extra=None, shear=0.0, squeeze=1.0):
    """`string` letter by letter: `gap(i)` em of ink between letter i and the next; a space in
    `string` is a gap of `extra` em with no ink."""
    em = em_of(f)
    extra = extra or {}
    width = 120 + int(sum(f.getlength(c) for c in string) + len(string) * em * 2.5)
    image = Image.new("L", (width, 260), 255)
    draw = ImageDraw.Draw(image)
    x, base = 60.0, 60
    boxes = []
    for i, c in enumerate(string):
        if c == " ":
            w = extra.get(i, 1.0) * em
            boxes.append([x, base, x + w, base + em * CAP])
            x += w
            continue
        b = ink(f, c)
        left = x - b[0]
        draw.text((left, base), c, font=f, fill=0)
        boxes.append([x, base + b[1], x + b[2] - b[0], base + b[3]])
        x += b[2] - b[0]
        nxt = string[i + 1] if i + 1 < len(string) else None
        if nxt is not None and nxt != " ":
            x += gap(i) * em
    right = int(x) + 60
    image = image.crop((0, 0, right, 260))
    if shear:
        image = image.transform(image.size, Image.AFFINE, (1, shear, -shear * 130, 0, 1, 0), fillcolor=255)
    if squeeze != 1.0:
        image = image.resize((int(image.width * squeeze), image.height), Image.LANCZOS)
        boxes = [[b[0] * squeeze, b[1], b[2] * squeeze, b[3]] for b in boxes]
        em = em
    image.convert("RGB").save(OUT / f"{name}.png")
    CASES.append({"file": f"{name}.png", "text": string, "boxes": [[round(v, 2) for v in b] for b in boxes], "em": round(em, 3)})


def whole(name, f, string, shear=0.0):
    """`string` drawn as the face sets it; `shear` leans it (tan of the angle, top to the right)."""
    l, t, r, b = f.getbbox(string)
    image = Image.new("L", (r - l + 200, 260), 255)
    ImageDraw.Draw(image).text((100 - l, 60), string, font=f, fill=0)
    if shear:
        image = image.transform(image.size, Image.AFFINE, (1, shear, -shear * 130, 0, 1, 0), fillcolor=255)
    image.convert("RGB").save(OUT / f"{name}.png")
    CASES.append({"file": f"{name}.png", "text": string, "boxes": [], "em": round(em_of(f), 3)})


def hershey(name, string, width):
    """`string` in Hershey Sans 1-stroke (included, single-line) drawn as strokes `width` px wide
    with round ends, its cap height 100 px."""
    import re

    svg = (Path(__file__).resolve().parents[4] / "frontend/public/fonts/hershey-sans-1/HersheySans1.svg").read_text()
    glyphs = {}
    for g in re.finditer(r'<glyph unicode="(.)"[^>]*?horiz-adv-x="([0-9.]+)"(?:[^>]*?d="([^"]*)")?', svg):
        glyphs[g.group(1)] = (float(g.group(2)), g.group(3) or "")
    scale = 100 / 662
    image = Image.new("L", (int(sum(glyphs[c][0] for c in string) * scale) + 200, 260), 255)
    draw = ImageDraw.Draw(image)
    x, base = 100.0, 180
    for c in string:
        advance, d = glyphs[c]
        for stroke in d.split("M")[1:]:
            pts = [tuple(float(v) for v in p.split()) for p in stroke.split("L")]
            xy = [(x + px * scale, base - py * scale) for px, py in pts]
            if len(xy) > 1:
                draw.line(xy, fill=0, width=width, joint="curve")
            for px, py in xy:
                r = width / 2
                draw.ellipse((px - r, py - r, px + r, py + r), fill=0)
        x += advance * scale
    image.convert("RGB").save(OUT / f"{name}.png")
    CASES.append({"file": f"{name}.png", "text": string, "boxes": [], "em": round(100 / CAP, 3)})


def main():
    OUT.mkdir(exist_ok=True)
    mono = face(MONO)
    # Weight (thin, regular, black) and lean (3, 8 and 14 degrees) of the fallback face.
    whole("weight_thin", face(ADWAITA, b"Thin"), "Hamburg 42")
    whole("weight_regular", face(ADWAITA, b"Regular"), "Hamburg 42")
    whole("weight_black", face(ADWAITA, b"Black"), "Hamburg 42")
    for deg, tan in [(3, 0.0524), (8, 0.1405), (14, 0.2493)]:
        whole(f"slant_{deg}", face(ADWAITA, b"Regular"), "Hamburg 42", shear=tan)
    # Single-line strokes around the bound of 0.07 cap height (7 px here): 3, 6 and 9 px wide.
    for w in (3, 6, 9):
        hershey(f"stroke_{w}", "Hamburg 42", w)
    # Letter spacing: a line squeezed to half its width, and one tracked 0.6 em apart.
    placed("tight", mono, "Hamburger", lambda i: 0.12, squeeze=0.5)
    placed("loose", mono, "Hamburger", lambda i: 0.6)
    # Spaces the reader missed: 0.6 em after "Search" (space), 0.24 em (no space), letters 0.25 em
    # apart with a 0.4 em gap (under twice the median: no space), and the lone-letter guard.
    placed("space_yes", mono, "Searchfonts", lambda i: 0.6 if i == 5 else 0.08)
    placed("space_short", mono, "Searchfonts", lambda i: 0.24 if i == 5 else 0.08)
    placed("space_median", mono, "Searchfonts", lambda i: 0.4 if i == 5 else 0.25)
    placed("guard_letter", mono, "Teilb", lambda i: 0.6 if i == 3 else 0.08)
    placed("guard_digit", mono, "Teil3", lambda i: 0.6 if i == 3 else 0.08)
    placed("guard_amp", mono, "Rock&Roll", lambda i: 0.6 if i in (3, 4) else 0.08)
    placed("guard_plus", mono, "Plus+Minus", lambda i: 0.6 if i in (3, 4) else 0.08)
    placed("guard_minus", mono, "Nord-Sud", lambda i: 0.6 if i in (3, 4) else 0.08)
    # A chip row read with its spaces: 2 em between words (cut), 1.3 em (under 1.5 em: one text),
    # and 1.6 em with letters 0.45 em apart (under four times the median: one text).
    placed("split_yes", mono, "Bold Italic Light", lambda i: 0.08, extra={4: 2.0, 11: 2.0})
    placed("split_short", mono, "Bold Italic Light", lambda i: 0.08, extra={4: 1.3, 11: 1.3})
    placed("split_median", mono, "Bold Italic Light", lambda i: 0.45, extra={4: 1.6, 11: 1.6})
    (OUT / "r3.json").write_text(json.dumps(CASES, indent=1) + "\n")


main()
