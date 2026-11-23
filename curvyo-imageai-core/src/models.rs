//! The pinned model list compiled into the app (ADR 0075 §4, ADR 0076 §1): name, version, size,
//! SHA-256, licence, attribution and the one URL of each model; the same list as `models.toml`.

/// The three model kinds (`specs/0135` criterion 47).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelKind {
    /// Remove background: a salient-object mask.
    Background,
    /// Upscale: 4 x super-resolution.
    Upscale,
    /// AI denoise.
    Denoise,
}

impl ModelKind {
    /// The key used in `models.toml` and by the frontend.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Upscale => "upscale",
            Self::Denoise => "denoise",
        }
    }
}

/// One pinned model file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile {
    /// What it does.
    pub kind: ModelKind,
    /// The model's name, stored with each result.
    pub name: &'static str,
    /// The model's version, stored with each result.
    pub version: &'static str,
    /// The SHA-256 of the ONNX file; all zeros for a placeholder.
    pub sha256: [u8; 32],
    /// The size of the ONNX file in bytes.
    pub bytes: u64,
    /// Whether the hash and size are placeholders until the file is converted and uploaded.
    pub placeholder: bool,
    /// The SPDX licence of the weights.
    pub licence: &'static str,
    /// The copyright line of the weights.
    pub copyright: &'static str,
    /// The upstream repository.
    pub upstream: &'static str,
    /// The training data note (ADR 0075 §9).
    pub training_data: &'static str,
}

/// The one host of the model files (ADR 0076 §2).
pub const MODEL_HOST: &str = "https://models.curvyo.org";

/// The pinned models (`models.toml`).
pub const MODELS: [ModelFile; 3] = [
    ModelFile {
        kind: ModelKind::Background,
        name: "u2netp",
        version: "1",
        sha256: [
            0x47, 0xbb, 0x7b, 0x9d, 0x53, 0xf1, 0xee, 0xbd, 0xe6, 0x58, 0xcf, 0xb6, 0x3a, 0xa5,
            0xe1, 0x10, 0xe6, 0xfc, 0xab, 0x39, 0xb5, 0x07, 0x79, 0xeb, 0x2e, 0x55, 0x22, 0xe5,
            0xdf, 0xe1, 0x82, 0xed,
        ],
        bytes: 4_611_299,
        placeholder: false,
        licence: "Apache-2.0",
        copyright: "Copyright (c) 2020 Xuebin Qin",
        upstream: "https://github.com/xuebinqin/U-2-Net",
        training_data: "DUTS-TR (research saliency dataset; terms: research use)",
    },
    ModelFile {
        kind: ModelKind::Upscale,
        name: "realesr-animevideov3",
        version: "1",
        sha256: [
            0xb5, 0xdc, 0x4e, 0x5c, 0x20, 0xf4, 0x04, 0x24, 0x7a, 0x91, 0xee, 0x0c, 0xc1, 0x01,
            0x4c, 0x79, 0xf2, 0x94, 0x02, 0x74, 0xb2, 0x58, 0x6e, 0x67, 0xb1, 0x29, 0xd8, 0x36,
            0x9c, 0xb7, 0xc1, 0xa8,
        ],
        bytes: 2_492_873,
        placeholder: false,
        licence: "BSD-3-Clause",
        copyright: "Copyright (c) 2021, Xintao Wang",
        upstream: "https://github.com/xinntao/Real-ESRGAN",
        training_data: "Anime video frames with synthetic degradations (Real-ESRGAN AnimeVideo-v3; research data)",
    },
    ModelFile {
        kind: ModelKind::Denoise,
        name: "ffdnet-color",
        version: "1",
        sha256: [
            0x91, 0x32, 0x8f, 0xb5, 0x8c, 0x01, 0x84, 0xb3, 0x1c, 0x8e, 0xcf, 0xbf, 0x93, 0x4d,
            0xf8, 0x89, 0xa4, 0x7f, 0x29, 0xff, 0xd1, 0x3e, 0xa0, 0xa1, 0xb7, 0xc4, 0x0a, 0x70,
            0x6e, 0xd8, 0x80, 0xa0,
        ],
        bytes: 3_423_405,
        placeholder: false,
        licence: "MIT",
        copyright: "Copyright (c) 2019 Kai Zhang",
        upstream: "https://github.com/cszn/KAIR",
        training_data: "BSD400, ImageNet validation and Waterloo Exploration pictures with Gaussian noise (research sets)",
    },
];

/// The pinned model of `kind`.
#[must_use]
pub const fn model(kind: ModelKind) -> &'static ModelFile {
    match kind {
        ModelKind::Background => &MODELS[0],
        ModelKind::Upscale => &MODELS[1],
        ModelKind::Denoise => &MODELS[2],
    }
}

impl ModelFile {
    /// The lowercase hex of the SHA-256.
    #[must_use]
    pub fn sha256_hex(&self) -> String {
        use std::fmt::Write;
        self.sha256
            .iter()
            .fold(String::with_capacity(64), |mut hex, b| {
                let _ = write!(hex, "{b:02x}");
                hex
            })
    }

    /// The file's one URL: `https://models.curvyo.org/<name>/<version>/<sha256>.onnx`.
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "{MODEL_HOST}/{}/{}/{}.onnx",
            self.name,
            self.version,
            self.sha256_hex()
        )
    }

    /// The URL of the licence text served beside the file (ADR 0076 §9).
    #[must_use]
    pub fn licence_url(&self) -> String {
        format!("{MODEL_HOST}/{}/{}/LICENSE.txt", self.name, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_finds_its_own_entry() {
        for kind in [
            ModelKind::Background,
            ModelKind::Upscale,
            ModelKind::Denoise,
        ] {
            assert_eq!(model(kind).kind, kind);
        }
    }
}
