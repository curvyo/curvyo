//! The pure parts of a recovery copy's container (`specs/0136-autosave-and-restore`, ADR 0079
//! §4 and §5): listing the assets a copy leaves out, checking a copy cheaply, and assembling a
//! copy and its assets into a normal `.curvyo` that Open reads. Zip in and out over bytes.

use std::io::{Cursor, Write};
use std::sync::Arc;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::assets::asset_extension;
use crate::container::{CURRENT_LORO_SNAPSHOT_VERSION, Manifest};
use crate::container_assets::{derived_hash, member_hash};
use crate::document::{CURRENT_FORMAT_VERSION, Document};
use crate::error::{OpenError, SaveError};

const ASSETS_DIR: &str = "assets/";

/// One asset a recovery copy leaves out: its member name below `assets/` (`<hash>.<ext>`, or
/// `derived/<hash>.png`) and its bytes.
#[derive(Debug, Clone)]
pub struct RecoveryAsset {
    /// The name below `assets/`.
    pub name: String,
    /// The bytes.
    pub bytes: Arc<[u8]>,
}

/// Every asset of `document`, named as [`crate::pack`] names its members. Unreferenced derived
/// results are pruned first, as a save does.
#[must_use]
pub fn recovery_assets(document: &Document) -> Vec<RecoveryAsset> {
    document.prune_derived_assets();
    let store = document.assets.borrow();
    let originals = store.iter().map(|(hash, bytes)| RecoveryAsset {
        name: format!("{}.{}", hash.as_str(), asset_extension(bytes)),
        bytes: Arc::clone(bytes),
    });
    let derived = store.iter_derived().map(|(hash, bytes)| RecoveryAsset {
        name: format!("derived/{}.png", hash.as_str()),
        bytes: Arc::clone(bytes),
    });
    originals.chain(derived).collect()
}

/// Whether `name` is a member name below `assets/` that Open reads.
fn is_asset_name(name: &str) -> bool {
    let member = format!("{ASSETS_DIR}{name}");
    member_hash(&member).is_some() || derived_hash(&member).is_some()
}

/// Checks a recovery copy without opening the document: the central directory reads, the
/// manifest is not newer than this build, and the snapshot member is there. The pages of the
/// copy are not counted here (the record holds them).
///
/// # Errors
/// The named errors of Open: [`OpenError::NotAProject`], [`OpenError::Damaged`],
/// [`OpenError::FormatTooNew`].
pub fn check_recovery_copy(bytes: &[u8]) -> Result<(), OpenError> {
    if !(bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")) {
        return Err(OpenError::NotAProject);
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| OpenError::Damaged)?;
    let manifest: Manifest = {
        let mut file = archive
            .by_name("manifest.json")
            .map_err(|_| OpenError::Damaged)?;
        let mut text = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut text).map_err(|_| OpenError::Damaged)?;
        serde_json::from_slice(&text).map_err(|_| OpenError::Damaged)?
    };
    if manifest.format_version > CURRENT_FORMAT_VERSION {
        return Err(OpenError::FormatTooNew {
            found: manifest.format_version,
            supported: CURRENT_FORMAT_VERSION,
        });
    }
    if manifest.loro_snapshot_version > CURRENT_LORO_SNAPSHOT_VERSION {
        return Err(OpenError::FormatTooNew {
            found: manifest.loro_snapshot_version,
            supported: CURRENT_LORO_SNAPSHOT_VERSION,
        });
    }
    for member in ["document.loro", "document.json"] {
        archive.by_name(member).map_err(|_| OpenError::Damaged)?;
    }
    Ok(())
}

/// A normal `.curvyo` made of a recovery `copy` and its `assets` (name below `assets/`, bytes):
/// the copy's members as they are, then each asset Stored. A name Open would not read is left
/// out, so a missing or stray file never refuses the copy.
///
/// # Errors
/// [`SaveError::Container`] when `copy` is no readable container.
pub fn assemble_recovery_copy(copy: &[u8], assets: &[(&str, &[u8])]) -> Result<Vec<u8>, SaveError> {
    let mut archive = ZipArchive::new(Cursor::new(copy)).map_err(|_| SaveError::Container)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let member = archive
            .by_index_raw(index)
            .map_err(|_| SaveError::Container)?;
        writer
            .raw_copy_file(member)
            .map_err(|_| SaveError::Container)?;
    }
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in assets {
        if !is_asset_name(name) {
            continue;
        }
        writer
            .start_file(format!("{ASSETS_DIR}{name}"), options)
            .map_err(|_| SaveError::Container)?;
        writer.write_all(bytes).map_err(|_| SaveError::Container)?;
    }
    Ok(writer
        .finish()
        .map_err(|_| SaveError::Container)?
        .into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetHash;
    use crate::container::{pack, pack_recovery, pack_snapshot, unpack};

    fn picture() -> (AssetHash, Arc<[u8]>) {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(b"not really a picture");
        (AssetHash::of(&bytes), Arc::from(bytes))
    }

    fn members(bytes: &[u8]) -> Vec<String> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index_raw(i).unwrap().name().unwrap().to_string())
            .collect()
    }

    #[test]
    fn a_recovery_copy_holds_no_assets_and_no_share_record() {
        let document = Document::new(1);
        let (hash, bytes) = picture();
        document.assets.borrow_mut().insert(hash, bytes);
        let copy = pack_recovery(&document, "0.1.0").unwrap();
        assert!(members(&copy).iter().all(|m| !m.starts_with("assets/")));
        assert!(
            members(&pack(&document, "0.1.0").unwrap())
                .iter()
                .any(|m| m.starts_with("assets/"))
        );
        assert!(check_recovery_copy(&copy).is_ok());
    }

    #[test]
    fn copy_plus_assets_assembles_to_what_save_writes() {
        let document = Document::new(1);
        let (hash, bytes) = picture();
        document
            .assets
            .borrow_mut()
            .insert(hash.clone(), bytes.clone());
        let copy = pack_recovery(&document, "0.1.0").unwrap();
        let listed = recovery_assets(&document);
        assert_eq!(listed.len(), 1);
        let pairs: Vec<(&str, &[u8])> = listed
            .iter()
            .map(|a| (a.name.as_str(), &*a.bytes))
            .collect();
        let assembled = assemble_recovery_copy(&copy, &pairs).unwrap();
        let reopened = unpack(2, &assembled).unwrap();
        assert_eq!(reopened.size(), document.size());
        assert_eq!(&*reopened.assets.borrow().get(&hash).unwrap(), &*bytes);
        let saved = pack(&document, "0.1.0").unwrap();
        let mut a = members(&assembled);
        // ADR 0084 §1: the page list is written by a save only; the restored copy gets it at
        // its next save.
        let mut b: Vec<_> = members(&saved)
            .into_iter()
            .filter(|name| name != crate::LIBRARY_MEMBER)
            .collect();
        a.sort();
        b.sort();
        assert_eq!(a, b, "the same members as Save writes");
        assert_eq!(
            members(&pack_snapshot(&document, "0.1.0").unwrap()).len(),
            b.len()
        );
    }

    #[test]
    fn a_derived_asset_keeps_its_place_below_assets_derived() {
        let document = Document::new(1);
        let copy = pack_recovery(&document, "0.1.0").unwrap();
        let derived = b"\x89PNG\r\n\x1a\nresult".to_vec();
        let name = format!("derived/{}.png", AssetHash::of(&derived).as_str());
        let assembled = assemble_recovery_copy(
            &copy,
            &[(name.as_str(), derived.as_slice()), ("nonsense.exe", b"x")],
        )
        .unwrap();
        let names = members(&assembled);
        assert!(names.contains(&format!("assets/{name}")));
        assert!(!names.iter().any(|m| m.contains("nonsense")));
    }

    #[test]
    fn the_check_names_the_same_errors_as_open() {
        assert!(matches!(
            check_recovery_copy(b"plain text"),
            Err(OpenError::NotAProject)
        ));
        let copy = pack_recovery(&Document::new(1), "0.1.0").unwrap();
        assert!(matches!(
            check_recovery_copy(&copy[..copy.len() / 2]),
            Err(OpenError::Damaged)
        ));
    }
}
