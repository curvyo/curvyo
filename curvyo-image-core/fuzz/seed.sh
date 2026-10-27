#!/bin/sh
# Copies the decoder fixtures and the format-25 golden projects into seeds/ (git-ignored).
set -eu
here=$(dirname "$0")
images="$here/../tests/fixtures/images"
projects="$here/../../curvyo-document-core/tests/fixtures"
cp "$here/../../Cargo.lock" "$here/Cargo.lock"
mkdir -p "$here/seeds/png" "$here/seeds/jpeg" "$here/seeds/webp" "$here/seeds/project"
cp "$images"/*.png "$images"/*renamed* "$here/seeds/png/"
cp "$images"/*.jpg "$here/seeds/jpeg/"
cp "$images"/*.webp "$here/seeds/webp/"
cp "$projects"/image_*v25.curvyo "$here/seeds/project/"
