//! Acceptance tests of `specs/0136-autosave-and-restore` on the recovery folder, written from
//! `specification.md` before the implementation was read. Black box: public API only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use curvyo_storage_io::RecoveryFolder;

const KEY: &str = "0123456789abcdef0123456789abcdef";
const OTHER: &str = "fedcba9876543210fedcba9876543210";

fn folder() -> (tempfile::TempDir, RecoveryFolder) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("recovery");
    (dir, RecoveryFolder::new(root))
}

// Criterion 9: the folder is the one the caller passes in; the app never writes beside a project.
#[test]
fn c09_the_folder_reports_where_the_copies_are_kept() {
    let (dir, f) = folder();
    assert_eq!(f.path(), dir.path().join("recovery"));
}

// Criterion 2: a new copy replaces the old one only when complete; a failed write leaves the
// previous copy intact.
#[test]
fn c02_a_failed_write_leaves_the_previous_copy_intact() {
    let (_dir, f) = folder();
    f.write_copy(KEY, b"previous copy", b"{\"record\":1}")
        .unwrap();

    // Make the copy's folder read-only so the next write cannot replace it.
    let copy_dir = f.path().join(KEY);
    let mut perms = fs::metadata(&copy_dir).unwrap().permissions();
    perms.set_mode(0o500);
    fs::set_permissions(&copy_dir, perms.clone()).unwrap();

    let result = f.write_copy(
        KEY,
        b"newer copy that must not half replace",
        b"{\"record\":2}",
    );

    perms.set_mode(0o700);
    fs::set_permissions(&copy_dir, perms).unwrap();

    assert!(
        result.is_err(),
        "the write into a read-only folder must fail"
    );
    assert_eq!(f.read_copy(KEY).unwrap(), b"previous copy".to_vec());
}

#[test]
fn c02_a_complete_write_replaces_the_copy_whole() {
    let (_dir, f) = folder();
    f.write_copy(KEY, b"a much longer first copy", b"{\"record\":1}")
        .unwrap();
    f.write_copy(KEY, b"short", b"{\"record\":2}").unwrap();
    assert_eq!(f.read_copy(KEY).unwrap(), b"short".to_vec());
}

// Criterion 11: a copy that a running session holds is listed as locked, and a second session
// cannot take it.
#[test]
fn c11_a_running_session_is_listed_locked_and_a_second_begin_is_refused() {
    let (_dir, f) = folder();
    f.write_copy(KEY, b"copy", b"{}").unwrap();
    let lock = f.begin(KEY).unwrap();
    let listed = f.list();
    let mine = listed
        .iter()
        .find(|c| c.key == KEY)
        .expect("the copy is listed");
    assert!(mine.locked, "a running session's copy is listed as locked");
    assert!(
        f.begin(KEY).is_err(),
        "a second session cannot take the same key"
    );
    drop(lock);
    let listed = f.list();
    let mine = listed.iter().find(|c| c.key == KEY).expect("still listed");
    assert!(!mine.locked, "after the session ends the copy is offerable");
}

// Criterion 7 (storage side): a truncated record is listed with its bytes, so the page can name
// it as damaged; the copy itself is still listed.
#[test]
fn c07_a_damaged_record_is_still_listed_and_its_bytes_come_back() {
    let (_dir, f) = folder();
    let truncated = b"{\"format_version\":1,\"key\":\"0123";
    f.write_copy(KEY, b"copy", truncated).unwrap();
    let listed = f.list();
    let mine = listed
        .iter()
        .find(|c| c.key == KEY)
        .expect("the damaged copy is listed");
    assert_eq!(mine.record.as_deref(), Some(&truncated[..]));
}

// Criterion 7: Discard works on a damaged copy and touches nothing else.
#[test]
fn c07_discard_removes_a_damaged_copy_and_leaves_the_others() {
    let (_dir, f) = folder();
    f.write_copy(KEY, b"copy", b"{\"format_version\":1,")
        .unwrap();
    f.write_copy(OTHER, b"other", b"{}").unwrap();
    f.delete_all(KEY).unwrap();
    let keys: Vec<String> = f.list().into_iter().map(|c| c.key).collect();
    assert_eq!(keys, vec![OTHER.to_owned()]);
    assert_eq!(f.read_copy(OTHER).unwrap(), b"other".to_vec());
}

// Criterion 3: Save or a clean Close deletes the copy; a second delete is harmless.
#[test]
fn c03_deleting_the_copy_removes_it_from_the_listing() {
    let (_dir, f) = folder();
    f.write_copy(KEY, b"copy", b"{}").unwrap();
    f.delete_copy(KEY).unwrap();
    assert!(f.list().iter().all(|c| c.key != KEY));
}

// Criterion 1 (worst case) and 12: the bytes a copy takes are reported for the 200 MB rule.
#[test]
fn c12_the_listing_reports_the_bytes_of_the_copy() {
    let (_dir, f) = folder();
    f.write_copy(KEY, &[0_u8; 1_000], b"{}").unwrap();
    let listed = f.list();
    let mine = listed.iter().find(|c| c.key == KEY).expect("listed");
    assert!(mine.bytes >= 1_000, "the copy's bytes are counted");
}

// Keys that the app does not make are refused (no path escapes the folder).
#[test]
fn c09_a_key_that_is_not_thirty_two_hex_digits_is_refused() {
    let (_dir, f) = folder();
    assert!(f.write_copy("../escape", b"x", b"{}").is_err());
    assert!(
        f.write_copy("not-hex-not-hex-not-hex-not-hex!", b"x", b"{}")
            .is_err()
    );
}
