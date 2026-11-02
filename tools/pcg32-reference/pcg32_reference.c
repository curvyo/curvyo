/*
 * Reference output of PCG32 for the golden test of `curvyo-ui-core/src/tiling/pcg32.rs`
 * (`specs/0069-tiling` criterion 20). This is M. E. O'Neill's pcg-c `pcg32_oneseq`
 * (`pcg_oneseq_64_xsh_rr_32`): 64-bit LCG state, multiplier 6364136223846793005, the fixed
 * increment 1442695040888963407 (PCG_DEFAULT_INCREMENT_64), XSH-RR output of the old state, and
 * `pcg_oneseq_64_srandom_r` seeding. The jump-ahead is pcg-c's `pcg_advance_lcg_64`.
 *
 * Upstream: pcg-c by Melissa E. O'Neill, https://github.com/imneme/pcg-c, licensed
 * Apache-2.0 OR MIT. This file is a reimplementation of those algorithms from pcg-c, not a
 * copy of its sources. Nothing in the build, the tests or CI compiles or runs it; it exists
 * only to regenerate the fixture by hand.
 *
 * Build and run from the repository root (the binary stays inside the project's `target/`):
 *   cc -O2 -o target/pcg32_reference tools/pcg32-reference/pcg32_reference.c
 *   target/pcg32_reference > tests/fixtures/tiling/pcg32.json
 */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

#define PCG_MULT 6364136223846793005ULL
#define PCG_INC 1442695040888963407ULL

typedef struct { uint64_t state; } pcg32_oneseq;

static void step(pcg32_oneseq *rng) { rng->state = rng->state * PCG_MULT + PCG_INC; }

static uint32_t output_xsh_rr(uint64_t state) {
    uint32_t xorshifted = (uint32_t)(((state >> 18u) ^ state) >> 27u);
    uint32_t rot = (uint32_t)(state >> 59u);
    return (xorshifted >> rot) | (xorshifted << ((-rot) & 31u));
}

static uint32_t next_u32(pcg32_oneseq *rng) {
    uint64_t old = rng->state;
    step(rng);
    return output_xsh_rr(old);
}

static void seed(pcg32_oneseq *rng, uint64_t initstate) {
    rng->state = 0u;
    step(rng);
    rng->state += initstate;
    step(rng);
}

static uint64_t advance_lcg_64(uint64_t state, uint64_t delta, uint64_t cur_mult, uint64_t cur_plus) {
    uint64_t acc_mult = 1u, acc_plus = 0u;
    while (delta > 0) {
        if (delta & 1) {
            acc_mult *= cur_mult;
            acc_plus = acc_plus * cur_mult + cur_plus;
        }
        cur_plus = (cur_mult + 1) * cur_plus;
        cur_mult *= cur_mult;
        delta /= 2;
    }
    return acc_mult * state + acc_plus;
}

int main(void) {
    const uint64_t seeds[4] = {0u, 1u, 42u, 4294967295u};
    printf("{\n  \"algorithm\": \"pcg32_oneseq (pcg_oneseq_64_xsh_rr_32), multiplier 6364136223846793005, increment 1442695040888963407\",\n");
    printf("  \"sequences\": [\n");
    for (int s = 0; s < 4; s++) {
        pcg32_oneseq rng;
        seed(&rng, seeds[s]);
        printf("    { \"seed\": %" PRIu64 ", \"outputs\": [", seeds[s]);
        for (int i = 0; i < 16; i++) {
            printf("%s%" PRIu32, i ? ", " : "", next_u32(&rng));
        }
        printf("] }%s\n", s < 3 ? "," : "");
    }
    printf("  ],\n  \"advance\": [\n");
    const uint64_t deltas[3] = {1000u, (uint64_t)7 << 32 | 13u, 0xFFFFFFFFFFFFFFFFull};
    for (int d = 0; d < 3; d++) {
        pcg32_oneseq rng;
        seed(&rng, 42u);
        rng.state = advance_lcg_64(rng.state, deltas[d], PCG_MULT, PCG_INC);
        printf("    { \"seed\": 42, \"delta\": %" PRIu64 ", \"next\": %" PRIu32 " }%s\n", deltas[d], next_u32(&rng), d < 2 ? "," : "");
    }
    printf("  ]\n}\n");
    return 0;
}
