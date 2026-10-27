// Shared body of the three decoder targets: what the import does with a picked file
// (`probe`, then `decode_working`, then the GPU mip chain), plus the working-image invariants.

/// Runs the import's decoder path over `data` and checks the working image it returns.
pub fn run(data: &[u8]) {
    if curvyo_image_core::probe(data).is_err() {
        return;
    }
    let Ok(working) = curvyo_image_core::decode_working(data) else {
        return;
    };
    let pixels = u64::from(working.width) * u64::from(working.height);
    assert!(working.width > 0 && working.height > 0, "empty working image");
    assert!(pixels <= curvyo_image_core::MAX_WORKING_PIXELS, "working image over the limit");
    assert!(
        working.width.max(working.height) <= curvyo_image_core::MAX_WORKING_SIDE,
        "working side over the limit"
    );
    assert_eq!(working.pixels.len() as u64, pixels * 4, "RGBA8 buffer size");
    if pixels <= 1 << 20 {
        let mips = working.premultiplied_mips();
        let last = mips.last().expect("at least level 0");
        assert_eq!((last.width, last.height), (1, 1), "mip chain ends at 1 x 1");
    }
}

/// `data` with its first bytes replaced by `signature`, so the mutations stay in one format.
pub fn with_signature(data: &[u8], signature: &[(usize, &[u8])]) -> Option<Vec<u8>> {
    let mut out = data.to_vec();
    for (at, bytes) in signature {
        let end = at + bytes.len();
        if out.len() < end {
            return None;
        }
        out[*at..end].copy_from_slice(bytes);
    }
    Some(out)
}
