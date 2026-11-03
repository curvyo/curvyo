#!/bin/sh
# Spike 0064: runs the echo server and the Tauri probe under Xvfb, then screenshots the window.
# usage: run-webview.sh <label>   (from specs/0064-machine-connector/spike, after both builds)
set -u
OUT=${OUT:-.}
label=$1
timeout 20 ./target/release/spike-0064-connector echo 0.0.0.0:7979 > "$OUT/echo-$label.log" 2>&1 &
Xvfb :97 -screen 0 800x600x24 > /dev/null 2>&1 &
xvfb=$!
sleep 1
DISPLAY=:97 timeout 12 /home/marc/curvyo/target/debug/spike-0064-webview > "$OUT/app-$label.log" 2>&1 &
sleep 8
import -display :97 -window root "$OUT/shot-$label.png"
wait %3 2>/dev/null
kill $xvfb
wait
cat "$OUT/echo-$label.log"
