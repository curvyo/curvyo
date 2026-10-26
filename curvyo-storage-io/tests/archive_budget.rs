//! The archive budget of `specs/0083-font-info-and-zip` criterion 13: adding an archive of 20
//! fonts and 10 MB takes at most 2 s. Measured in release (`cargo test --release -p
//! curvyo-storage-io --test archive_budget -- --nocapture`); asserted only in release and off CI,
//! printed always.
#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn included(path: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(path),
    )
    .unwrap()
}

#[test]
fn ac13_twenty_fonts_and_ten_mb_within_two_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let fonts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
    let mut fonts: Vec<(String, Vec<u8>)> = Vec::new();
    for family in std::fs::read_dir(&fonts_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
    {
        for file in std::fs::read_dir(family.path()).unwrap().flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            if [".ttf", ".otf", ".svg"]
                .iter()
                .any(|ext| name.ends_with(ext))
            {
                fonts.push((
                    name.clone(),
                    included(&format!("{}/{name}", family.file_name().to_string_lossy())),
                ));
            }
        }
    }
    // Copies of the first faces under other names (taken as the same face again) make 20.
    fonts.sort();
    fonts.truncate(20);
    let copies: Vec<(String, Vec<u8>)> = (0..20 - fonts.len())
        .map(|i| (format!("copy-{i}-{}", fonts[i].0), fonts[i].1.clone()))
        .collect();
    fonts.extend(copies);
    let mut seed = 7_u32;
    let filler: Vec<u8> = (0..8 * 1024 * 1024)
        .map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16).to_le_bytes()[0]
        })
        .collect();
    let path = dir.path().join("Pack.zip");
    let mut out = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    for (name, bytes) in fonts
        .iter()
        .chain(std::iter::once(&("Specimen.pdf".to_owned(), filler)))
    {
        out.start_file(
            format!("Pack/{name}"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        out.write_all(bytes).unwrap();
    }
    out.finish().unwrap();
    let size = std::fs::metadata(&path).unwrap().len();
    assert!(size >= 10 * 1000 * 1000, "{size}");
    let started = Instant::now();
    let report = curvyo_storage_io::add_font_archive(&dir.path().join("fonts"), &path);
    let took = started.elapsed();
    assert_eq!(report.added.len() + report.present, 20);
    let budget = Duration::from_secs(2);
    println!("20 fonts, {size} bytes: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) && std::env::var_os("CI").is_none() {
        assert!(took <= budget, "{took:?}");
    }
}
