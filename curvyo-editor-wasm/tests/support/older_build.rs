//! A project as a build without embedded fonts writes it (`specs/0107-document-fonts`): the
//! container with its `assets/` members left out. A test that models "another computer that does
//! not have the font" uses it, because a session now embeds the fonts its texts use.

use std::io::{Cursor, Read, Write};

/// `bytes` without the `assets/` members.
pub fn without_embedded_fonts(bytes: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let name = file.name().unwrap().to_string();
        if name.starts_with("assets/") {
            continue;
        }
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        writer.start_file(name, options).unwrap();
        writer.write_all(&content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
