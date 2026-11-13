# spiro-inner 0.1.0

Curvyo uses `spiro-inner` 0.1.0 for the series integral of a polynomial spiral
(`integrate_spiro`) in its Spiro solver: the constraint residual and its
Jacobian are built from it. The Newton iteration itself is Curvyo's own.
Its package metadata and README declare Apache-2.0 for the Rust port.
The port names Fredrick R. Brennan, Seth Erfurt and MFEK Authors (2020–2022)
and Raph Levien (2007) as copyright holders. The port README attributes the C
source to Levien and identifies the original C source as Apache-2.0 or MIT.

Sources:

- [Published crate](https://crates.io/crates/spiro-inner/0.1.0), archive SHA-256
  `ad0035e3e6451669d2355e03b9a1d8c98226b00f1993d31fda918580f718ae91`.
- [First port commit](https://github.com/MFEK/spiro-inner.rlib/commit/a7809139441c),
  whose README distinguishes the Rust adaptation's licence and the C source's licence.
- [Levien's original Spiro repository](https://github.com/raphlinus/spiro), whose
  README distinguishes its Apache-2.0/MIT original from the GPL `libspiro` fork.
- [Apache License, version 2.0](https://www.apache.org/licenses/LICENSE-2.0).

The package source also refers ambiguously to “Raph Levien's libspiro”. The
initial port README explicitly identifies Levien's original C source and its
Apache-2.0/MIT licence; the `integrate_spiro` and `compute_ends` formulas were
compared against the original `ppedit/spiro.c`; Curvyo's endpoint calculation
uses those same formulas. Curvyo does not incorporate
code from the GPL `libspiro` fork. The curvature-continuous (`c` point) formulation of `0115`
(clothoid segments, end arcs) follows Levien's permissive original (Apache-2.0 or MIT), the
`ppedit/spiro.c` source above, in Curvyo's own solver; no code is taken from
the GPL fork. Its numerical output (G4 `o` points) was test data
until `0115` (2026-11-12) moved the solver to curvature-continuous `c` points, which are
checked by property tests instead.

Since 2026-11-12 (`0115`, after the customer's review) the solver runs the iteration of Levien's
`solve_spiro` as Inkscape's Spiro mode does: from straight segments, at most ten full Newton steps
with the angle errors and each segment's turn reduced after every step, and the last iterate drawn
without a convergence check. Inkscape's `src/live_effects/spiro.cpp` (GPL-2.0-or-later) and
`libspiro` (GPL-3) were read to learn that procedure; the project's AGPL-3.0 would allow copying
them, but at the customer's preference no code was taken from either. Curvyo's iteration is its
own, written from the procedure and from Levien's permissive original (`ppedit/spiro.c`, whose
`solve_spiro` runs the same ten steps) and its `spiro-inner` port, as recorded in ADR 0053
(amendment 2026-11-12, "Pure function of the controls"). The per-step reductions (each segment's turn, `ks[0] = 2 mod_2pi(ks[0] / 2)`, and the angle errors) come from the procedure of the GPL forks; the ten steps and the `1e-12` stop on the squared step are in the permissive original. Keep the package's copyright and licence
information with any redistribution of the dependency.
