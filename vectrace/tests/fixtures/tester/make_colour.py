"""Renders the tester posters of 0091 slice 2, multicolour (`acceptance_0091_slice2_tester.rs`).

Run from the repository root: `python3 -I vectrace/tests/fixtures/tester/make_colour.py`.
Faces: Source Sans 3 and Source Serif 4 (OFL, included in Curvyo). The colour, truth and ink box
of every line go into `colour_boxes.toml` next to this script.

`colour_poster.png` (2000 x 1700, white paper):
- red text on white ("Größe") beside a red disc, so red is a colour of a shape too;
- green text on white ("Übermaß"): a colour only the text has;
- black text on white with umlauts and ß ("Äpfel & Öl", "Brücke über Täler");
- white text on a blue box ("Straße 7"): the box stays whole, the text is the paper's colour;
- blue text on white ("Süße Grüße"), the box's colour.

`grey_text.png` (1400 x 500): light grey text on white, a colour near the paper's.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
FONTS = ROOT / "frontend/public/fonts"
SANS = FONTS / "source-sans-3/SourceSans3-Regular.otf"
SANS_BOLD = FONTS / "source-sans-3/SourceSans3-Bold.otf"
SERIF = FONTS / "source-serif-4/SourceSerif4-Regular.ttf"

RED = (200, 30, 30)
GREEN = (30, 150, 60)
BLUE = (30, 60, 200)
BLACK = (0, 0, 0)
WHITE = (255, 255, 255)
GREY = (190, 190, 190)

LINES = []


def text(draw, xy, string, face, size, fill):
    font = ImageFont.truetype(str(face), size)
    draw.text(xy, string, font=font, fill=fill)
    LINES.append((string, fill, draw.textbbox(xy, string, font=font)))


def colour_poster():
    image = Image.new("RGB", (2000, 1700), WHITE)
    draw = ImageDraw.Draw(image)
    text(draw, (120, 80), "Größe", SANS_BOLD, 200, RED)
    text(draw, (120, 380), "Übermaß", SERIF, 140, GREEN)
    text(draw, (120, 620), "Äpfel & Öl", SANS, 110, BLACK)
    # White text on a coloured box (113 A, criterion 23).
    draw.rectangle((110, 830, 1210, 1090), fill=BLUE)
    text(draw, (170, 870), "Straße 7", SANS_BOLD, 140, WHITE)
    text(draw, (120, 1200), "Süße Grüße", SANS_BOLD, 110, BLUE)
    text(draw, (120, 1440), "Brücke über Täler", SANS, 90, BLACK)
    # A red shape with no text.
    draw.ellipse((1500, 1150, 1880, 1530), fill=RED)
    image.save(OUT / "colour_poster.png")


def grey_text():
    image = Image.new("RGB", (1400, 500), WHITE)
    draw = ImageDraw.Draw(image)
    text(draw, (100, 120), "Hellgrau", SANS_BOLD, 180, GREY)
    draw.rectangle((1000, 120, 1300, 380), fill=BLACK)
    image.save(OUT / "grey_text.png")


colour_poster()
grey_text()
with open(OUT / "colour_boxes.toml", "w", encoding="utf-8") as out:
    out.write("# Each line of the slice 2 posters (`make_colour.py`): file, text, colour, ink box in px.\n")
    files = ["colour_poster.png"] * 6 + ["grey_text.png"]
    for file, (string, fill, box) in zip(files, LINES):
        out.write(
            f'\n[[lines]]\nfile = "{file}"\ntext = "{string}"\ncolour = {list(fill)}\nbox = {list(box)}\n'
        )
