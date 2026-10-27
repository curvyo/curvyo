//! Opens fuzzed bytes as a project container and reads every image asset it names: no panic,
//! no hang, bounded reads (`specs/0058-bitmap-import` criterion 44, ADR 0017 §2).
#![no_main]

use curvyo_document_core::{ObjectSnapshot, unpack};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(document) = unpack(1, data) else {
        return;
    };
    for object in document.read_objects().objects.iter() {
        if let ObjectSnapshot::Image(image) = object {
            if let Some(bytes) = document.asset(&image.asset) {
                assert!(bytes.len() as u64 <= curvyo_document_core::MAX_ASSET_BYTES);
            }
        }
    }
});
