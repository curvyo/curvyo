# PCG32: the generator of the tiling variance

`specs/0069-tiling` criterion 20, `specs/0069-tiling/adrs.md` decisions 2 and 3. Code:
`curvyo-ui-core/src/tiling/pcg32.rs`; reference: `tools/pcg32-reference/pcg32_reference.c`;
golden outputs: `tests/fixtures/tiling/pcg32.json`.

## Algorithm

M. E. O'Neill's PCG32 in the `pcg32_oneseq` variant of pcg-c (`pcg_oneseq_64_xsh_rr_32`):

- State: one `u64`.
- Step: `state = state * 6364136223846793005 + 1442695040888963407` (wrapping). The increment
  is pcg-c's fixed `PCG_DEFAULT_INCREMENT_64`; there is no stream selector.
- Output (XSH-RR) of the state *before* the step:
  `xorshifted = (u32)(((old >> 18) ^ old) >> 27)`, `rot = old >> 59`,
  `output = xorshifted.rotate_right(rot)`.
- Seeding (`pcg_oneseq_64_srandom_r`): `state = 0`, step, `state += seed`, step.
- Jump-ahead (`pcg_advance_lcg_64`): skipping `delta` outputs composes the LCG with itself by
  repeated squaring, O(log delta).

## Use in tiling

- The seed is the recipe's `u32` seed, widened to `u64`. The dice in the bar draws a new seed
  with `crypto.getRandomValues` in the frontend; the core reads no clock or entropy.
- Every cell has its own stretch of eight draws: the stream of the seed is advanced by
  `cell_index · 8`, `cell_index = row · 2^32 + column` (a circle uses row 0 and the instance
  index as column).
- The cell then draws, always and in this order: rotation, scale X, scale Y, offset X,
  offset Y, opacity, hue; the eighth draw is spare. A channel at zero still draws, so one
  channel never changes another and adding cells never changes existing ones.
- A draw `x` becomes the unit value `u = x / 2^32` (exact in `f64`, in `[0, 1)`); a channel
  with range `r`, row step `s` and column step `t` gives `row · s + column · t + (2u - 1) · r`.

## Checking the goldens

```text
mkdir -p target
cc -O2 -o target/pcg32_reference tools/pcg32-reference/pcg32_reference.c
target/pcg32_reference > tests/fixtures/tiling/pcg32.json
cargo test -p curvyo-ui-core tiling
cargo test --target wasm32-unknown-unknown -p curvyo-ui-core --test tiling_golden
```

The C program is a reimplementation of pcg-c (M. E. O'Neill, Apache-2.0 OR MIT); nothing in
the build or CI compiles it. It prints the first 16 outputs for the seeds 0, 1, 42 and 4,294,967,295 and three
jump-ahead results; the Rust tests compare against that file natively and in the wasm build.
