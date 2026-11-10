"""Renders the multicolour poster of 0091 slice 2 (`specs/0091-trace-text-recognition`
criteria 23 and 24), in the manner of `../tester/make_posters.py`.

Run from the repository root: `python3 -I vectrace/tests/fixtures/colour/make_colour_poster.py`
(needs Pillow). Faces: Source Sans 3 and Source Serif 4 (OFL, included in Curvyo). Flat colours,
anti-aliased letters. Lines top to bottom, as the tests list them:
"Rote Farbe", "OFFEN", "Laser 42", "Blaue Größe", "Grün auf Weiß". `colour_small.png` is its
top left part (red text on white, white text on a red box), small enough for every test run.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
FONTS = ROOT / "frontend/public/fonts"
SANS = FONTS / "source-sans-3/SourceSans3-Regular.otf"
SANS_BOLD = FONTS / "source-sans-3/SourceSans3-Bold.otf"
SERIF = FONTS / "source-serif-4/SourceSerif4-Regular.ttf"

RED = (208, 34, 42)
BLUE = (35, 78, 184)
GREEN = (31, 122, 58)
YELLOW = (250, 204, 21)
WHITE = (255, 255, 255)


def text(draw, xy, string, face, size, fill):
    draw.text(xy, string, font=ImageFont.truetype(str(face), size), fill=fill)


def poster():
    image = Image.new("RGB", (1600, 1200), WHITE)
    draw = ImageDraw.Draw(image)
    # Coloured text on white.
    text(draw, (100, 60), "Rote Farbe", SANS_BOLD, 130, RED)
    # White text on a coloured box (113 A: the box stays whole, the text white above it).
    draw.rectangle((90, 300, 760, 520), fill=RED)
    text(draw, (150, 320), "OFFEN", SANS_BOLD, 140, WHITE)
    # Coloured text on a coloured box.
    draw.rectangle((860, 300, 1500, 520), fill=BLUE)
    text(draw, (920, 330), "Laser 42", SANS_BOLD, 120, YELLOW)
    # More coloured text on white, umlauts and ß.
    text(draw, (100, 600), "Blaue Größe", SERIF, 120, BLUE)
    text(draw, (100, 820), "Grün auf Weiß", SANS, 110, GREEN)
    # Plain coloured shapes with no text.
    draw.ellipse((1150, 700, 1450, 1000), fill=GREEN)
    draw.rectangle((100, 1040, 700, 1140), fill=YELLOW)
    image.save(OUT / "colour_poster.png")
    image.crop((0, 0, 800, 560)).save(OUT / "colour_small.png")


poster()
