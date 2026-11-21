# ADRs for bed size from the controller

Architect, 2026-11-21. Covers criteria 1 to 6 of `specification.md` (Fragen 255 B and 256 A).
**No new ADR, no new crate, no new dependency, no protocol change, no `format_version` change.**
Everything below is a **(Proposal)** unless it quotes an ADR or a criterion. No customer question.

- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) §6 with
  [`specs/0064-machine-connector/adrs.md`](../0064-machine-connector/adrs.md) C6: the wire already
  carries the travel. `Welcome.controller.settings.travel: Option<[Millimetres; 2]>` (`$130`,
  `$131`) exists in `curvyo-connector-core/src/message.rs`, golden `welcome_grbl.json`, and
  `Welcome` is sent again when the controller is identified or lost. So the spec's "additive
  field in `Welcome.controller` or `Status`" is already done: nothing is added, `Status` stays as is.
- [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md) §1, §5: the copy's
  work area is the output page frame; "Differs from profile" is derived (`differs`, 0.01 mm).
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §4 (amended): the page is resized about the
  origin corner, never typed. Criterion 2's "page behaviour of `0093` criterion 3" is this rule
  (equal for a top-left origin). **For the PO:** cite ADR 0042 §4 in criterion 2.
- `specs/0127` G19, G2: `$$` is read at identify; the connector writes no setting (G5).

## Feature-local decisions

- 2026-11-21: **Source stays `-io/src/grbl/settings.rs::travel()`.** One fix there: drop a
  non-finite value (`"inf"`, `"nan"` parse as `f64`). `serde_json` writes those as `null`, which
  the app cannot decode into `[Millimetres; 2]`, so the whole `Welcome` would be `Malformed` and the
  Controller line lost. Finite values outside 1 to 10,000 mm (0, negative) still go on the wire:
  they are the controller's report and the Travel row shows them; the app decides the offer.
- 2026-11-21: **Units.** GRBL's `$130`/`$131` are always mm; `$13` changes reports only. No
  conversion, pinned by a test with `$13=1`. Missing `$130` or `$131`, or an unparsable one: no
  travel (as today), no offer.
- 2026-11-21: **The decision lives in `curvyo-laser-core`** beside `clamp_work_area`:
  `machine_size_offer(page: DocumentSize, reported: [f64; 2]) -> Option<DocumentSize>`, `Some`
  only when both sides are finite, within `MIN_WORK_MM..=MAX_WORK_MM`, and either side differs
  from `page` by more than `PRESET_MATCH_TOLERANCE` (0.01 mm). Pure, no connector types (as
  `send_button`, 0064 B2), so laser-core gains no edge and stays wasm32 and `forbid(unsafe_code)`.
- 2026-11-21: **Session API in `curvyo-editor-wasm`** (`session/manufacturing/machine.rs`):
  `machine_size_offer(x_mm, y_mm) -> Option<[f64; 2]>` for the active output page (derived on
  every read, no stored refusal: criterion 4) and `use_machine_size(x_mm, y_mm, now_ms) -> bool`,
  which re-checks the offer, writes copy and page in one step through `write_active_copy` with a
  new `ProfileChange::UseMachineSize` (commit label `use_machine_size` in `document-core`,
  additive, no format change), and then, only for a found user record, pushes the library write
  as `update_machine_profile` does (record's values kept, new `work_area`, no undo). Built-in or
  "Not in your profiles": copy only, "Differs from profile" follows from `differs`.
- 2026-11-21: **Frontend** passes `info.settings.travel` of its own connection (already in
  `connectorText.ts`) into the two calls and formats "Machine reports 400 x 850 mm" with the
  existing `millimetres` helper. One app, one connection, one offer (criterion 5); the browser
  build has no connector, so no line. `curvyo-app` is not touched.

## Tests needed

- laser-core table (criteria 1, 3): equal, 0.01 mm apart (no offer), 0.011 apart, each side
  alone, 1 and 10,000 (offer), 0.99, 10,000.01, 0, negative, NaN, infinity (none).
- `-io` unit (`settings.rs`): `$130=inf` gives no travel; `$13=1` leaves 400 x 850 as is.
- `-io` connector test: simulator `SimConfig` gains a travel (default 400 x 300, tests only);
  with 400 x 850 its `$$` reaches `Welcome.controller.settings.travel = [400.0, 850.0]`.
- Goldens: none new; `welcome_grbl.json` and every message golden decode and re-encode unchanged.
- editor-wasm session (criteria 2, 4): press gives page 400 x 850, one undo restores 400 x 300,
  one library write for a user profile and none for the built-in (state `differs`), offer
  absent after the press, back after a hand edit; content placement per ADR 0042 §4.
- One browser check (criteria 1, 2, 5) with the simulator; no `format_version` change.
