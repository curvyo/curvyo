"""Writes <svg>.inkscape-bbox.csv: Inkscape 1.4.4's bounds of every element that is drawn as one
leaf (a shape, or a use of one), outside defs/symbol/clipPath/mask/pattern/marker/switch/text and
without clip-path, mask or filter on itself or an ancestor; measured on a scratch copy in which
every element has stroke:none (so the bounds are geometric), in Inkscape px."""
import subprocess, sys, os, tempfile, xml.etree.ElementTree as ET
src, out = sys.argv[1], sys.argv[2]
SVG = '{http://www.w3.org/2000/svg}'
XL = '{http://www.w3.org/1999/xlink}href'
tmpdir = tempfile.mkdtemp(dir=os.environ['TMPOUT'])
copy = os.path.join(tmpdir, 'nostroke.svg')
subprocess.run(['inkscape', src, '--actions',
  'select-by-selector:*;object-set-property:stroke,none;select-clear;export-plain-svg:false;export-filename:%s;export-type:svg;export-do' % copy],
  check=True, capture_output=True)
q = subprocess.run(['inkscape', '--query-all', copy], check=True, capture_output=True, text=True).stdout
tree = ET.parse(copy)
root = tree.getroot()
parent = {c: p for p in root.iter() for c in p}
by_id = {e.get('id'): e for e in root.iter() if e.get('id')}
SHAPES = {'path', 'rect', 'circle', 'ellipse', 'line', 'polyline', 'polygon'}
HIDDEN = {'defs', 'symbol', 'clipPath', 'mask', 'pattern', 'marker', 'switch', 'text'}
def tag(e):
    return e.tag[len(SVG):] if e.tag.startswith(SVG) else None
def clipped(e):
    while e is not None:
        st = e.get('style') or ''
        if e.get('clip-path') or e.get('mask') or e.get('filter') or 'clip-path' in st or 'mask' in st or 'filter' in st:
            return True
        e = parent.get(e)
    return False
def leaf(e, depth=0):
    t = tag(e)
    if t is None or depth > 8 or clipped(e):
        return False
    if depth == 0:
        a = parent.get(e)
        while a is not None:
            if tag(a) in HIDDEN:
                return False
            a = parent.get(a)
    if t in SHAPES:
        return True
    if t == 'use':
        h = (e.get(XL) or e.get('href') or '').lstrip('#')
        return h in by_id and leaf(by_id[h], depth + 1)
    return False
with open(out, 'w') as f:
    f.write('# Inkscape 1.4.4 --query-all (geometric: stroke:none on a scratch copy); one line per single-leaf element; id,x,y,w,h in Inkscape px\n')
    for line in q.splitlines():
        i = line.split(',')[0]
        if i in by_id and leaf(by_id[i]):
            f.write(line + '\n')
