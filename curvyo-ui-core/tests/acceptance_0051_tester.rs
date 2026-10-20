//! Independent tester cases for `specs/0051-sidebar-and-no-menu/specification.md`, `curvyo-ui-core`
//! side: the file chord table per platform (criterion 32), the typed-character match (33), the
//! tooltips (34), and the tab list (2, 4). Written from the specification table, not from the code.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_ui_core::{
    FileCommand, PanelTab, Platform, chord_label, command_tooltip, file_chord, tab_entries,
};

const PLATFORMS: [Platform; 4] = [
    Platform::Linux,
    Platform::Windows,
    Platform::MacOs,
    Platform::Browser,
];

/// The table of criterion 32 as the spec states it: (key, shift, command, linux, windows, macos, browser).
const TABLE: [(&str, bool, FileCommand, [bool; 4]); 6] = [
    ("n", false, FileCommand::New, [true, true, true, false]),
    ("o", false, FileCommand::Open, [true, true, true, true]),
    ("s", false, FileCommand::Save, [true, true, true, true]),
    ("s", true, FileCommand::SaveAs, [true, true, true, true]),
    ("w", false, FileCommand::Close, [true, true, true, false]),
    // Ctrl+Q: Linux only as a page chord (macOS: the menu's Quit item, not a page chord).
    ("q", false, FileCommand::Quit, [true, false, false, false]),
];

#[test]
fn c32_chord_table_per_platform_exact() {
    for (key, shift, command, expected) in TABLE {
        for (i, platform) in PLATFORMS.into_iter().enumerate() {
            let got = file_chord(key, shift, true, false, platform);
            let want = expected[i].then_some(command);
            assert_eq!(
                got,
                want,
                "Ctrl{}+{key} on {platform:?}",
                if shift { "+Shift" } else { "" }
            );
        }
    }
}

#[test]
fn c32_nothing_outside_the_table_is_a_chord() {
    // Every printable ASCII key x shift x ctrl x alt x platform: only the table rows answer.
    for c in (0x20u8..0x7f).map(char::from) {
        let key = c.to_string();
        for shift in [false, true] {
            for ctrl in [false, true] {
                for alt in [false, true] {
                    for platform in PLATFORMS {
                        let got = file_chord(&key, shift, ctrl, alt, platform);
                        if let Some(command) = got {
                            let lower = c.to_ascii_lowercase().to_string();
                            let row = TABLE.iter().find(|r| r.2 == command).unwrap();
                            assert_eq!(row.0, lower, "{key:?} gave {command:?}");
                            assert!(
                                ctrl && !alt,
                                "{key:?} ctrl={ctrl} alt={alt} gave {command:?}"
                            );
                            assert_eq!(row.1, shift, "{key:?} shift={shift} gave {command:?}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn c32_no_chord_without_the_primary_modifier_or_with_alt() {
    for platform in PLATFORMS {
        for (key, shift, _, _) in TABLE {
            assert_eq!(file_chord(key, shift, false, false, platform), None);
            // AltGr reports Ctrl+Alt: never a chord (a German keyboard's AltGr+Q is "@").
            assert_eq!(file_chord(key, shift, true, true, platform), None);
        }
    }
}

#[test]
fn c33_matched_by_typed_character_case_insensitive_and_not_position() {
    // QWERTZ: the key that types "s" is "s"; shift reports "S".
    for platform in [Platform::Linux, Platform::Windows, Platform::MacOs] {
        assert_eq!(
            file_chord("s", false, true, false, platform),
            Some(FileCommand::Save)
        );
        assert_eq!(
            file_chord("S", true, true, false, platform),
            Some(FileCommand::SaveAs)
        );
        // Caps Lock: Ctrl+S reports "S" with shift false; still Save.
        assert_eq!(
            file_chord("S", false, true, false, platform),
            Some(FileCommand::Save)
        );
        // Z / Y are not file chords (undo keys stay free, criterion 35)
        for k in ["z", "Z", "y", "Y"] {
            assert_eq!(file_chord(k, false, true, false, platform), None);
            assert_eq!(file_chord(k, true, true, false, platform), None);
        }
    }
}

#[test]
fn c32_shift_variants_of_the_other_chords_are_not_chords() {
    for platform in PLATFORMS {
        for k in ["n", "o", "w", "q", "N", "O", "W", "Q"] {
            assert_eq!(
                file_chord(k, true, true, false, platform),
                None,
                "Ctrl+Shift+{k} on {platform:?}"
            );
        }
    }
}

#[test]
fn c32_hostile_key_strings() {
    for platform in PLATFORMS {
        for k in [
            "",
            "Save",
            "ss",
            "s ",
            " s",
            "Dead",
            "Unidentified",
            "Control",
            "Shift",
            "\u{0}",
            "ſ",
            "ｓ",
            "s\u{301}",
            "\u{1F600}",
            "S\u{200B}",
            "Process",
            "Enter",
            "F5",
        ] {
            assert_eq!(file_chord(k, false, true, false, platform), None, "{k:?}");
            assert_eq!(file_chord(k, true, true, false, platform), None, "{k:?}");
        }
    }
    // a huge string must not allocate/loop badly
    let huge = "s".repeat(1_000_000);
    assert_eq!(file_chord(&huge, false, true, false, Platform::Linux), None);
}

#[test]
fn c34_tooltips_name_the_chord_that_works_on_the_build() {
    use FileCommand::{Close, New, Open, Quit, Save, SaveAs};
    let linux = [
        (New, "New (Ctrl+N)"),
        (Open, "Open (Ctrl+O)"),
        (Save, "Save (Ctrl+S)"),
        (SaveAs, "Save as (Ctrl+Shift+S)"),
        (Close, "Close (Ctrl+W)"),
        (Quit, "Quit (Ctrl+Q)"),
    ];
    for (c, t) in linux {
        assert_eq!(command_tooltip(c, Platform::Linux), t);
    }
    assert_eq!(command_tooltip(New, Platform::MacOs), "New (Cmd+N)");
    assert_eq!(
        command_tooltip(SaveAs, Platform::MacOs),
        "Save as (Cmd+Shift+S)"
    );
    assert_eq!(command_tooltip(Close, Platform::MacOs), "Close (Cmd+W)");
    // browser: New, Close, Quit have no chord
    assert_eq!(command_tooltip(New, Platform::Browser), "New");
    assert_eq!(command_tooltip(Close, Platform::Browser), "Close");
    assert_eq!(command_tooltip(Open, Platform::Browser), "Open (Ctrl+O)");
    assert_eq!(command_tooltip(Save, Platform::Browser), "Save (Ctrl+S)");
    // Windows has no Ctrl+Q
    assert_eq!(command_tooltip(Quit, Platform::Windows), "Quit");
}

#[test]
fn c34_tooltip_and_matcher_cannot_differ() {
    // Every label that exists must be matched by the matcher to the same command, and every
    // chord the matcher answers must be labelled.
    for platform in PLATFORMS {
        for command in FileCommand::ALL {
            let label = chord_label(command, platform);
            let matched_key = TABLE.iter().find(|r| r.2 == command).unwrap();
            let page_chord = file_chord(matched_key.0, matched_key.1, true, false, platform);
            if page_chord.is_some() {
                let label = label.clone().unwrap_or_else(|| {
                    panic!("{command:?} on {platform:?} has a chord but no label")
                });
                let (prefix, key) = label.rsplit_once('+').unwrap();
                assert_eq!(key.to_ascii_lowercase(), matched_key.0);
                assert_eq!(prefix.contains("Shift"), matched_key.1);
                assert!(prefix.starts_with(if platform == Platform::MacOs {
                    "Cmd"
                } else {
                    "Ctrl"
                }));
            }
            if label.is_some() {
                // labelled but not a page chord: only macOS Quit (application menu) is allowed
                assert!(
                    page_chord.is_some()
                        || (platform == Platform::MacOs && command == FileCommand::Quit),
                    "{command:?} on {platform:?} is labelled but no chord"
                );
            }
        }
    }
}

#[test]
fn command_names_round_trip_and_reject_garbage() {
    for c in FileCommand::ALL {
        assert_eq!(FileCommand::from_name(c.name()), Some(c));
    }
    assert_eq!(FileCommand::ALL.len(), 6);
    // the page (`FileCommandName` in useProject.ts) uses exactly these names
    let names: Vec<_> = FileCommand::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(names, ["new", "open", "save", "save-as", "close", "quit"]);
    for bad in ["", "NEW", "new ", "save_as", "saveas", "export", "\u{0}"] {
        assert_eq!(FileCommand::from_name(bad), None, "{bad:?}");
    }
    assert_eq!(Platform::from_name("linux"), Some(Platform::Linux));
    assert_eq!(Platform::from_name("macos"), Some(Platform::MacOs));
    assert_eq!(Platform::from_name("MacOS"), None);
    assert_eq!(Platform::from_name("darwin"), None);
}

#[test]
fn c45_tabs_found_by_name_order_file_document_style_history() {
    for non_empty in [false, true] {
        for active in [
            PanelTab::File,
            PanelTab::Document,
            PanelTab::Style,
            PanelTab::History,
        ] {
            let entries = tab_entries(active, non_empty);
            let names: Vec<_> = entries.iter().map(|e| e.name).collect();
            assert_eq!(
                names,
                ["file", "document", "style", "layers", "history", "fonts"]
            );
            // exactly one selected; it is the active tab
            let sel: Vec<_> = entries
                .iter()
                .filter(|e| e.selected)
                .map(|e| e.name)
                .collect();
            assert_eq!(sel, [active.name()]);
            // only Style can be dimmed
            for e in &entries {
                if e.name != "style" {
                    assert!(e.enabled, "{} must never be dimmed", e.name);
                }
            }
            let style = entries.iter().find(|e| e.name == "style").unwrap();
            assert_eq!(style.enabled, non_empty);
            let file = entries.iter().find(|e| e.name == "file").unwrap();
            assert_eq!(file.label, "File");
            assert_eq!(file.tooltip, "File");
        }
    }
    for t in [
        PanelTab::File,
        PanelTab::Document,
        PanelTab::Style,
        PanelTab::Layers,
        PanelTab::History,
    ] {
        assert_eq!(PanelTab::from_name(t.name()), Some(t));
    }
    assert_eq!(
        PanelTab::from_name("layers"),
        Some(PanelTab::Layers),
        "Layers is built since 0050 shipped"
    );
    assert_eq!(PanelTab::from_name("File"), None);
}

#[test]
fn c14_file_tab_never_switches_by_itself() {
    use curvyo_ui_core::PanelTabs;
    let mut tabs = PanelTabs::new(false);
    assert!(tabs.press(PanelTab::File, false));
    for step in [true, false, true, true, false, false, true] {
        tabs.observe(step);
        assert_eq!(tabs.active(), PanelTab::File, "scope became {step}");
    }
    // pressing the dimmed Style tab with nothing in scope fails and keeps File
    tabs.observe(false);
    assert!(!tabs.press(PanelTab::Style, false));
    assert_eq!(tabs.active(), PanelTab::File);
    // starting with a selection: default is Style, not File
    assert_eq!(PanelTabs::new(true).active(), PanelTab::Style);
    assert_eq!(PanelTabs::new(false).active(), PanelTab::Document);
}
