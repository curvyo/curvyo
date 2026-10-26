#!/bin/sh
set -e
cd "$(dirname "$0")" && cp seed_mm_a4.svg.txt "${TMPDIR:-/tmp}/seed.svg"
A="select-by-id:s1;transform-translate:0,0;select-clear;select-by-id:s2;transform-translate:0,0;select-clear"
A="$A;select-by-id:p1;clone;object-set-attribute:inkscape:label,clone_moved;transform-translate:0,40;select-clear"
A="$A;select-by-id:p1;clone;object-set-attribute:inkscape:label,clone_rot30;transform-rotate:30;transform-translate:0,70;select-clear"
A="$A;select-by-id:p1;clone;object-set-attribute:inkscape:label,clone_flip;object-flip-horizontal;transform-translate:0,100;select-clear"
A="$A;select-by-id:p1;clone;object-set-attribute:inkscape:label,clone_scaled;transform-scale:2;transform-translate:0,130;select-clear"
A="$A;select-by-id:r1;clone;object-set-attribute:inkscape:label,clone_of_rect;transform-translate:0,170;select-clear"
A="$A;select-by-id:r4;transform-rotate:30;select-clear"
A="$A;select-by-id:d1,d2;object-to-path;path-combine;object-set-attribute:inkscape:label,donut;object-set-property:fill-rule,evenodd;select-clear"
A="$A;select-by-id:x1,x2;object-to-path;path-combine;object-set-attribute:inkscape:label,overlap;object-set-property:fill-rule,evenodd;select-clear"
A="$A;select-by-id:t1;object-to-path;select-clear"
A="$A;select-by-id:g1;transform-rotate:-20;select-clear"
A="$A;select-by-id:p2;clone;object-set-attribute:inkscape:label,clone_in_rot_group;transform-translate:-200,100;select-clear"
A="$A;export-plain-svg:false;export-filename:inkscape_mm_a4.svg;export-type:svg;export-do"
inkscape "${TMPDIR:-/tmp}/seed.svg" --actions="$A" 2>/dev/null
