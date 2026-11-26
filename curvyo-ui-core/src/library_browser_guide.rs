//! The "Host your own library" guide (spec `0152` criteria 32 and 33, UX notes 0.6): how to put a
//! Git catalogue of templates on the net, worded for the `catalogue_format` this build reads and
//! shipped with the app (it sends nothing).

use serde::Serialize;

use curvyo_library_core::{
    CATALOGUE_FILE, MAX_INDEX_BYTES, MAX_PICTURE_BYTES, MAX_TEMPLATE_BYTES, MAX_TEMPLATES,
    SUPPORTED_CATALOGUE_FORMAT,
};

use crate::library_browser_text::grouped;

/// The example repository (`curvyo/templates`).
pub const EXAMPLE_REPOSITORY: &str = "https://github.com/curvyo/templates";

/// The guide's text, in the order the page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guide {
    /// "Host your own library".
    pub title: String,
    /// What a library is.
    pub intro: String,
    /// The files of one template's folder: `(file, what it is)`.
    pub layout: Vec<(String, String)>,
    /// The steps, in order.
    pub steps: Vec<String>,
    /// A two-template `curvyo-catalogue.json` (the Copy button copies it).
    pub example: String,
    /// The example repository's address.
    pub example_url: String,
    /// The fields of an entry: `(field, rule)`.
    pub fields: Vec<(String, String)>,
    /// The limits and the other rules, one line each.
    pub rules: Vec<String>,
    /// "Private repositories": the heading of the token steps (desktop build only, UX notes T13).
    pub private_title: String,
    /// The token steps in plain sentences, numbered by the page (`specs/0156` criterion 16).
    pub private_steps: Vec<String>,
}

fn mib(bytes: u64) -> String {
    format!("{} MiB", bytes / (1024 * 1024))
}

/// The example index: two templates, every field, valid for this build's `catalogue_format`.
fn example() -> String {
    format!(
        r#"{{
  "catalogue_format": {SUPPORTED_CATALOGUE_FORMAT},
  "name": "My templates",
  "templates": [
    {{
      "path": "boxes/finger-box",
      "name": "Finger box",
      "author": "Jane Doe",
      "tags": ["box", "laser", "customizable"],
      "licence": {{"spdx": "CC0-1.0", "name": "CC0 1.0", "url": "https://creativecommons.org/publicdomain/zero/1.0/"}},
      "fonts": [],
      "size": 48213,
      "sha256": "9f2c1a0e5b7d4c3a2f1e0d9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f3e2d1c0b9a",
      "thumbnail_sha256": "1b3d5f7092a4c6e8f0b2d4f6081a3c5e7f90b2d4f6a8c0e2f4b6d8f0a2c4e6f8",
      "format_version": 58
    }},
    {{
      "path": "signs/name-plate",
      "name": "Name plate",
      "author": "Max Mustermann",
      "tags": ["sign"],
      "licence": {{"spdx": "CC-BY-4.0", "name": "CC BY 4.0", "url": "https://creativecommons.org/licenses/by/4.0/"}},
      "credit": "Name plate by Max Mustermann, CC BY 4.0",
      "fonts": [{{"family": "Inter", "style": "Bold", "spdx": "OFL-1.1"}}],
      "size": 20480,
      "sha256": "4e07408562bedb8b60ce05c1decfe3ad16b72230967de01f640b7e4729b49fce"
    }}
  ]
}}"#
    )
}

/// The guide for this build.
#[must_use]
pub fn hosting_guide() -> Guide {
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    Guide {
        title: "Host your own library".to_owned(),
        intro: format!(
            "A library is a Git repository with one file {CATALOGUE_FILE} at its top and one folder per template."
        ),
        layout: vec![
            pair("template.curvyo", "The template: a Curvyo document."),
            pair(
                "thumbnail.png",
                &format!("Its picture, optional, at most {}.", mib(MAX_PICTURE_BYTES)),
            ),
        ],
        steps: vec![
            "Create a public repository, for example on GitHub.".to_owned(),
            "Add one folder per template, with the files above.".to_owned(),
            format!(
                "Write {CATALOGUE_FILE} at the top: one entry per template, with the size of its file in bytes and its SHA-256 (for example from sha256sum)."
            ),
            "Push the repository.".to_owned(),
            "In Curvyo: Library tab, Add source…, Git catalogue, and the address of the raw files, for example https://raw.githubusercontent.com/<owner>/<repository>/main/.".to_owned(),
        ],
        example: example(),
        example_url: EXAMPLE_REPOSITORY.to_owned(),
        fields: vec![
            pair("catalogue_format", &format!("{SUPPORTED_CATALOGUE_FORMAT}, the format this Curvyo reads.")),
            pair("name", "The library's name, shown for the source."),
            pair("path", "The template's folder: 1 to 4 parts of lower-case letters, digits, '.', '_' and '-', at most 200 characters."),
            pair("name (of a template)", "Its name, required."),
            pair("author", "Optional."),
            pair("tags", "Optional words; the tag customizable marks a template that takes your changes."),
            pair("licence", "spdx, name and url; without one the template reads as Licence unknown, never as free."),
            pair("credit", "Required for a licence that needs attribution (CC BY and the like)."),
            pair("fonts", "family, style and spdx of each font it uses; a font without a known licence is not fetched."),
            pair("size", "The size of template.curvyo in bytes."),
            pair("sha256", "The SHA-256 of template.curvyo, 64 lower-case hex digits."),
            pair("thumbnail_sha256", "The SHA-256 of thumbnail.png, when there is one."),
            pair("format_version", "The format_version of template.curvyo; a newer one is shown but not loaded."),
        ],
        rules: vec![
            format!(
                "{CATALOGUE_FILE} is at most {} with at most {} templates.",
                mib(MAX_INDEX_BYTES as u64),
                grouped(MAX_TEMPLATES as u64)
            ),
            format!("A template file is at most {}.", mib(MAX_TEMPLATE_BYTES)),
            "An entry that breaks a rule is left out; the others are listed.".to_owned(),
            "A file whose size or SHA-256 differs from its entry is not loaded.".to_owned(),
        ],
        private_title: "Private repositories".to_owned(),
        private_steps: vec![
            "At the host, create a read-only token for that one repository; on GitHub: Settings, Developer settings, Fine-grained tokens, only that repository, Contents: Read-only, and an expiry.".to_owned(),
            "In Curvyo: Library tab, Add source…, Git catalogue, enter the address, and paste the token into Access token.".to_owned(),
            "The token is kept in your system's key store and sent only to the catalogue's own address; it is never written to online-sources.toml or shown again.".to_owned(),
            "To change or remove it later, press the key action of the source in the Library tab.".to_owned(),
            "A token that has expired reads \"The token was refused.\"; make a new one and use Change token….".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_library_core::Catalogue;

    #[test]
    fn the_example_is_an_index_this_build_reads_whole() {
        let catalogue = Catalogue::parse(hosting_guide().example.as_bytes()).unwrap();
        assert_eq!(catalogue.templates.len(), 2);
        assert_eq!(
            catalogue.skipped, 0,
            "every rule of the guide holds for its example"
        );
    }

    #[test]
    fn the_guide_states_the_limits_of_this_build_and_the_private_steps() {
        let guide = hosting_guide();
        assert!(guide.rules[0].contains("4 MiB") && guide.rules[0].contains("10,000"));
        assert!(guide.rules[1].contains("64 MiB"));
        assert_eq!(guide.private_title, "Private repositories");
        assert_eq!(guide.private_steps.len(), 5);
        let all = guide.private_steps.join(" ");
        for word in [
            "Contents: Read-only",
            "expiry",
            "Access token",
            "key store",
            "online-sources.toml",
            "Change token",
        ] {
            assert!(all.contains(word), "{word}");
        }
        assert!(!all.contains("must be public"));
        assert!(guide.example_url.ends_with("curvyo/templates"));
        assert!(
            !guide.steps.iter().any(|s| s.contains("script")),
            "no index script exists yet (H12)"
        );
    }
}
