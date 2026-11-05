//! Tester acceptance tests for `specs/0097-membership-lifecycle` wording (criteria 10, 18, 22
//! and 34): no text of the share, leave, removal and add-back flows says "revoked", "lost access"
//! or that someone "can no longer read" / "cannot read"; the leave and From now on texts say what
//! the person keeps.

use curvyo_ui_core::membership_return_text as back;
use curvyo_ui_core::membership_text as text;
use curvyo_ui_core::membership_text::AdminChange;

const FORBIDDEN: [&str; 5] = [
    "revoked",
    "lost access",
    "can no longer read",
    "cannot read",
    "no longer has access",
];

fn check(all: &[String]) {
    for t in all {
        // What the server cannot read is the encryption promise, not a person's access.
        let lower = t.to_lowercase().replace("the server cannot read", "");
        for word in FORBIDDEN {
            assert!(!lower.contains(word), "{word:?} in {t:?}");
        }
    }
}

/// Criterion 34 by the texts the flows show, with sample names.
#[test]
fn no_membership_text_says_access_was_revoked() {
    let names = ["Ben", "A very long device name that will be cut somewhere"];
    let mut all: Vec<String> = Vec::new();
    for n in names {
        all.extend([
            text::leave_document_confirm(n),
            text::leave_workspace_confirm(n, 1, &[]),
            text::leave_workspace_confirm(n, 17, &["B2".to_owned(), "B3".to_owned()]),
            text::left_notice(n, true),
            text::left_notice(n, false),
            text::you_left_line(n),
            text::make_member_confirm(n),
            text::admin_removal_confirm_text(n),
            text::remove_name(n),
            text::not_applied_text(n, true),
            text::not_applied_text(n, false),
            text::mutual_removal_text(&[n.to_owned()], true),
            text::mutual_removal_text(&[n.to_owned()], false),
            text::rotation_pending_text(&[(n.to_owned(), true)], true),
            text::rotation_pending_text(&[(n.to_owned(), false)], false),
            text::make_admin_confirm(n, None),
            back::full_history_line(n),
            back::from_now_on_line(n),
            back::add_back_common(n),
            back::workspace_add_back(n, "Box job", 3),
            back::added_back_notice(n),
            back::added_back_prompt(n, "Poster"),
            back::keep_consequence(n),
            back::ended_heading(n, true),
            back::ended_heading(n, false),
        ]);
        for change in [
            AdminChange::NowAdmin,
            AdminChange::NoLongerAdmin,
            AdminChange::Left,
            AdminChange::Removed,
        ] {
            all.push(text::admin_change_text(change, n));
        }
    }
    all.extend(
        [
            text::ONLY_ADMIN,
            text::GROUP_ONLY_ADMIN,
            text::PERSON_ONLY_ADMIN,
            text::PERSON_LEAVE_REFUSED,
            text::ONE_ADMIN_LINE,
            text::ONLY_OWN_ADMINS_LINE,
            text::STEP_DOWN_CONFIRM,
            back::NOT_ADDED_BACK_YET,
            back::CHECK_UNVERIFIED,
            back::CHECK_NOT_FOUND,
            back::MERGE_SENTENCE,
        ]
        .map(str::to_owned),
    );
    if let Some(w) = text::frozen_room_warning(&["A".to_owned(), "C".to_owned()], None) {
        all.push(w);
    }
    check(&all);
}

/// Criterion 34 over every string literal of the flows' text modules, so a text added later
/// without a sample here is covered too.
#[test]
fn no_text_module_literal_says_access_was_revoked() {
    for source in [
        include_str!("../src/membership_text.rs"),
        include_str!("../src/membership_return_text.rs"),
        include_str!("../src/share_text.rs"),
        include_str!("../src/workspace_share_text.rs"),
    ] {
        // The modules' own tests list the forbidden words; only the texts before them count.
        let texts = source.split("#[cfg(test)]").next().unwrap_or(source);
        let literals: Vec<String> = texts
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .flat_map(|l| l.split('"').skip(1).step_by(2).map(str::to_owned))
            .collect();
        check(&literals);
    }
}

/// Criteria 10 and 18/22: the leave confirm says what stays and that the key is still held;
/// From now on says the app withholds keys, never that the person cannot read, and that the
/// returning person keeps what it had.
#[test]
fn leave_and_from_now_on_say_what_is_kept() {
    let leave = text::leave_document_confirm("Poster");
    assert!(leave.contains("copy stays"), "{leave}");
    assert!(leave.contains("still holds the key"), "{leave}");
    let leave = text::leave_workspace_confirm("Box job", 3, &[]);
    assert!(leave.contains("still holds the key"), "{leave}");
    let from_now = back::from_now_on_line("Ben");
    assert!(
        from_now.contains("The app does not give Ben the keys"),
        "{from_now}"
    );
    assert!(
        from_now.contains("Ben still holds the keys Ben had before"),
        "{from_now}"
    );
    let left = text::left_notice("Poster", false);
    assert!(left.contains("Your copy stays"), "{left}");
}
