#!/bin/sh
# Makes ../inkscape_shapes.svg (Inkscape 1.4.4 save) and ../inkscape_shapes.paths.svg (Inkscape's
# Object to Path of it) from seed_inkscape_shapes.svg.txt, which holds only the tool numbers.
# Inkscape computes every `d` itself: each object is moved by 0,0 (which makes Inkscape rewrite
# the object), some are rotated or flipped by Inkscape. TMPDIR must be a scratch directory.
set -e
here="$(cd "$(dirname "$0")" && pwd)"
out="$(cd "$here/.." && pwd)"
tmp="${TMPDIR:?set TMPDIR to a scratch dir}"
cp "$here/seed_inkscape_shapes.svg.txt" "$tmp/inkscape_shapes.svg"
IDS="a_slice a_arc a_chord a_big a_neg a_whole a_full a_ell a_rot30 a_mirror a_label r_ell p3 p5 p6 p6_rot s5 s_round s_rand s_twist s_rot s_flip sp1 sp_skew a_skew a_stretch p_stretch s_in_rot box1"
A=""
for i in $IDS; do A="$A;select-by-id:$i;transform-translate:0,0;select-clear"; done
B=""
B="$B;select-by-id:a_rot30;transform-rotate:30;select-clear"
B="$B;select-by-id:a_mirror;object-flip-horizontal;select-clear"
B="$B;select-by-id:p6_rot;transform-rotate:15;select-clear"
B="$B;select-by-id:s_rot;transform-rotate:20;select-clear"
B="$B;select-by-id:s_flip;object-flip-horizontal;select-clear"
# Pass 1 writes every object in Inkscape's own form (d, sodipodi:open, r2 and arg2 added); pass 2
# reads that file back as any later session would and moves, rotates and flips, so the saved `d`
# is the one Inkscape draws (a chord written by pass 1 alone has no closing Z yet).
inkscape "$tmp/inkscape_shapes.svg" --actions="${A#;};export-plain-svg:false;export-filename:$tmp/pass1.svg;export-type:svg;export-do" 2>/dev/null
inkscape "$tmp/pass1.svg" --actions="${A#;}$B;export-plain-svg:false;export-filename:$out/inkscape_shapes.svg;export-type:svg;export-do" 2>/dev/null
sed -i 's|sodipodi:docname="[^"]*"|sodipodi:docname="inkscape_shapes.svg"|' "$out/inkscape_shapes.svg"
# Object to Path of every shape (the box's faces through their group).
P=""
for i in $IDS f1 f2 f3 f4 f5 f6; do P="$P;select-by-id:$i;object-to-path;select-clear"; done
cp "$out/inkscape_shapes.svg" "$tmp/inkscape_shapes_in.svg"
inkscape "$tmp/inkscape_shapes_in.svg" --actions="${P#;};export-plain-svg:false;export-filename:$out/inkscape_shapes.paths.svg;export-type:svg;export-do" 2>/dev/null
sed -i 's|sodipodi:docname="[^"]*"|sodipodi:docname="inkscape_shapes.paths.svg"|' "$out/inkscape_shapes.paths.svg"
