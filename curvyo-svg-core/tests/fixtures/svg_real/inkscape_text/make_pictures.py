"""Writes alpha.png (200 x 100 RGBA, alpha ramp left to right) and photo.jpg (120 x 80 RGB,
baseline JPEG, quality 85) into the directory given; deterministic pixels."""
import sys
from PIL import Image
out = sys.argv[1]
a = Image.new('RGBA', (200, 100))
a.putdata([(x, y * 2, 128, 255 - x) for y in range(100) for x in range(200)])
a.save(f'{out}/alpha.png')
p = Image.new('RGB', (120, 80))
p.putdata([((x * 2) % 256, (y * 3) % 256, (x + y) % 256) for y in range(80) for x in range(120)])
p.save(f'{out}/photo.jpg', quality=85)
