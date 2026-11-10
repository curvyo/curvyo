//! Real raster fixtures of the reference set (`specs/0089-trace-quality-and-colour` criterion 1,
//! Frage 82): CC0 or public-domain images committed under `tests/fixtures/trace/reference/` with
//! a JSON sidecar each, the customer's reference images in `tests/assets/trace-references/`
//! (sidecars in `reference/customer/`), plus the images from the directory in
//! `CURVYO_TRACE_LOCAL`, of which only numbers are ever stored.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::super::{Picture, fixture_dir};
use super::reference::{Group, Use};

/// A real image ready to trace.
pub struct RealImage {
    /// The sidecar's id, or `local.<hash>` for a local image.
    pub id: String,
    /// `None` for a local image (its group is unknown).
    pub group: Option<Group>,
    pub dpi: f64,
    pub picture: Picture,
}

/// The directory of the committed real fixtures.
pub fn reference_dir() -> PathBuf {
    fixture_dir().join("reference")
}

/// Every sidecar (`*.json`) of the committed real fixtures with its path, sorted by name.
pub fn sidecars() -> Vec<(PathBuf, Value)> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(reference_dir())
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            let value = serde_json::from_str(&text).unwrap();
            (p, value)
        })
        .collect()
}

fn decode(path: &Path) -> Picture {
    let bytes = std::fs::read(path).unwrap();
    let image = curvyo_image_core::decode_working(&bytes).unwrap();
    Picture {
        width: image.width,
        height: image.height,
        rgba: image.pixels,
    }
}

/// The committed real fixtures, decoded.
pub fn real_images() -> Vec<RealImage> {
    sidecars()
        .into_iter()
        .map(|(path, sidecar)| {
            let file = path.with_file_name(sidecar["file"].as_str().unwrap());
            RealImage {
                id: sidecar["id"].as_str().unwrap().to_string(),
                group: Group::from_key(sidecar["group"].as_str().unwrap()),
                dpi: sidecar["dpi"].as_f64().unwrap(),
                picture: decode(&file),
            }
        })
        .collect()
}

/// The sidecars of the customer's reference images (`customer/*.json`), sorted by name: each
/// names its image by a path from the repository root (`tests/assets/trace-references/`).
pub fn customer_sidecars() -> Vec<(PathBuf, Value)> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(reference_dir().join("customer"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            (p, value)
        })
        .collect()
}

/// The repository root (the image paths of the customer's sidecars start there).
pub fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The longest side the customer's images are traced at, px.
const CUSTOMER_MAX_SIDE: u32 = 2000;

/// `picture` reduced by a whole factor with area averaging until its longest side is at most
/// [`CUSTOMER_MAX_SIDE`].
fn reduce_to_max_side(picture: Picture) -> Picture {
    let factor = picture
        .width
        .max(picture.height)
        .div_ceil(CUSTOMER_MAX_SIDE) as usize;
    if factor <= 1 {
        return picture;
    }
    let (w, h) = (picture.width as usize, picture.height as usize);
    let (rw, rh) = (w / factor, h / factor);
    let mut rgba = Vec::with_capacity(rw * rh * 4);
    for y in 0..rh {
        for x in 0..rw {
            for k in 0..4 {
                let mut sum = 0_u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        sum += u32::from(
                            picture.rgba[((y * factor + dy) * w + x * factor + dx) * 4 + k],
                        );
                    }
                }
                let n = (factor * factor) as u32;
                rgba.push(((sum + n / 2) / n) as u8);
            }
        }
    }
    Picture {
        width: rw as u32,
        height: rh as u32,
        rgba,
    }
}

/// The customer's reference images, decoded in place and reduced (`specs/0089` criterion 1).
pub fn customer_images() -> Vec<RealImage> {
    customer_sidecars()
        .into_iter()
        .map(|(_, sidecar)| {
            let file = repository_root().join(sidecar["file"].as_str().unwrap());
            RealImage {
                id: sidecar["id"].as_str().unwrap().to_string(),
                group: Group::from_key(sidecar["group"].as_str().unwrap()),
                dpi: sidecar["dpi"].as_f64().unwrap(),
                picture: reduce_to_max_side(decode(&file)),
            }
        })
        .collect()
}

/// Whether a customer sidecar has its fields, a known group and use and an existing image.
pub fn customer_sidecar_complete(sidecar: &Value) -> Result<(), String> {
    for key in ["id", "file", "group", "use", "preserve", "dpi", "origin"] {
        if sidecar.get(key).is_none() {
            return Err(format!("missing {key}"));
        }
    }
    if Group::from_key(sidecar["group"].as_str().unwrap_or("")).is_none() {
        return Err(format!("unknown group {}", sidecar["group"]));
    }
    if Use::from_key(sidecar["use"].as_str().unwrap_or("")).is_none() {
        return Err(format!("unknown use {}", sidecar["use"]));
    }
    let file = repository_root().join(sidecar["file"].as_str().unwrap_or(""));
    if !file.is_file() {
        return Err(format!("no image at {}", file.display()));
    }
    Ok(())
}

/// FNV-1a over `bytes`: a stable id that names no file.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The images of the directory in `CURVYO_TRACE_LOCAL` (PNG, JPEG, WebP), at 150 dpi, keyed
/// `local.<hash of the bytes>`; empty when the variable is not set.
pub fn local_images() -> Vec<RealImage> {
    let Some(dir) = std::env::var_os("CURVYO_TRACE_LOCAL") else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.retain(|p| {
        p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            ["png", "jpg", "jpeg", "webp"].contains(&e.to_ascii_lowercase().as_str())
        })
    });
    paths.sort();
    paths
        .iter()
        .filter_map(|p| {
            let bytes = std::fs::read(p).ok()?;
            let image = curvyo_image_core::decode_working(&bytes).ok()?;
            Some(RealImage {
                id: format!("local.{:08x}", fnv(&bytes) >> 32),
                group: None,
                dpi: 150.0,
                picture: Picture {
                    width: image.width,
                    height: image.height,
                    rgba: image.pixels,
                },
            })
        })
        .collect()
}

/// Whether a sidecar has every field criterion 1 asks for, with a known group and use.
pub fn sidecar_complete(sidecar: &Value) -> Result<(), String> {
    for key in [
        "id",
        "file",
        "group",
        "use",
        "preserve",
        "dpi",
        "origin",
        "author",
        "licence",
        "licence_url",
        "retrieved",
        "modified",
    ] {
        if sidecar.get(key).is_none() {
            return Err(format!("missing {key}"));
        }
    }
    if Group::from_key(sidecar["group"].as_str().unwrap_or("")).is_none() {
        return Err(format!("unknown group {}", sidecar["group"]));
    }
    if Use::from_key(sidecar["use"].as_str().unwrap_or("")).is_none() {
        return Err(format!("unknown use {}", sidecar["use"]));
    }
    let licence = sidecar["licence"].as_str().unwrap_or("");
    if !(licence.starts_with("CC0") || licence.starts_with("Public domain")) {
        return Err(format!(
            "licence {licence} is neither CC0 nor public domain"
        ));
    }
    Ok(())
}
