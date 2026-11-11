"""Renders the tester fixtures of 0091 slice 3 (`acceptance_0091_slice3_tester.rs`).

Run from the repository root: `python3 -I vectrace/tests/fixtures/tester/make_turned.py`
(needs Pillow, and `make_posters.py` and `make_colour.py` run before, for `poster.png` and
`colour_poster.png`). Faces: Source Sans 3 (OFL, included in Curvyo).

Turns follow Pillow: a positive angle is counter-clockwise on the screen, so a line turned by 90
reads from the bottom to the top. In the document's y-down matrix that is `atan2(b, a) = -angle`.
- `poster_180.png`, `colour_poster_180.png`: the slice 1 and slice 2 posters turned by 180.
- `steep.png`: one line turned by 90 (bottom to top) and one by -90 (top to bottom).
- `mixed.png`: an upright line above an upside-down line.
- `turned_195.png`: one line turned by 195, i.e. 180 plus 15.
- `tight_180.png`: three upside-down lines with marks, set close together.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
SANS = ROOT / "frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"


def strip(string, size=96):
    face = ImageFont.truetype(str(SANS), size)
    left, top, right, bottom = face.getbbox(string)
    mask = Image.new("L", (right + 40, bottom + 40), 0)
    ImageDraw.Draw(mask).text((20, 20), string, font=face, fill=255)
    return mask


def put(image, string, degrees, centre):
    turned = strip(string).rotate(degrees, resample=Image.BICUBIC, expand=True)
    x = centre[0] - turned.width // 2
    y = centre[1] - turned.height // 2
    image.paste((0, 0, 0), (x, y), turned)


for name in ("poster", "colour_poster"):
    Image.open(OUT / f"{name}.png").rotate(180).save(OUT / f"{name}_180.png")

steep = Image.new("RGB", (1400, 1400), "white")
put(steep, "Hochkant hinauf 90", 90, (350, 700))
put(steep, "Hochkant hinab 90", -90, (1050, 700))
steep.save(OUT / "steep.png")

mixed = Image.new("RGB", (1400, 700), "white")
put(mixed, "Oben aufrecht 11", 0, (700, 180))
put(mixed, "Unten kopfüber 22", 180, (700, 500))
mixed.save(OUT / "mixed.png")

turned = Image.new("RGB", (1400, 1400), "white")
put(turned, "Winkel 195 Grad", 195, (700, 700))
turned.save(OUT / "turned_195.png")

# Three tight lines, upside down, for the room a turned line grows by above its text: the lines
# of the middle and the top carry marks (Ä, Ö) that sit under the line above them once turned.
tight = Image.new("RGB", (1400, 700), "white")
for i, string in enumerate(["Äpfel Öl Ärger", "Ölfässer Übung", "Ärmel Öfen Äste"]):
    put(tight, string, 180, (700, 140 + i * 130))
tight.save(OUT / "tight_180.png")
