//! The `assets/` members of the project container (ADR 0017 §2, `specs/0058-bitmap-import`
//! criteria 23 and 44): written after the three document members in hash order, Stored (the
//! pictures are compressed already); read bounded, unhashed, and never an open failure.

use std::io::{Cursor, Read};
use std::sync::Arc;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::assets::{
    AssetHash, AssetStore, MAX_ASSET_BYTES, MAX_PROJECT_ASSET_BYTES, asset_extension,
};
use crate::error::SaveError;

const ASSETS_DIR: &str = "assets/";
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
    Ok(())
}

/// The hash a member name `assets/<64 hex>.<ext>` names.
fn member_hash(name: &str) -> Option<AssetHash> {
    let file = name.strip_prefix(ASSETS_DIR)?;
    let (hash, extension) = file.split_once('.')?;
    EXTENSIONS.contains(&extension).then_some(())?;
    AssetHash::parse(hash)
}

/// Every readable asset member of `archive`. A member over [`MAX_ASSET_BYTES`], one that cannot
/// be read, and everything past [`MAX_PROJECT_ASSET_BYTES`] in total are left out: the image
/// that names it shows as unreadable. The bytes are not hashed here; the decode checks them.
pub(crate) fn read_assets(archive: &mut ZipArchive<Cursor<&[u8]>>) -> AssetStore {
    let mut store = AssetStore::default();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let Ok(mut file) = archive.by_index(index) else {
            continue;
        };
        let Some(hash) = file.name().ok().and_then(|name| member_hash(&name)) else {
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
    }
}
