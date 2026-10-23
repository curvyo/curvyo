//! The maker's font library as the page sees it (`specs/0075-font-library` Parts A to C, tags):
//! the library over what is known of each listed family, viewed as JSON. Plain Rust, so the host
//! tests it; `wasm_font_library` only binds it (ADR 0001 §5: strings and scalars).

use std::collections::{BTreeMap, BTreeSet};

use curvyo_text_core::{DeclaredClass, declared_class};
use curvyo_ui_core::{
    BuiltInTag, CheckResult, FamilyFacts, FontLibrary, FontLibraryFileError, MemberSource,
    TagEntry, TagNameError, family_key, read_font_library, write_font_library,
};
use serde_json::{Value, json};

/// The library and the facts of the listed families, by family key.
#[derive(Debug, Default)]
pub struct FontLibraryGlue {
    library: FontLibrary,
    facts: BTreeMap<String, FamilyFacts>,
}

/// The 12 class bytes from 24 hex digits.
fn os2_from_hex(hex: &str) -> Option<[u8; 12]> {
    let bytes: Vec<u8> = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect();
    bytes.try_into().ok()
}

const fn class_code(class: DeclaredClass) -> &'static str {
    match class {
        DeclaredClass::Serif => "serif",
        DeclaredClass::Sans => "sans",
        DeclaredClass::Script => "script",
    }
}

fn source_code(source: MemberSource) -> String {
    match source {
        MemberSource::ByYou => "you".to_owned(),
        MemberSource::Shipped => "shipped".to_owned(),
        MemberSource::Declared(class) => format!("declared:{}", class_code(class)),
        MemberSource::Detected(pieces) => format!("detected:{pieces}"),
        MemberSource::SingleLine => "detected:single_line".to_owned(),
        MemberSource::Nothing => String::new(),
    }
}

fn name_error(error: &TagNameError) -> Value {
    match error {
        TagNameError::Empty => json!({ "error": "empty" }),
        TagNameError::TooLong => json!({ "error": "long" }),
        TagNameError::Reserved => json!({ "error": "reserved" }),
        TagNameError::Taken(name) => json!({ "error": "taken", "taken": name }),
        TagNameError::TooMany => json!({ "error": "many" }),
    }
}

impl FontLibraryGlue {
    /// Reads the file text: `"ok"`, `"newer"` or `"broken"`. The library is the default unless
    /// the text was read (criterion 12).
    pub fn load(&mut self, text: &str) -> &'static str {
        match read_font_library(text) {
            Ok(library) => {
                self.library = library;
                "ok"
            }
            Err(FontLibraryFileError::Newer) => "newer",
            Err(FontLibraryFileError::Broken(_)) => "broken",
        }
    }

    /// The canonical file text.
    #[must_use]
    pub fn text(&self) -> String {
        write_font_library(&self.library)
    }

    /// Replaces what is known of the listed families, from JSON
    /// `[{"name", "os2": hex|null, "shipped": [ids]|null, "pieces": n|-1|null,
    /// "single_line": bool?}]`.
    pub fn set_families(&mut self, json: &str) {
        let Ok(Value::Array(families)) = serde_json::from_str::<Value>(json) else {
            return;
        };
        self.facts.clear();
        for family in families {
            let key = family_key(family["name"].as_str().unwrap_or_default());
            let shipped = family["shipped"].as_array().map(|ids| {
                ids.iter()
                    .filter_map(|id| BuiltInTag::from_id(id.as_str()?))
                    .collect::<BTreeSet<_>>()
            });
            let facts = FamilyFacts {
                declared: family["os2"]
                    .as_str()
                    .and_then(os2_from_hex)
                    .and_then(declared_class),
                check: check_of(family["pieces"].as_i64()),
                shipped,
                single_line: family["single_line"].as_bool().unwrap_or(false),
            };
            self.facts.insert(key, facts);
        }
    }

    /// Sets the check result of the family `name` (`-1` unreadable).
    pub fn set_check(&mut self, name: &str, pieces: i64) {
        if let Some(facts) = self.facts.get_mut(&family_key(name)) {
            facts.check = check_of(Some(pieces));
        }
    }

    fn facts(&self, key: &str) -> FamilyFacts {
        self.facts.get(key).cloned().unwrap_or_default()
    }

    /// The strip, the list and the tag lines for the listed families `names` (JSON array, list
    /// order): the chosen cell, every tag with the positions of its members in `names`, the
    /// starred positions, and per family its tags in chip order with their sources.
    #[must_use]
    pub fn view(&self, names: &str) -> String {
        let names: Vec<String> = serde_json::from_str(names).unwrap_or_default();
        let keys: Vec<String> = names.iter().map(|name| family_key(name)).collect();
        let facts: Vec<FamilyFacts> = keys.iter().map(|key| self.facts(key)).collect();
        let row_tags: Vec<Vec<(TagEntry, MemberSource)>> = keys
            .iter()
            .zip(&facts)
            .map(|(key, facts)| self.library.tags_of(key, facts))
            .collect();
        let tags: Vec<Value> = self
            .library
            .all_tags()
            .into_iter()
            .map(|tag| {
                let members: Vec<usize> = row_tags
                    .iter()
                    .enumerate()
                    .filter(|(_, carried)| carried.iter().any(|(t, _)| t.id == tag.id))
                    .map(|(at, _)| at)
                    .collect();
                json!({ "id": tag.id, "name": tag.name, "builtin": tag.builtin, "members": members })
            })
            .collect();
        let favourites: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, key)| self.library.is_favourite(key))
            .map(|(at, _)| at)
            .collect();
        let rows: Vec<Vec<Value>> = row_tags
            .into_iter()
            .map(|carried| {
                carried
                    .into_iter()
                    .map(|(tag, source)| {
                        let source = if tag.builtin {
                            source_code(source)
                        } else {
                            String::new()
                        };
                        json!({ "id": tag.id, "name": tag.name, "source": source })
                    })
                    .collect()
            })
            .collect();
        json!({
            "chosen": self.library.chosen(),
            "tags": tags,
            "favourites": favourites,
            "rows": rows,
        })
        .to_string()
    }

    /// Stars or unstars `name`; returns the new state.
    pub fn toggle_favourite(&mut self, name: &str) -> bool {
        self.library.toggle_favourite(&family_key(name))
    }

    /// The add field's Enter: `{"changed"}` or `{"error"}`.
    pub fn add_tag(&mut self, name: &str, tag: &str) -> String {
        let key = family_key(name);
        let facts = self.facts(&key);
        match self.library.add_tag(&key, tag, &facts) {
            Ok(changed) => json!({ "changed": changed }),
            Err(error) => name_error(&error),
        }
        .to_string()
    }

    /// A tag's x; returns whether anything changed.
    pub fn remove_tag(&mut self, name: &str, id: &str) -> bool {
        let key = family_key(name);
        let facts = self.facts(&key);
        self.library.remove_tag(&key, id, &facts)
    }

    /// The add field's completions for `prefix`, as a JSON array of names in chip order.
    #[must_use]
    pub fn completions(&self, name: &str, prefix: &str) -> String {
        let key = family_key(name);
        let facts = self.facts(&key);
        json!(self.library.completions(&key, prefix, &facts)).to_string()
    }

    /// Rename in the Tags list: `{}` or `{"error", "taken"?}`.
    pub fn rename_tag(&mut self, id: &str, name: &str) -> String {
        match self.library.rename_tag(id, name) {
            Ok(()) => json!({}),
            Err(error) => name_error(&error),
        }
        .to_string()
    }

    /// Delete in the Tags list.
    pub fn delete_tag(&mut self, id: &str) -> bool {
        self.library.delete_tag(id)
    }

    /// A strip press.
    pub fn choose(&mut self, id: &str) -> bool {
        self.library.choose(id)
    }
}

fn check_of(pieces: Option<i64>) -> Option<CheckResult> {
    match pieces? {
        n if n < 0 => Some(CheckResult::Unreadable),
        n => Some(CheckResult::Pieces(
            usize::try_from(n).unwrap_or(usize::MAX),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criteria 2, 9 and 10 through the page's view: a declared Script family carries Script and
    /// Cuttable, a star puts it in Favourites, an own tag joins its tag line, a removed built-in
    /// tag leaves it.
    #[test]
    fn the_view_follows_the_library() {
        let mut glue = FontLibraryGlue::default();
        // PANOSE Latin Hand Written: Script.
        let os2 = "000003000000000000000000";
        glue.set_families(&format!(
            r#"[{{"name":"Lobster","os2":"{os2}","shipped":null,"pieces":1}},
               {{"name":"Source Sans 3","os2":null,"shipped":["sans"],"pieces":8}}]"#
        ));
        glue.toggle_favourite("Lobster");
        assert_eq!(glue.add_tag("Lobster", "Wedding"), r#"{"changed":true}"#);
        assert!(glue.remove_tag("Lobster", "script"));
        let view: Value =
            serde_json::from_str(&glue.view(r#"["Lobster","Source Sans 3"]"#)).unwrap();
        let members = |id: &str| {
            view["tags"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["id"] == id)
                .unwrap()["members"]
                .clone()
        };
        assert_eq!(members("cuttable"), json!([0]));
        assert_eq!(members("sans"), json!([1]));
        assert_eq!(members("Wedding"), json!([0]));
        assert_eq!(view["favourites"], json!([0]));
        assert_eq!(
            view["rows"][0],
            json!([
                { "id": "cuttable", "name": "Cuttable", "source": "detected:1" },
                { "id": "Wedding", "name": "Wedding", "source": "" },
            ])
        );
        assert_eq!(
            glue.completions("Lobster", "s"),
            r#"["Single line","Script","Sans","Serif"]"#
        );
    }
}
