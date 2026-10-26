//! Opens fuzzed bytes as a font archive and reads every font and document entry: no panic, no
//! hang, bounded reads (`specs/0083-font-info-and-zip` criterion 16).
#![no_main]

use curvyo_text_core::{EntryClass, FontArchive};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(mut archive) = FontArchive::open(data.to_vec()) else {
        return;
    };
    for i in 0..archive.entries().len() {
        if matches!(archive.entries()[i].class, EntryClass::Font(_) | EntryClass::Document) {
            let _ = archive.read(i);
        }
    }
});
