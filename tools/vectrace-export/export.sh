#!/usr/bin/env bash
# Exports the vectrace crate with its history into a standalone repository directory.
# Usage: tools/vectrace-export/export.sh <output-dir>   (must not exist; run from anywhere)
set -euo pipefail

out=${1:?usage: export.sh <output-dir>}
repo=$(git rev-parse --show-toplevel)
kit="$repo/tools/vectrace-export"
[ ! -e "$out" ] || { echo "$out exists" >&2; exit 1; }

cd "$repo"
git subtree split -P vectrace -b vectrace-export >/dev/null 2>&1 || { echo "git subtree split failed" >&2; exit 1; }
git clone -q --branch vectrace-export --single-branch "$repo" "$out"
git -C "$out" remote remove origin
git -C "$out" branch -m main

# Overlay: the template files replace the crate's workspace-bound ones.
cp -r "$kit/template/." "$out/"
cp "$repo/rust-toolchain.toml" "$out/rust-toolchain.toml"
mkdir -p "$out/assets/models"
cp -r "$repo/tests/assets/trace-references" "$out/assets/trace-references"
cp -r "$repo/frontend/public/models/paddleocr-latin" "$out/assets/models/paddleocr-latin"
git -C "$out" add -A
git -C "$out" commit -q -m "chore: standalone repository files

Cargo.toml without the workspace, rust-toolchain.toml, deny.toml, CI, release, README,
ASSETS.md and the assets (reference images, OCR weights)."
echo "exported to $out (history of vectrace/ plus one overlay commit; branch vectrace-export kept here)"
