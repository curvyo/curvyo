#!/bin/sh
# Copies the slice-2 text and image fixtures into corpus/read_svg/ (criterion 109) and the
# workspace Cargo.lock beside this file.
set -eu
here=$(dirname "$0")
fixtures="$here/../../curvyo-ui-core/tests/fixtures/svg"
cp "$here/../../Cargo.lock" "$here/Cargo.lock"
mkdir -p "$here/corpus/read_svg"
for f in "$fixtures"/text_*.svg "$fixtures"/images*.svg; do
    case "$f" in *.paths.svg) continue ;; esac
    cp "$f" "$here/corpus/read_svg/"
done
