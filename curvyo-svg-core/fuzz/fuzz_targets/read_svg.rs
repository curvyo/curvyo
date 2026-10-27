//! Reads fuzzed bytes as an SVG file with the import's reader: no panic, no hang, bounded memory
//! (`specs/0024-svg-import-export` criteria 69 and 70).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = curvyo_svg_core::read_svg(data);
});
