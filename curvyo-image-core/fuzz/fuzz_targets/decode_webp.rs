//! Decodes fuzzed bytes as a WebP with the import's path (probe, working image, mips): no panic,
//! no hang, bounded memory (`specs/0058-bitmap-import` criteria 19 and 20).
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "shared.rs"]
mod shared;

fuzz_target!(|data: &[u8]| {
    if let Some(bytes) = shared::with_signature(data, &[(0, b"RIFF".as_slice()), (8, b"WEBP".as_slice())]) {
        shared::run(&bytes);
    }
});
