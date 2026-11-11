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
code from the GPL `libspiro` fork; its numerical output is used as independent
test data. Keep the package's copyright and licence information with any
redistribution of the dependency.
