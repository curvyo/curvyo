//! The `assets/` members of the project container (ADR 0017 §2, `specs/0058-bitmap-import`
//! criteria 23 and 44; ADR 0077 §1 for `assets/derived/`): written after the three document
//! members in hash order, Stored (the pictures are compressed already); read bounded and never an
//! open failure.

use std::io::{Cursor, Read};
use std::sync::Arc;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::assets::{
    AssetHash, AssetStore, MAX_ASSET_BYTES, MAX_DERIVED_ASSET_BYTES, MAX_DERIVED_PROJECT_BYTES,
    MAX_PROJECT_ASSET_BYTES, asset_extension,
};
use crate::error::SaveError;

const ASSETS_DIR: &str = "assets/";
const DERIVED_DIR: &str = "assets/derived/";
const EXTENSIONS: [&str; 4] = ["png", "jpg", "webp", "bin"];

/// Writes every asset of `store` as `assets/<hash>.<ext>`, Stored.
pub(crate) fn write_assets(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    store: &AssetStore,
) -> Result<(), SaveError> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (hash, bytes) in store.iter() {
        let name = format!("{ASSETS_DIR}{}.{}", hash.as_str(), asset_extension(bytes));
        writer
            .start_file(name, options)
            .map_err(|_| SaveError::Container)?;
        std::io::Write::write_all(writer, bytes).map_err(|_| SaveError::Container)?;
    }
    for (hash, bytes) in store.iter_derived() {
        let name = format!("{DERIVED_DIR}{}.png", hash.as_str());
        writer
            .start_file(name, options)
            .map_err(|_| SaveError::Container)?;
        std::io::Write::write_all(writer, bytes).map_err(|_| SaveError::Container)?;
    }
    Ok(())
}

/// The hash a member name `assets/derived/<64 hex>.png` names.
pub(crate) fn derived_hash(name: &str) -> Option<AssetHash> {
    AssetHash::parse(name.strip_prefix(DERIVED_DIR)?.strip_suffix(".png")?)
}

/// The hash a member name `assets/<64 hex>.<ext>` names.
pub(crate) fn member_hash(name: &str) -> Option<AssetHash> {
    let file = name.strip_prefix(ASSETS_DIR)?;
    let (hash, extension) = file.split_once('.')?;
    EXTENSIONS.contains(&extension).then_some(())?;
    AssetHash::parse(hash)
}

/// Every readable asset member of `archive`. A member over [`MAX_ASSET_BYTES`], one that cannot
/// be read, and everything past [`MAX_PROJECT_ASSET_BYTES`] in total are left out: the image
/// that names it shows as unreadable. The bytes of an original are not hashed here (the decode
/// checks them); a derived member is (ADR 0077 §1: an oversized or mismatching one is a missing
/// result).
pub(crate) fn read_assets(archive: &mut ZipArchive<Cursor<&[u8]>>) -> AssetStore {
    let mut store = AssetStore::default();
    let mut total = 0_u64;
    let mut derived_total = 0_u64;
    for index in 0..archive.len() {
        let Ok(mut file) = archive.by_index(index) else {
            continue;
        };
        let name = file.name().ok().map(|n| n.to_string()).unwrap_or_default();
        if let Some(hash) = derived_hash(&name) {
            let mut bytes = Vec::new();
            let read = (&mut file)
                .take(MAX_DERIVED_ASSET_BYTES + 1)
                .read_to_end(&mut bytes);
            let size = bytes.len() as u64;
            if read.is_ok()
                && size <= MAX_DERIVED_ASSET_BYTES
                && derived_total + size <= MAX_DERIVED_PROJECT_BYTES
                && AssetHash::of(&bytes) == hash
            {
                derived_total += size;
                store.load_derived(hash, Arc::from(bytes));
            }
            continue;
        }
        let Some(hash) = member_hash(&name) else {
            continue;
        };
        let mut bytes = Vec::new();
        let read = (&mut file)
            .take(MAX_ASSET_BYTES + 1)
            .read_to_end(&mut bytes);
        let size = bytes.len() as u64;
        if read.is_err() || size > MAX_ASSET_BYTES || total + size > MAX_PROJECT_ASSET_BYTES {
            continue;
        }
        total += size;
        store.insert(hash, Arc::from(bytes));
    }
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_hash_named_members_with_a_known_extension_are_assets() {
        let hash = AssetHash::of(b"x");
        assert_eq!(
            member_hash(&format!("assets/{}.png", hash.as_str())),
            Some(hash.clone())
        );
        assert_eq!(member_hash(&format!("assets/{}.exe", hash.as_str())), None);
        assert_eq!(member_hash(&format!("other/{}.png", hash.as_str())), None);
        assert_eq!(member_hash("assets/short.png"), None);
        let derived = format!("assets/derived/{}.png", hash.as_str());
        assert_eq!(
            member_hash(&derived),
            None,
            "a derived member is no original"
        );
        assert_eq!(derived_hash(&derived), Some(hash.clone()));
        assert_eq!(
            derived_hash(&format!("assets/derived/{}.jpg", hash.as_str())),
            None
        );
    }
}
