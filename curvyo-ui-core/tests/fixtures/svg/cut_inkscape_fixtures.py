"""Cuts the real Inkscape 1.4.4 files of the 0024 slice 2 tester into the golden fixtures.

text_ink.svg and text_ink.paths.svg (curvyo-svg-core/tests/fixtures/svg_real/inkscape_text/,
made by make_slice2.sh) hold every text of criterion 107. This script keeps whole top-level
children of their layers, unchanged byte for byte, and writes:

  text_inkscape.svg / .paths.svg   layer "Text" without the flowed texts t_inline and t_shape
  text_frame.svg / .paths.svg      layer "Text" with only t_inline and t_shape
  text_on_path.svg / .paths.svg    layer "Paths" (carriers and texts on them)

and copies images_ink.svg to images.svg. Run from anywhere: python3 -I cut_inkscape_fixtures.py
"""
import os
import re
import shutil

HERE = os.path.dirname(os.path.abspath(__file__))
REAL = os.path.join(HERE, '../../../../curvyo-svg-core/tests/fixtures/svg_real/inkscape_text')
FRAMES = {'t_inline', 't_shape'}


def parse(lines):
    """Head lines, layers as (opening lines, [(name, block lines)], closing line), tail lines."""
    head, layers, tail = [], [], []
    i = 0
    while not lines[i].startswith('  <g'):
        head.append(lines[i])
        i += 1
    while i < len(lines) and lines[i].startswith('  <g'):
        opening = []
        while not lines[i].startswith('    <'):
            opening.append(lines[i])
            i += 1
        children = []
        while not lines[i].startswith('  </g>'):
            block = [lines[i]]
            i += 1
            while not (lines[i].startswith('    <') and not lines[i].startswith('    </')) \
                    and not lines[i].startswith('  </g>'):
                block.append(lines[i])
                i += 1
            text = ''.join(block)
            m = re.search(r'inkscape:label="([^"]*)"', text) or re.search(r'\bid="([^"]*)"', text)
            children.append((m.group(1), block))
        layers.append((opening, children, lines[i]))
        i += 1
    tail = lines[i:]
    return head, layers, tail


def label(opening):
    return re.search(r'inkscape:label="([^"]*)"', ''.join(opening)).group(1)


def cut(source, target, layer_name, keep):
    with open(source, encoding='utf-8') as f:
        lines = f.readlines()
    head, layers, tail = parse(lines)
    out = list(head)
    for opening, children, closing in layers:
        if label(opening) != layer_name:
            continue
        out.extend(opening)
        for name, block in children:
            if keep(name):
                out.extend(block)
        out.append(closing)
    out.extend(tail)
    with open(target, 'w', encoding='utf-8') as f:
        f.writelines(out)


for suffix in ('.svg', '.paths.svg'):
    source = os.path.join(REAL, 'text_ink' + suffix)
    cut(source, os.path.join(HERE, 'text_inkscape' + suffix), 'Text', lambda n: n not in FRAMES)
    cut(source, os.path.join(HERE, 'text_frame' + suffix), 'Text', lambda n: n in FRAMES)
    cut(source, os.path.join(HERE, 'text_on_path' + suffix), 'Paths', lambda n: True)
shutil.copyfile(os.path.join(REAL, 'images_ink.svg'), os.path.join(HERE, 'images.svg'))
