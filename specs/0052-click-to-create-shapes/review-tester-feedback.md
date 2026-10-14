# Tester check: Shift/Ctrl tap toggles (criterion 37, 2026-10-16)

Verdict: PASS. `curvyo-editor-wasm/tests/acceptance_0052_toggle.rs`, 10 black-box tests through
`Session`: tap updates preview and chip without a pointer move, auto-repeat ignored, Shift+letter
and Ctrl+letter take the tap back, a later plain key keeps it, no toggle after a field edit
(checks gone), check click and key flip the same state, Shift does nothing for the polygon, new
placement starts off, held key XOR toggle, plain create-drag unchanged. Full
`curvyo-editor-wasm` run: 3131 passed. Not covered: the real browser key events (WebKitGTK
without Shift key-up); the tests drive `placement_key_down` directly.
