"""Renders the tester posters of 0091 slice 1 (`acceptance_0091_tester.rs`).

Run from the repository root: `python3 -I curvyo-ocr-core/tests/fixtures/tester/make_posters.py`.
Faces: Source Sans 3 and Source Serif 4 (OFL, included in Curvyo) and Liberation Mono Bold
(OFL, not included, for the nearest-font case of criterion 4; taken from the system fonts).
The truth of every line is in `truth.toml` next to this script.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
FONTS = ROOT / "frontend/public/fonts"
SANS = FONTS / "source-sans-3/SourceSans3-Regular.otf"
SANS_BOLD = FONTS / "source-sans-3/SourceSans3-Bold.otf"
SERIF = FONTS / "source-serif-4/SourceSerif4-Regular.ttf"
MONO = Path("/usr/share/fonts/liberation/LiberationMono-Bold.ttf")


BOXES = []


def text(draw, xy, string, face, size, fill=(0, 0, 0)):
    font = ImageFont.truetype(str(face), size)
    draw.text(xy, string, font=font, fill=fill)
    BOXES.append((string, draw.textbbox(xy, string, font=font)))


def poster():
    BOXES.clear()
    image = Image.new("RGB", (2000, 2000), "white")
    draw = ImageDraw.Draw(image)
    text(draw, (120, 80), "Größe", SANS_BOLD, 200)
    text(draw, (120, 380), "Übermaß", SERIF, 140)
    text(draw, (120, 620), "Äpfel & Öl", SANS, 110)
    text(draw, (120, 820), "Tel. 0421 98765", SANS, 72)
    text(draw, (120, 960), "Fräsen ab 3 Stück, 12.50 EUR", SERIF, 44)
    # Text on a filled shape (113 A): a black box with white letters.
    draw.rectangle((110, 1130, 1210, 1390), fill=(0, 0, 0))
    text(draw, (170, 1170), "Straße 7", SANS_BOLD, 140, fill=(255, 255, 255))
    # A face Curvyo does not include (114 B).
    text(draw, (120, 1560), "Mono 314 Zoll", MONO, 90)
    # Lowercase ä and ü in an included face that becomes text (criterion 2).
    text(draw, (120, 1760), "Brücke über Täler", SANS, 90)
    # A plain shape with no text beside the lines.
    draw.ellipse((1500, 1450, 1880, 1830), fill=(0, 0, 0))
    image.save(OUT / "poster.png")
    # The ink box of each line in source pixels (x0, y0, x1, y1), umlaut dots included.
    with open(OUT / "boxes.toml", "w", encoding="utf-8") as out:
        out.write("# The ink box of each line of poster.png in px (`make_posters.py`).\n")
        for string, box in BOXES:
            out.write(f'\n[[lines]]\ntext = "{string}"\nbox = {list(box)}\n')


def rotated(degrees, string, name):
    face = ImageFont.truetype(str(SANS), 96)
    left, top, right, bottom = face.getbbox(string)
    strip = Image.new("L", (right + 40, bottom + 40), 0)
    ImageDraw.Draw(strip).text((20, 20), string, font=face, fill=255)
    turned = strip.rotate(degrees, resample=Image.BICUBIC, expand=True)
    image = Image.new("RGB", (1400, 1400), "white")
    x = (1400 - turned.width) // 2
    y = (1400 - turned.height) // 2
    image.paste((0, 0, 0), (x, y), turned)
    image.save(OUT / name)


def clean():
    image = Image.new("RGB", (1200, 500), "white")
    text(ImageDraw.Draw(image), (100, 100), "Größe", SANS_BOLD, 200)
    image.save(OUT / "clean.png")


def plain():
    image = Image.new("RGB", (800, 600), "white")
    draw = ImageDraw.Draw(image)
    draw.ellipse((100, 100, 400, 400), fill=(0, 0, 0))
    draw.polygon([(500, 500), (700, 120), (760, 520)], fill=(0, 0, 0))
    image.save(OUT / "plain.png")


poster()
rotated(7, "Winkel sieben 7", "rotated_7.png")
rotated(30, "Winkel 30 Grad", "rotated_30.png")
rotated(90, "Neunzig 90", "rotated_90.png")
clean()
plain()
