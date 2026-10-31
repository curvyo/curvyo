# Initial sketch-core architecture review

2026-10-30. Verdict: **APPROVE for document integration**, R2 corrected in source below.
Bounded source review only; no builds/tests rerun. This is not feature acceptance.
Full feature acceptance remains pending independent testing and integration.

## Closed finding

### R2 — Independently validate implicit entity invariants before publishing (closed)

`src/residuals.rs::validate` now invokes `intrinsic_geometry` before relation
checks. It verifies fixed ellipse minor radius, fixed arc radius and wrapped
start/end angles, then verifies both endpoints of every solved arc against
centre + radius × (cos(angle), sin(angle)). Named length/angle tolerances and
finite residual checks reject disagreement before the candidate is returned.

Three focused source tests inject inconsistent arc endpoints, changed fixed arc
radius and changed fixed ellipse radius. Implementer reports 21 core tests and
native/WASM Clippy passing; architect inspected the checks and tests without
rerunning them. R2 is closed at source-review level.

## Confirmed boundaries

- `Sketch`, `Entity`, `Constraint`, stable IDs and typed expression trees are
  independent of document storage and editor state. Solver input is an immutable
  seed; candidates are separate and failure does not replace caller state.
- Backend IDs map to stable user IDs; helper contact points remain derived seed
  state. Store `contact_seeds` in the fingerprinted checkpoint, not as independent
  editable constraint authority. Reconcile obsolete seeds when constraints are
  removed or changed; the codec must not serialize the whole runtime Sketch as
  one authoritative graph register.
- Formula name validation shares parser grammar, quantity deserialization now
  validates finiteness, references revalidate quantities, and evaluation/parser/
  dependency traversal use explicit stacks. These address the prior three
  formula source findings; test results remain implementer/tester evidence.
- Construction functions clone input and return a solved candidate; rejected
  construction does not partially mutate the original. Fillets retain original
  segment IDs as construction references and add trimmed geometry/relationships.
- Contour assembly recognizes explicit coincidence and extracts cycles before
  attached branches; construction geometry is excluded. Final transformed
  approximation tolerance remains document integration's responsibility.
- No speculative backend trait or general workbench framework is introduced.
  Production persistence, undo, clones and UI are still separate integration work.

## Clarified tangency contract (not a blocker)

An initial observation that generic helper contacts lacked arc-span validation
was withdrawn after the implementer relayed the lead's explicit contract:
**generic Tangent acts on analytic carriers; shared-endpoint slot/fillet tangency
and On constraints retain finite geometry semantics.** Do not add finite-span
rejection to generic tangency under that contract. Keep this distinction explicit
in the public relation documentation and valid-selection UX.

## Next review checkpoint

`core-api.md` correctly distinguishes graph records, derived contact seeds,
validated checkpoint outline input and nonmutating candidate results. No remaining
public API blocker was found for document integration. Apply ADR0026 per-record
codec/checkpoint rules; remove obsolete contact seeds with associated constraints.
Independent tester owns the current test slot. This approval does not establish
all acceptance criteria, numerical budgets, complete persistence semantics or
customer acceptance of the finished feature.
