# ADR 0025: Sketch core and solver boundary

**Status:** Proposed (2026-10-30); pure core boundary reviewed and pinned ACS 0.1.19 provisionally selected after feasibility and production validation. Full story acceptance remains separate.

## Context

The customer authorized spec 0066, including ellipses, constraint-preserving
fillets, parameters and formulas. A solver is only one part of that contract.
The recorded ACS experiment and production tests establish bounded coverage and
numerical evidence; upstream capability claims alone are not acceptance evidence.

Options: embed numerical code in document-core; isolate a pure sketch core;
or put a foreign solver behind a platform adapter. Choose the isolated core:
solver iteration and expression evaluation need neither CRDT nor editor state.
A foreign adapter is not selected: its portability, licence and unsafe boundary
would need separate evidence. An own solver remains an alternative if reuse fails.

## Decision

**(Proposal)** Add `curvyo-sketch-core` with `#![forbid(unsafe_code)]`, typed
length/angle quantities and no document, UI, I/O, clock or threading dependency.
It owns stable entity references, canonical sketch data, typed expressions,
validation, numerical adaptation and explicit result/diagnostic types.
Document-core depends on it for storage mapping and resolved geometry; the
editor owns gestures, history transactions and preview lifetime. Avoid a cycle:
use existing dependency-free unit types where available, otherwise map units
at the boundary rather than importing document-core into sketch-core.

Use a concrete solver adapter, not a backend trait without a second used
implementation. External solver indices never become persisted identifiers.
Inputs include an explicit seed and optional drag objective; outputs include
candidate coordinates, residual validation and mapped original constraint IDs.
Separate redundancy, contradiction, under/full constraint and numerical failure.
A zero rank-derived DOF count does not prove constraint satisfaction.

ACS 0.1.19 is the provisionally selected concrete backend, following the recorded
feasibility checkpoint below and subsequent production validation. Keep its exact
version/features, licence tree and native/WASM evidence reviewable. Run the
accepted entity/constraint matrix, degenerate and conflicting
cases, branch-stable repeated drags, ellipse singularities and fillet updates.
Compare native/WASM coordinates within 1e-6 mm from identical inputs/seeds.
Measure thread CPU on the 100-entity workload against AC6's 16 ms budget,
including diagnosis and failed solves; report stage costs and machine/build.
Do not replace these tests with a small successful demonstration.

Expression evaluation precedes geometric solving. Resolve stable parameter IDs,
check length/angle/scalar types, evaluate an acyclic dependency graph and reject
cycles/non-finite results with named diagnostics. Display rounding is not input.
Reference dimensions read solved geometry; they cannot feed a driving expression
in this first implementation, avoiding an implicit second cyclic solver.

## Consequences

The numerical backend can change without changing stored IDs or expression
semantics. The app retains ownership of constraint acceptance and diagnostics.
The recorded evidence does not establish universal numerical robustness or a
delivery estimate. No new hard geometry limits are introduced. Provisional
production backend selection does not constitute full customer acceptance.

## Feasibility checkpoint (2026-10-30)

Bounded source/evidence review permits starting the concrete production adapter
with `acs-solver =0.1.19` as the provisional backend; it is not full story
acceptance. Its cached manifest declares MIT, no build script and an opt-in
`c-abi` feature (leave disabled). Dependencies are nalgebra, serde, serde_json
and wasm-bindgen; workspace dependency/license checks still apply.

The isolated experiment records 42 native/WASM cases, 768 compared numeric
values, maximum difference 1.2461446930467808e-14 and no comparison failures.
Repeated moving geometry checks all 49 hard segment lengths and the requested
endpoint over 20 seeded steps. Recorded thread CPU is 102.852096 ms for those
20 steps, about 5.14 ms/frame on average; this aggregate does not establish the
16 ms worst-frame bound or moving-frame diagnosis cost. Earlier rigid-chain
maxima are 1.30 ms without analysis and 2.94 ms with analysis. Architect inspected
saved evidence and source; did not rerun these experiments.

Known unsupported ellipse-circle/ellipse-ellipse tangent requests are expected
rejections in this corpus, not successful tangency tests. Define the product's
valid selection matrix explicitly before claiming complete coverage; no accepted
constraint combination may disappear merely because a backend lacks it.

The reviewed numerical/JSON entry modules have no direct filesystem, network,
clock or thread calls. The C ABI is separate. This is not an exhaustive transitive
unsafe audit. The JSON entry validates duplicate IDs and geometry references;
the lower-level system constructor can panic on unknown variables. Production
must validate graph references, types, finite values and iteration conversion
before calling it, and validate returned IDs, finiteness and hard residuals
before publishing a candidate. Do not depend on catching panics on WASM.

Variable registration sorts IDs and diagnosis sorts constraint indices. Component
grouping uses HashMap iteration, so canonical input order alone is not a proof
of deterministic processing. Preserve explicit seeds and stable output mapping;
add disconnected-component repeatability tests before final acceptance. Existing
native/WASM agreement is corpus evidence, not universal numerical determinism.

## Dependency-policy disposition

The final gate exposed `paste 1.0.15` through pinned ACS 0.1.19 → nalgebra
0.33.3 → simba 0.9.1. [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)
classifies it as **informational: unmaintained**, with no patched version;
this advisory does not report a security vulnerability. Simba 0.9.1 requires
`paste` unconditionally, so disabling an optional feature does not remove it.

**(Proposal, reviewed)** Add only this advisory ID to the existing informational
transitive-dependency exceptions, with the rationale tracked in
[technical debt](../technical-debt.md). Keep numerical dependency versions and
features unchanged. This follows the repository's existing advisory disposition;
it does not disable advisory checks globally or change the licence policy.

The [upstream Simba 0.10.2 manifest](https://raw.githubusercontent.com/dimforge/simba/master/Cargo.toml)
no longer lists `paste`, but that line is outside nalgebra 0.33.3's `simba ^0.9`
requirement. No compatible replacement has been established for this pinned tree.
A fork, replacement or numerical dependency upgrade needs its own compatibility
and native/WASM regression evidence. Revisit this exception at each ACS/nalgebra
upgrade. The lead owns the configuration change and ensuing `cargo deny` check;
this source/docs review does not claim that rerun passed.
