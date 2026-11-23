//! Acceptance tests of `specs/0136-autosave-and-restore`, written from `specification.md` and
//! `ux-notes.md` before the implementation was read. Black box: public API only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_ui_core::{
    Due, FILE_CHANGED_LINE, INTERVAL_MS, LocalTime, PruneCandidate, RECOVERY_BLOCK_HEADING,
    RECOVERY_FORMAT, RECOVERY_MAX_BYTES, RECOVERY_MAX_COPIES, RecordError, RecoveryBase,
    RecoveryRecord, RecoverySchedule, Unrestorable, decode_record, encode_record, help_line,
    pages_text, prune_plan, row_line_two, time_text,
};

const MIN: u64 = 60_000;

/// Drives one write the way the page does: begin, then written.
fn write_at(s: &mut RecoverySchedule, now: u64) {
    s.begin();
    s.written(now);
}

// Criterion 1: the one rule, with a fake clock. The interval is a constant of 5 minutes.
#[test]
fn c01_the_interval_is_five_minutes_as_a_constant() {
    assert_eq!(INTERVAL_MS, 5 * MIN);
}

#[test]
fn c01_first_step_is_written_at_once() {
    let mut s = RecoverySchedule::new();
    s.step();
    assert_eq!(s.due(0, true), Due::Write);
}

#[test]
fn c01_no_step_since_the_last_write_writes_nothing() {
    let mut s = RecoverySchedule::new();
    s.step();
    write_at(&mut s, 0);
    assert_eq!(s.due(10 * MIN, true), Due::Nothing);
}

#[test]
fn c01_step_within_five_minutes_of_a_write_waits_and_is_not_written_one_by_one() {
    let mut s = RecoverySchedule::new();
    s.step();
    write_at(&mut s, 0);
    s.step();
    assert_eq!(
        s.due(10_000, true),
        Due::Nothing,
        "10 s after a write: no write yet"
    );
    s.step();
    assert_eq!(
        s.due(20_000, true),
        Due::Nothing,
        "a third step: still no write"
    );
}

#[test]
fn c01_exactly_one_trailing_write_at_the_end_of_the_interval() {
    let mut s = RecoverySchedule::new();
    s.step();
    write_at(&mut s, 0);
    s.step();
    s.step();
    assert_eq!(
        s.due(INTERVAL_MS - 1, true),
        Due::Nothing,
        "4:59.999 is not yet due"
    );
    assert_eq!(
        s.due(INTERVAL_MS, true),
        Due::Write,
        "at 5:00 the trailing write is due"
    );
    write_at(&mut s, INTERVAL_MS);
    assert_eq!(
        s.due(2 * INTERVAL_MS, true),
        Due::Nothing,
        "no second trailing write"
    );
}

#[test]
fn c01_no_trailing_write_when_no_step_came_in_between() {
    let mut s = RecoverySchedule::new();
    s.step();
    write_at(&mut s, 0);
    assert_eq!(s.due(INTERVAL_MS, true), Due::Nothing);
    assert_eq!(s.due(3 * INTERVAL_MS, true), Due::Nothing);
}

#[test]
fn c01_a_step_five_minutes_or_more_after_the_last_write_is_written_at_once() {
    let mut s = RecoverySchedule::new();
    s.step();
    write_at(&mut s, 0);
    s.step();
    assert_eq!(s.due(INTERVAL_MS, true), Due::Write);
}

// Criterion 8: a clean project is never recoverable.
#[test]
fn c08_a_clean_project_with_no_copy_is_never_written() {
    let mut s = RecoverySchedule::new();
    assert_eq!(s.due(0, false), Due::Nothing);
    s.step();
    assert_eq!(
        s.due(0, false),
        Due::Nothing,
        "a step that leaves the project clean"
    );
}

// Criterion 3 and ux 9: a clean project that holds a copy loses it (undo back to the saved state).
#[test]
fn c03_a_clean_project_with_a_copy_deletes_it() {
    let mut s = RecoverySchedule::new();
    s.step();
    write_at(&mut s, 0);
    s.step();
    assert_eq!(s.due(INTERVAL_MS, false), Due::DeleteCopy);
}

// Criterion 7: a damaged record is "Cannot be restored: damaged"; a newer format is refused.
#[test]
fn c07_a_truncated_record_is_damaged() {
    let bytes = encode_record(&sample_record(Some("Poster.curvyo")));
    let truncated = &bytes[..bytes.len() / 2];
    assert_eq!(decode_record(truncated), Err(RecordError::Damaged));
}

#[test]
fn c07_a_record_of_a_newer_format_is_refused_as_newer() {
    let mut record = sample_record(None);
    record.format_version = RECOVERY_FORMAT + 1;
    let bytes = encode_record(&record);
    assert_eq!(decode_record(&bytes), Err(RecordError::Newer));
}

#[test]
fn c07_the_record_round_trips_with_and_without_a_name_and_base() {
    let named = sample_record(Some("Poster.curvyo"));
    assert_eq!(decode_record(&encode_record(&named)).unwrap(), named);
    let untitled = sample_record(None);
    assert_eq!(decode_record(&encode_record(&untitled)).unwrap(), untitled);
}

#[test]
fn c07_damaged_row_line_two_has_the_ux_words() {
    let at = local(2026, 11, 23, 12, 3);
    assert_eq!(
        row_line_two(at, at, 3, Some(Unrestorable::Damaged)),
        "Cannot be restored: damaged."
    );
    assert_eq!(
        row_line_two(at, at, 3, Some(Unrestorable::Newer)),
        "Cannot be restored: made by a newer Curvyo."
    );
}

// Criterion 12: at most 20 copies and 200 MB beyond the newest; the newest is always kept.
// The test reads the spec text: "at most 20 copies ... beyond the newest copy".
#[test]
fn c12_constants_are_twenty_copies_and_two_hundred_megabytes() {
    assert_eq!(RECOVERY_MAX_COPIES, 20);
    assert_eq!(RECOVERY_MAX_BYTES, 200_000_000);
}

#[test]
fn c12_twenty_beyond_the_newest_stay_and_the_twenty_first_beyond_goes() {
    // 21 copies in total: newest plus 20 beyond it, nothing deleted.
    let twenty_one: Vec<PruneCandidate> = (0..21).map(|i| cand(i, 1_000, false)).collect();
    assert!(
        prune_plan(&twenty_one).is_empty(),
        "newest plus 20 beyond all stay"
    );
    // 22 copies: 21 beyond the newest, the oldest goes.
    let twenty_two: Vec<PruneCandidate> = (0..22).map(|i| cand(i, 1_000, false)).collect();
    assert_eq!(
        prune_plan(&twenty_two),
        vec!["key-0".to_owned()],
        "the oldest goes"
    );
}

#[test]
fn c12_the_newest_copy_is_kept_whatever_its_size() {
    // The newest is 900 MB: kept. Beyond it, 10 bytes fit; 300 MB exceeds the 200 MB and goes.
    let small_beyond = vec![cand(9, 900_000_000, false), cand(1, 10, false)];
    assert_eq!(prune_plan(&small_beyond), Vec::<String>::new());
    let big_beyond = vec![cand(9, 900_000_000, false), cand(1, 300_000_000, false)];
    assert_eq!(
        prune_plan(&big_beyond),
        vec!["key-1".to_owned()],
        "the newest stays; the copy beyond it goes past 200 MB"
    );
}

#[test]
fn c12_beyond_the_newest_bytes_total_at_most_two_hundred_megabytes() {
    // Newest is 1 byte; beyond it 100 MB + 100 MB = exactly 200 MB stays, one byte more goes.
    let exact = vec![
        cand(3, 1, false),
        cand(2, 100_000_000, false),
        cand(1, 100_000_000, false),
    ];
    assert!(
        prune_plan(&exact).is_empty(),
        "exactly 200 MB beyond the newest stays"
    );
    let one_over = vec![
        cand(3, 1, false),
        cand(2, 100_000_000, false),
        cand(1, 100_000_001, false),
    ];
    assert_eq!(prune_plan(&one_over), vec!["key-1".to_owned()]);
}

#[test]
fn c12_a_locked_copy_is_never_deleted() {
    let copies = vec![cand(3, 1, false), cand(2, 1, true), cand(1, 1, false)];
    assert_eq!(prune_plan(&copies), Vec::<String>::new());
}

// Criterion 9 and ux 3: the help line names the place, and only then.
#[test]
fn c09_the_help_line_names_the_path_or_the_browser() {
    assert_eq!(
        help_line(Some("/home/marc/.local/share/curvyo/recovery")),
        "Kept in /home/marc/.local/share/curvyo/recovery"
    );
    assert_eq!(help_line(None), "Kept in this browser");
}

// Criterion 4 and ux 4: the block heading, the row line two and the third line.
#[test]
fn c04_the_block_heading_and_the_changed_line_have_the_ux_words() {
    assert_eq!(RECOVERY_BLOCK_HEADING, "Unsaved work from a crash");
    assert_eq!(FILE_CHANGED_LINE, "File changed since");
}

#[test]
fn c04_line_two_reads_time_and_page_count() {
    let now = local(2026, 11, 23, 15, 0);
    assert_eq!(
        row_line_two(local(2026, 11, 23, 12, 3), now, 1, None),
        "12:03 · 1 page"
    );
    assert_eq!(
        row_line_two(local(2026, 11, 23, 12, 3), now, 3, None),
        "12:03 · 3 pages"
    );
    assert_eq!(pages_text(1), "1 page");
    assert_eq!(pages_text(3), "3 pages");
}

#[test]
fn c04_the_time_follows_the_workspace_row_format() {
    let now = local(2026, 11, 23, 15, 0);
    assert_eq!(
        time_text(local(2026, 11, 23, 12, 3), now),
        "12:03",
        "same day"
    );
    assert_eq!(
        time_text(local(2026, 11, 2, 12, 3), now),
        "2 Nov 12:03",
        "same year"
    );
    assert_eq!(
        time_text(local(2025, 9, 29, 12, 3), now),
        "29 Sep 2025 12:03",
        "earlier year"
    );
}

fn local(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> LocalTime {
    LocalTime {
        year,
        month,
        day,
        hour,
        minute,
    }
}

fn sample_record(name: Option<&str>) -> RecoveryRecord {
    RecoveryRecord {
        format_version: RECOVERY_FORMAT,
        key: "0123456789abcdef0123456789abcdef".to_owned(),
        name: name.map(str::to_owned),
        base: name.map(|n| RecoveryBase {
            path: format!("/home/marc/{n}"),
            modified_ms: 1_700_000_000_000,
            length: 4_096,
        }),
        last_step_ms: 1_795_000_000_000,
        pages: 3,
    }
}

fn cand(rank: u64, bytes: u64, locked: bool) -> PruneCandidate {
    PruneCandidate {
        key: format!("key-{rank}"),
        bytes,
        // Higher rank is newer; the key says which copy it is.
        last_step_ms: 1_000_000 + rank,
        locked,
    }
}
