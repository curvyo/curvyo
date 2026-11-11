"""Renders the fixtures of 0091 slice 3 (upside-down and steep text, criteria 30, 31 and 33).

Run from the repository root: `python3 -I vectrace/tests/fixtures/turned/make_turned.py`.
The 180 degree poster is the reference poster (`../poster.png`) turned exactly; the steep and the
mixed lines are drawn as the tester's `rotated()` in `../tester/make_posters.py` does, in Source
Sans 3 (OFL, included in Curvyo). The truth of every line is in `truth.toml` next to this script.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
SANS = ROOT / "frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"


def turned_line(string, degrees, size=96):
    """The line as a white-on-black mask turned by `degrees` (counter-clockwise on screen)."""
    face = ImageFont.truetype(str(SANS), size)
    _, _, right, bottom = face.getbbox(string)
    strip = Image.new("L", (right + 40, bottom + 40), 0)
    ImageDraw.Draw(strip).text((20, 20), string, font=face, fill=255)
    return strip.rotate(degrees, resample=Image.BICUBIC, expand=True)


def poster_180():
    Image.open(OUT.parent / "poster.png").transpose(Image.Transpose.ROTATE_180).save(
        OUT / "poster_180.png"
    )


def steep():
    image = Image.new("RGB", (1400, 1400), "white")
    up = turned_line("Steil nach oben", 90)
    down = turned_line("Steil nach unten", -90)
    image.paste((0, 0, 0), (350 - up.width // 2, (1400 - up.height) // 2), up)
    image.paste((0, 0, 0), (1050 - down.width // 2, (1400 - down.height) // 2), down)
    image.save(OUT / "steep.png")


def mixed():
    image = Image.new("RGB", (1400, 900), "white")
    upright = turned_line("Oben richtig", 0)
    upside = turned_line("Kopfstand 180", 180)
    image.paste((0, 0, 0), ((1400 - upright.width) // 2, 150), upright)
    image.paste((0, 0, 0), ((1400 - upside.width) // 2, 550), upside)
    image.save(OUT / "mixed.png")


poster_180()
steep()
mixed()
