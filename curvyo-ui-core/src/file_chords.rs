//! The chords of the file commands: which key press is New, Open, Save, Save as, Close or Quit on
//! which platform, and the key text the buttons' tooltips show
//! (`specs/0051-sidebar-and-no-menu/` criteria 32 and 34, `adrs.md` decision 7).
//!
//! The table, the match on the typed character, the exceptions of the browser build and the
//! tooltip text live here and nowhere else, so a tooltip and its chord cannot drift apart. The
//! gate (a drag in flight, an open entry chip, a key repeat) is the session's.

/// A command of the File tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCommand {
    /// A new, empty project.
    New,
    /// Open a project file.
    Open,
    /// Save to the project's file, or ask for one when there is none.
    Save,
    /// Always ask for the file to save to.
    SaveAs,
    /// End the project; the result is an empty project, as for New.
    Close,
    /// Leave the application.
    Quit,
}

impl FileCommand {
    /// Every command, in the order of the File tab.
    pub const ALL: [Self; 6] = [
        Self::New,
        Self::Open,
        Self::Save,
        Self::SaveAs,
        Self::Close,
        Self::Quit,
    ];

    /// The name the host and the page use for the command.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Open => "open",
            Self::Save => "save",
            Self::SaveAs => "save-as",
            Self::Close => "close",
            Self::Quit => "quit",
        }
    }

    /// The command called `name`; `None` for an unknown name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }

    /// The words of the tooltip, without the key.
    const fn words(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Open => "Open",
            Self::Save => "Save",
            Self::SaveAs => "Save as",
            Self::Close => "Close",
            Self::Quit => "Quit",
        }
    }

    /// The typed character of the chord and whether Shift is part of it.
    const fn key(self) -> (char, bool) {
        match self {
            Self::New => ('n', false),
            Self::Open => ('o', false),
            Self::Save => ('s', false),
            Self::SaveAs => ('s', true),
            Self::Close => ('w', false),
            Self::Quit => ('q', false),
        }
    }
}

/// Where the editor runs. All four differ in which chords they leave to the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// The desktop build on Linux.
    Linux,
    /// The desktop build on Windows.
    Windows,
    /// The desktop build on macOS.
    MacOs,
    /// The browser build.
    Browser,
}

impl Platform {
    /// The platform called `name` (`"linux"`, `"windows"`, `"macos"`, `"browser"`); `None` for an
    /// unknown name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            "macos" => Self::MacOs,
            "browser" => Self::Browser,
            _ => return None,
        })
    }

    /// Whether the page receives the chord of `command`. Otherwise the browser keeps it (New and
    /// Close would open and close tabs), the window system has another key (Quit on Windows is
    /// Alt+F4) or the application menu owns it (Cmd+Q on macOS).
    const fn page_handles(self, command: FileCommand) -> bool {
        !matches!(
            (self, command),
            (
                Self::Browser,
                FileCommand::New | FileCommand::Close | FileCommand::Quit
            ) | (Self::Windows | Self::MacOs, FileCommand::Quit)
        )
    }

    /// Whether the chord of `command` is shown on a tooltip: every chord the page handles, and
    /// Cmd+Q, which the application menu handles.
    const fn labelled(self, command: FileCommand) -> bool {
        self.page_handles(command) || matches!((self, command), (Self::MacOs, FileCommand::Quit))
    }

    const fn primary(self) -> &'static str {
        match self {
            Self::MacOs => "Cmd",
            _ => "Ctrl",
        }
    }
}

/// The command a key press is, or `None`. `key` is `KeyboardEvent.key`: the typed character, so a
/// German keyboard works; `primary` is Ctrl (Cmd on macOS) and the only modifier besides Shift.
/// Alt, which `AltGr` reports with Ctrl, never makes a chord.
#[must_use]
pub fn file_chord(
    key: &str,
    shift: bool,
    primary: bool,
    alt: bool,
    platform: Platform,
) -> Option<FileCommand> {
    if !primary || alt {
        return None;
    }
    let mut chars = key.chars();
    let typed = chars.next()?.to_ascii_lowercase();
    if chars.next().is_some() {
        return None;
    }
    FileCommand::ALL
        .into_iter()
        .find(|&command| command.key() == (typed, shift) && platform.page_handles(command))
}

/// The key text of a command on `platform`: `"Ctrl+Shift+S"`, `"Cmd+N"`, or `None` where the build
/// has no key for it.
#[must_use]
pub fn chord_label(command: FileCommand, platform: Platform) -> Option<String> {
    if !platform.labelled(command) {
        return None;
    }
    let (key, shift) = command.key();
    let shift = if shift { "Shift+" } else { "" };
    Some(format!(
        "{}+{shift}{}",
        platform.primary(),
        key.to_ascii_uppercase()
    ))
}

/// The tooltip of a button: the words, then the chord in brackets when the build has one
/// (`"Save as (Ctrl+Shift+S)"`, `"New"` in the browser).
#[must_use]
pub fn command_tooltip(command: FileCommand, platform: Platform) -> String {
    match chord_label(command, platform) {
        Some(label) => format!("{} ({label})", command.words()),
        None => command.words().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use FileCommand::{Close, New, Open, Quit, Save, SaveAs};
    use Platform::{Browser, Linux, MacOs, Windows};

    const PLATFORMS: [Platform; 4] = [Linux, Windows, MacOs, Browser];

    /// The table of criterion 32, row by row: a plain press of the key with the primary modifier.
    #[test]
    fn the_chord_table_per_platform() {
        let rows: [(&str, bool, [Option<FileCommand>; 4]); 6] = [
            // key, shift, [Linux, Windows, macOS, Browser]
            ("n", false, [Some(New), Some(New), Some(New), None]),
            ("o", false, [Some(Open), Some(Open), Some(Open), Some(Open)]),
            ("s", false, [Some(Save), Some(Save), Some(Save), Some(Save)]),
            (
                "s",
                true,
                [Some(SaveAs), Some(SaveAs), Some(SaveAs), Some(SaveAs)],
            ),
            ("w", false, [Some(Close), Some(Close), Some(Close), None]),
            ("q", false, [Some(Quit), None, None, None]),
        ];
        for (key, shift, expected) in rows {
            for (platform, want) in PLATFORMS.into_iter().zip(expected) {
                assert_eq!(
                    file_chord(key, shift, true, false, platform),
                    want,
                    "{key} shift {shift} on {platform:?}"
                );
            }
        }
    }

    /// The typed character is matched, not the key's place: a capital letter (Shift held, or Caps
    /// Lock) is the same key, and a key that types something else is not a chord.
    #[test]
    fn the_typed_character_is_matched_case_insensitively() {
        assert_eq!(file_chord("S", true, true, false, Linux), Some(SaveAs));
        assert_eq!(file_chord("S", false, true, false, Linux), Some(Save));
        assert_eq!(file_chord("O", false, true, false, MacOs), Some(Open));
        // QWERTZ: the key in the place of Y types "z", the key in the place of Z types "y".
        assert_eq!(file_chord("y", false, true, false, Linux), None);
        assert_eq!(file_chord("Enter", false, true, false, Linux), None);
        assert_eq!(file_chord("", false, true, false, Linux), None);
        assert_eq!(file_chord("ss", false, true, false, Linux), None);
    }

    /// Without the primary modifier, with Alt (`AltGr`), or with Shift on a key that has no shifted
    /// chord, nothing is a chord.
    #[test]
    fn a_press_without_the_right_modifiers_is_no_chord() {
        for platform in PLATFORMS {
            assert_eq!(file_chord("s", false, false, false, platform), None);
            assert_eq!(file_chord("s", false, true, true, platform), None);
            assert_eq!(file_chord("s", true, true, true, platform), None);
            for key in ["n", "o", "w", "q"] {
                assert_eq!(
                    file_chord(key, true, true, false, platform),
                    None,
                    "Shift+{key} on {platform:?}"
                );
            }
        }
    }

    /// Criterion 34: a tooltip names exactly the chord that works. For every platform and command,
    /// the label parses back to the press that `file_chord` answers with that command; the one
    /// exception is Cmd+Q, which the application menu handles and the page never sees.
    #[test]
    fn a_label_and_its_chord_cannot_differ() {
        for platform in PLATFORMS {
            for command in FileCommand::ALL {
                let chords: Vec<_> = ["n", "o", "s", "w", "q"]
                    .into_iter()
                    .flat_map(|key| [(key, false), (key, true)])
                    .filter(|&(key, shift)| {
                        file_chord(key, shift, true, false, platform) == Some(command)
                    })
                    .map(|(key, shift)| (key.to_string(), shift))
                    .collect();
                match chord_label(command, platform) {
                    Some(label) => {
                        if platform == MacOs && command == Quit {
                            assert_eq!(chords, []);
                            assert_eq!(label, "Cmd+Q");
                            continue;
                        }
                        let primary = if platform == MacOs { "Cmd+" } else { "Ctrl+" };
                        let rest = label.strip_prefix(primary).unwrap();
                        let (shift, key) = rest
                            .strip_prefix("Shift+")
                            .map_or((false, rest), |key| (true, key));
                        assert_eq!(
                            chords,
                            [(key.to_lowercase(), shift)],
                            "{command:?} on {platform:?}"
                        );
                    }
                    None => assert!(chords.is_empty(), "{command:?} on {platform:?}"),
                }
            }
        }
    }

    /// Criterion 34 by example: the words and the key, per platform.
    #[test]
    fn tooltips_name_the_chord_of_the_build() {
        assert_eq!(command_tooltip(New, Linux), "New (Ctrl+N)");
        assert_eq!(command_tooltip(New, MacOs), "New (Cmd+N)");
        assert_eq!(command_tooltip(New, Browser), "New");
        assert_eq!(command_tooltip(SaveAs, Windows), "Save as (Ctrl+Shift+S)");
        assert_eq!(command_tooltip(SaveAs, MacOs), "Save as (Cmd+Shift+S)");
        assert_eq!(command_tooltip(Close, Linux), "Close (Ctrl+W)");
        assert_eq!(command_tooltip(Quit, Linux), "Quit (Ctrl+Q)");
        assert_eq!(command_tooltip(Quit, Windows), "Quit");
        assert_eq!(command_tooltip(Quit, MacOs), "Quit (Cmd+Q)");
        assert_eq!(command_tooltip(Save, Browser), "Save (Ctrl+S)");
    }

    #[test]
    fn names_round_trip() {
        for command in FileCommand::ALL {
            assert_eq!(FileCommand::from_name(command.name()), Some(command));
        }
        assert_eq!(FileCommand::from_name("export"), None);
        for (name, platform) in [
            ("linux", Linux),
            ("windows", Windows),
            ("macos", MacOs),
            ("browser", Browser),
        ] {
            assert_eq!(Platform::from_name(name), Some(platform));
        }
        assert_eq!(Platform::from_name("beos"), None);
    }
}
