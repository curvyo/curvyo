#!/bin/sh
# Makes the Inkscape 1.4.4 files of the 0024 slice 2 tester (see ../SOURCES.md):
#   text_ink.svg / text_ink.paths.svg, flowroot_ink.svg / flowroot_ink.paths.svg,
#   images_ink.svg (+ linked.png beside it is NOT kept: the link must stay unresolved).
# Fonts: only frontend/public/fonts through an isolated fontconfig, so Inkscape lays out with the
# same faces Curvyo ships. TMPDIR must be a scratch directory.
set -e
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../../.." && pwd)"
tmp="${TMPDIR:?set TMPDIR to a scratch dir}"
mkdir -p "$tmp/fc-cache"
cat > "$tmp/fonts.conf" <<CONF
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig><dir>$root/frontend/public/fonts</dir><cachedir>$tmp/fc-cache</cachedir></fontconfig>
CONF
export FONTCONFIG_FILE="$tmp/fonts.conf"
cd "$here"
TEXTS="t_one t_multi t_right t_bold t_italic t_light t_bolditalic t_semibold t_serif t_spacing t_kerns t_mixed t_super t_vertical t_rot30 t_stretch t_mirror t_missing t_inline t_shape tp_open tp_circle tp_rect tp_side tp_moved"
# Save: Inkscape re-lays out every text it is asked to move (a zero move writes its own x/y).
A=""
for t in $TEXTS; do A="$A;select-by-id:$t;transform-translate:0,0;select-clear"; done
cp seed_text.svg.txt "$tmp/seed_text.svg"
inkscape "$tmp/seed_text.svg" --actions="${A#;};export-plain-svg:false;export-filename:$here/text_ink.svg;export-type:svg;export-do" 2>/dev/null
P=""
for t in $TEXTS; do P="$P;select-by-id:$t;object-to-path;select-clear"; done
inkscape "$here/text_ink.svg" --actions="${P#;};export-plain-svg:false;export-filename:$here/text_ink.paths.svg;export-type:svg;export-do" 2>/dev/null
cp seed_flowroot.svg.txt "$tmp/seed_flowroot.svg"
inkscape "$tmp/seed_flowroot.svg" --actions="select-by-id:flow1;transform-translate:0,0;select-clear;select-by-id:flow2;transform-translate:0,0;select-clear;export-plain-svg:false;export-filename:$here/flowroot_ink.svg;export-type:svg;export-do" 2>/dev/null
inkscape "$here/flowroot_ink.svg" --actions="select-by-id:flow1;object-to-path;select-clear;select-by-id:flow2;object-to-path;select-clear;export-plain-svg:false;export-filename:$here/flowroot_ink.paths.svg;export-type:svg;export-do" 2>/dev/null
# Images: pictures made by make_pictures.py, embedded into the seed, saved by Inkscape.
python3 -I make_pictures.py "$tmp"
python3 -I - "$tmp" <<'PY'
import base64, sys
tmp = sys.argv[1]
s = open('seed_images.svg.txt').read()
for key, name in (('@PNG@', 'alpha.png'), ('@JPEG@', 'photo.jpg')):
    s = s.replace(key, base64.b64encode(open(f'{tmp}/{name}', 'rb').read()).decode())
open(f'{tmp}/seed_images.svg', 'w').write(s)
PY
inkscape "$tmp/seed_images.svg" --actions="select-by-id:img_rot;transform-rotate:30;select-clear;select-by-id:img_png;transform-translate:0,0;select-clear;export-plain-svg:false;export-filename:$here/images_ink.svg;export-type:svg;export-do" 2>/dev/null
# Inkscape rewrites the link relative to where it saves; name it plainly (the file never exists).
sed -i 's|xlink:href="[^"]*linked.png"|xlink:href="linked.png"|' "$here/images_ink.svg"
