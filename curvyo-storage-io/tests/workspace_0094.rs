//! The disk side of a local workspace (`specs/0094-workspace` criteria 1, 2, 4, 5, 7, 13 to 18,
//! 20, 23, 24): marker, scan, file operations, trash and the recents file, on real folders.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;

use curvyo_storage_io::{
    DocumentProbe, MARKER_DIR, MARKER_FILE, MAX_RECENT_WORKSPACES, RecentWorkspaces,
    RecentWorkspacesError, ScannedEntry, ScannedKind, WorkspaceError, create_document,
    create_folder, fresh_workspace_id, marker_text, move_entry, open_workspace, parse_marker,
    probe_document, read_document, scan_workspace, trash_entry,
};

const ID_A: [u8; 16] = [0x11; 16];
const ID_B: [u8; 16] = [0x22; 16];

fn p(text: &str) -> Vec<String> {
    text.split('/')
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .collect()
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// Every file below `root` with its bytes, the marker directory included.
fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.push((path.display().to_string(), Vec::new()));
                stack.push(path);
            } else {
                out.push((path.display().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn paths(entries: &[ScannedEntry]) -> Vec<String> {
    entries.iter().map(|e| e.path.join("/")).collect()
}

/// A folder with `Lid.curvyo`, `Sides/Left.curvyo` and an unrelated `notes.txt`.
fn box_job() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Lid.curvyo"), b"lid").unwrap();
    fs::create_dir(dir.path().join("Sides")).unwrap();
    fs::write(dir.path().join("Sides/Left.curvyo"), b"left").unwrap();
    fs::write(dir.path().join("notes.txt"), b"notes").unwrap();
    dir
}

// --- marker (criteria 1, 2, 23, 24) ---

#[test]
fn the_marker_golden_reads_and_writes_back_byte_for_byte() {
    let text = fixture("workspace_v1.toml");
    let id = parse_marker(&text).unwrap();
    assert_eq!(
        id,
        *b"\x00\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\xbb\xcc\xdd\xee\xff"
    );
    assert_eq!(marker_text(id), text);
}

#[test]
fn a_newer_or_damaged_marker_is_refused() {
    assert!(matches!(
        parse_marker("format_version = 2\nid = \"x\"\nmore = 1\n"),
        Err(WorkspaceError::TooNew { found: 2 })
    ));
    for text in [
        "",
        "format_version = 1\n",
        "format_version = 1\nid = \"0011\"\n",
        "format_version = 1\nid = \"00112233445566778899AABBCCDDEEFF\"\n",
        "format_version = 0\nid = \"00112233445566778899aabbccddeeff\"\n",
        "format_version = 1\nid = \"00112233445566778899aabbccddeeff\"\nname = \"x\"\n",
    ] {
        assert!(
            matches!(parse_marker(text), Err(WorkspaceError::Damaged)),
            "{text:?}"
        );
    }
}

#[test]
fn opening_a_folder_creates_only_the_marker() {
    let dir = box_job();
    let before = snapshot(dir.path());
    let id = open_workspace(dir.path(), ID_A).unwrap();
    assert_eq!(id, ID_A);
    let marker = dir.path().join(MARKER_DIR).join(MARKER_FILE);
    assert_eq!(fs::read_to_string(&marker).unwrap(), marker_text(ID_A));
    let mut after = snapshot(dir.path());
    after.retain(|(path, _)| !path.contains(MARKER_DIR));
    assert_eq!(
        after, before,
        "no existing file changed, no probe left behind"
    );
}

#[test]
fn reopening_and_a_copied_folder_keep_the_id() {
    let dir = box_job();
    open_workspace(dir.path(), ID_A).unwrap();
    assert_eq!(open_workspace(dir.path(), ID_B).unwrap(), ID_A);
    let copy = tempfile::tempdir().unwrap();
    fs::create_dir(copy.path().join(MARKER_DIR)).unwrap();
    fs::copy(
        dir.path().join(MARKER_DIR).join(MARKER_FILE),
        copy.path().join(MARKER_DIR).join(MARKER_FILE),
    )
    .unwrap();
    assert_eq!(open_workspace(copy.path(), ID_B).unwrap(), ID_A);
}

#[test]
fn two_workspaces_get_the_ids_they_were_given() {
    let first = box_job();
    let second = box_job();
    let a = open_workspace(first.path(), ID_A).unwrap();
    let b = open_workspace(second.path(), ID_B).unwrap();
    assert_ne!(a, b);
}

#[test]
fn a_newer_marker_is_refused_without_writing() {
    let dir = box_job();
    fs::create_dir(dir.path().join(MARKER_DIR)).unwrap();
    let marker = dir.path().join(MARKER_DIR).join(MARKER_FILE);
    fs::write(&marker, "format_version = 9\nid = \"x\"\n").unwrap();
    let before = snapshot(dir.path());
    assert!(matches!(
        open_workspace(dir.path(), ID_A),
        Err(WorkspaceError::TooNew { found: 9 })
    ));
    assert_eq!(snapshot(dir.path()), before);
}

#[test]
fn a_file_is_not_a_workspace() {
    let dir = box_job();
    assert!(matches!(
        open_workspace(&dir.path().join("Lid.curvyo"), ID_A),
        Err(WorkspaceError::NotAFolder)
    ));
    assert!(matches!(
        open_workspace(&dir.path().join("missing"), ID_A),
        Err(WorkspaceError::NotAFolder)
    ));
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_written_is_not_opened() {
    use std::os::unix::fs::PermissionsExt;
    let dir = box_job();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let result = open_workspace(dir.path(), ID_A);
    let marker_made = dir.path().join(MARKER_DIR).exists();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(result, Err(WorkspaceError::NotWritable)),
        "{result:?}"
    );
    assert!(!marker_made);
}

// --- scan (criteria 1, 5, 7) ---

#[test]
fn the_scan_lists_documents_and_folders_only() {
    let dir = box_job();
    open_workspace(dir.path(), ID_A).unwrap();
    fs::create_dir(dir.path().join("Empty")).unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".git/x.curvyo"), b"").unwrap();
    fs::write(dir.path().join(".hidden.curvyo"), b"").unwrap();
    fs::write(dir.path().join("Upper.CURVYO"), b"up").unwrap();
    fs::write(dir.path().join("Lid.curvyo.123.tmp"), b"").unwrap();
    fs::create_dir(dir.path().join("Folder.curvyo")).unwrap();
    let entries = scan_workspace(dir.path()).unwrap();
    assert_eq!(
        paths(&entries),
        [
            "Empty",
            "Folder.curvyo",
            "Lid.curvyo",
            "Sides",
            "Sides/Left.curvyo",
            "Upper.CURVYO"
        ]
    );
    let lid = entries.iter().find(|e| e.path == p("Lid.curvyo")).unwrap();
    assert_eq!(lid.kind, ScannedKind::Document);
    assert_eq!(lid.size, 3);
    assert!(lid.modified > 1_700_000_000);
    let folder = entries
        .iter()
        .find(|e| e.path == p("Folder.curvyo"))
        .unwrap();
    assert_eq!(folder.kind, ScannedKind::Folder);
}

#[cfg(unix)]
#[test]
fn the_scan_does_not_follow_symbolic_links() {
    let dir = box_job();
    std::os::unix::fs::symlink(dir.path(), dir.path().join("Loop")).unwrap();
    std::os::unix::fs::symlink(
        dir.path().join("Lid.curvyo"),
        dir.path().join("Link.curvyo"),
    )
    .unwrap();
    assert_eq!(
        paths(&scan_workspace(dir.path()).unwrap()),
        ["Lid.curvyo", "Sides", "Sides/Left.curvyo"]
    );
}

#[test]
fn the_scan_sees_changes_made_by_another_program() {
    let dir = box_job();
    fs::rename(dir.path().join("Lid.curvyo"), dir.path().join("Top.curvyo")).unwrap();
    fs::remove_file(dir.path().join("Sides/Left.curvyo")).unwrap();
    fs::write(dir.path().join("Sides/Right.curvyo"), b"r").unwrap();
    assert_eq!(
        paths(&scan_workspace(dir.path()).unwrap()),
        ["Sides", "Sides/Right.curvyo", "Top.curvyo"]
    );
}

/// The thread's CPU time (`CLOCK_THREAD_CPUTIME_ID`); wall time off Unix.
fn cpu_time_of<T>(work: impl FnOnce() -> T) -> (T, Duration) {
    #[cfg(unix)]
    let now = || {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        Duration::new(
            u64::try_from(t.tv_sec).unwrap(),
            u32::try_from(t.tv_nsec).unwrap(),
        )
    };
    #[cfg(not(unix))]
    let now = {
        let origin = std::time::Instant::now();
        move || origin.elapsed()
    };
    let start = now();
    let result = work();
    (result, now().saturating_sub(start))
}

#[test]
fn five_hundred_documents_in_nested_folders_scan_within_a_second_of_cpu() {
    let dir = tempfile::tempdir().unwrap();
    for folder in 0..20 {
        let inner = dir.path().join(format!("Job {folder}/Part {}", folder % 4));
        fs::create_dir_all(&inner).unwrap();
        for doc in 0..25 {
            fs::write(inner.join(format!("Piece {doc}.curvyo")), b"x").unwrap();
        }
    }
    let (entries, took) = cpu_time_of(|| scan_workspace(dir.path()).unwrap());
    let documents = entries
        .iter()
        .filter(|e| e.kind == ScannedKind::Document)
        .count();
    assert_eq!(documents, 500);
    println!("scan of 500 documents: {took:?}");
    assert!(took <= Duration::from_secs(1), "{took:?}");
}

// --- file operations (criteria 13, 14, 16, 18) ---

#[test]
fn a_new_document_is_written_and_never_replaces_one() {
    let dir = box_job();
    create_document(dir.path(), &p("Sides/Right.curvyo"), b"right").unwrap();
    assert_eq!(
        fs::read(dir.path().join("Sides/Right.curvyo")).unwrap(),
        b"right"
    );
    assert!(matches!(
        create_document(dir.path(), &p("Lid.curvyo"), b"new"),
        Err(WorkspaceError::Exists)
    ));
    assert_eq!(fs::read(dir.path().join("Lid.curvyo")).unwrap(), b"lid");
    assert_eq!(
        paths(&scan_workspace(dir.path()).unwrap()),
        [
            "Lid.curvyo",
            "Sides",
            "Sides/Left.curvyo",
            "Sides/Right.curvyo"
        ],
        "no temporary file left behind"
    );
}

#[test]
fn a_new_folder_is_created_once() {
    let dir = box_job();
    create_folder(dir.path(), &p("Sides/Inner")).unwrap();
    assert!(dir.path().join("Sides/Inner").is_dir());
    assert!(matches!(
        create_folder(dir.path(), &p("Sides")),
        Err(WorkspaceError::Exists)
    ));
}

#[test]
fn rename_and_move_never_replace_an_entry() {
    let dir = box_job();
    move_entry(dir.path(), &p("Lid.curvyo"), &p("Sides/Lid.curvyo")).unwrap();
    assert_eq!(
        fs::read(dir.path().join("Sides/Lid.curvyo")).unwrap(),
        b"lid"
    );
    assert!(matches!(
        move_entry(dir.path(), &p("Sides/Lid.curvyo"), &p("Sides/Left.curvyo")),
        Err(WorkspaceError::Exists)
    ));
    assert_eq!(
        fs::read(dir.path().join("Sides/Left.curvyo")).unwrap(),
        b"left"
    );
    create_folder(dir.path(), &p("Empty")).unwrap();
    create_folder(dir.path(), &p("Other")).unwrap();
    assert!(matches!(
        move_entry(dir.path(), &p("Other"), &p("Empty")),
        Err(WorkspaceError::Exists)
    ));
    assert!(matches!(
        move_entry(dir.path(), &p("Gone.curvyo"), &p("New.curvyo")),
        Err(WorkspaceError::NotFound)
    ));
    move_entry(dir.path(), &p("Sides"), &p("Walls")).unwrap();
    assert!(dir.path().join("Walls/Lid.curvyo").is_file());
}

#[test]
fn a_change_of_case_alone_is_a_rename() {
    let dir = box_job();
    move_entry(dir.path(), &p("Lid.curvyo"), &p("LID.curvyo")).unwrap();
    assert_eq!(paths(&scan_workspace(dir.path()).unwrap())[0], "LID.curvyo");
}

#[test]
fn paths_that_leave_the_workspace_are_refused() {
    let dir = box_job();
    for path in [
        p(""),
        vec!["..".to_string(), "x.curvyo".to_string()],
        vec![".".to_string()],
        vec![MARKER_DIR.to_string(), MARKER_FILE.to_string()],
        vec!["a/b".to_string()],
        vec!["a\\b".to_string()],
        vec![String::new()],
        vec!["/etc".to_string()],
    ] {
        assert!(
            matches!(
                create_document(dir.path(), &path, b"x"),
                Err(WorkspaceError::OutsideWorkspace)
            ),
            "{path:?}"
        );
        assert!(matches!(
            move_entry(dir.path(), &p("Lid.curvyo"), &path),
            Err(WorkspaceError::OutsideWorkspace)
        ));
        assert!(matches!(
            trash_entry(dir.path(), &path),
            Err(WorkspaceError::OutsideWorkspace)
        ));
    }
    assert_eq!(fs::read(dir.path().join("Lid.curvyo")).unwrap(), b"lid");
}

// --- trash (criterion 17) ---

#[test]
fn delete_moves_the_entry_to_the_trash() {
    let dir = box_job();
    let root = dir.path().canonicalize().unwrap();
    trash_entry(&root, &p("Sides")).unwrap();
    assert!(!root.join("Sides").exists());
    assert!(matches!(
        trash_entry(&root, &p("Sides")),
        Err(WorkspaceError::NotFound)
    ));
    #[cfg(target_os = "linux")]
    {
        let ours: Vec<_> = trash::os_limited::list()
            .unwrap()
            .into_iter()
            .filter(|item| item.original_parent == root)
            .collect();
        assert_eq!(ours.len(), 1, "the folder is in the trash, not erased");
        assert_eq!(ours[0].name, "Sides");
        trash::os_limited::purge_all(ours).unwrap();
    }
}

// --- recents (criterion 20) ---

#[test]
fn the_recents_golden_reads_and_writes_back() {
    let text = fixture("recent-workspaces_v1.toml");
    let recents = RecentWorkspaces::parse(&text).unwrap();
    assert_eq!(recents.path(0), Some(Path::new("/home/maker/Box job")));
    assert_eq!(recents.path(1), Some(Path::new("/home/maker/Sign")));
    assert_eq!(recents.to_text(), text);
}

#[test]
fn recents_put_the_latest_first_without_duplicates_and_keep_ten() {
    let mut recents = RecentWorkspaces::default();
    for n in 0..12 {
        recents.opened(Path::new(&format!("/w/{n}")));
    }
    recents.opened(Path::new("/w/5"));
    let rows = recents.rows();
    assert_eq!(rows.len(), MAX_RECENT_WORKSPACES);
    assert_eq!(rows[0].path, Path::new("/w/5"));
    assert_eq!(rows[1].path, Path::new("/w/11"));
    assert_eq!(
        rows.iter().filter(|r| r.path == Path::new("/w/5")).count(),
        1
    );
    assert!(rows.iter().all(|r| r.path != Path::new("/w/0")));
}

#[test]
fn recents_rows_are_removable_and_say_which_folders_exist() {
    let dir = box_job();
    let mut recents = RecentWorkspaces::default();
    recents.opened(Path::new("/no/such/Sign"));
    recents.opened(dir.path());
    let rows = recents.rows();
    assert!(rows[0].found);
    assert_eq!(
        rows[0].name,
        dir.path().file_name().unwrap().to_string_lossy()
    );
    assert_eq!(rows[1].name, "Sign");
    assert!(!rows[1].found);
    recents.remove(1);
    recents.remove(7);
    assert_eq!(recents.rows().len(), 1);
    assert!(dir.path().exists(), "removing a row touches no folder");
}

#[test]
fn a_newer_or_damaged_recents_file_is_refused() {
    assert_eq!(
        RecentWorkspaces::parse("format = 2\nother = true\n"),
        Err(RecentWorkspacesError::Newer)
    );
    for text in [
        "",
        "paths = []\n",
        "format = 1\npaths = [1]\n",
        "format = 1\npaths = []\nx = 1\n",
    ] {
        assert_eq!(
            RecentWorkspaces::parse(text),
            Err(RecentWorkspacesError::Damaged),
            "{text:?}"
        );
    }
}

/// A zip archive with the named members.
fn zip_with(members: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write as _;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in members {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// Criteria 6 and 8: a row reads only the container's thumbnail (none in a file without one), and
/// a file that is no container or a damaged one says so; nothing is written.
#[test]
fn a_probe_reads_the_thumbnail_or_says_why_the_file_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let png = b"\x89PNG\r\n\x1a\nthumb";
    fs::write(
        dir.path().join("With.curvyo"),
        zip_with(&[("manifest.json", b"{}"), ("thumbnail.png", png)]),
    )
    .unwrap();
    fs::write(
        dir.path().join("Without.curvyo"),
        zip_with(&[("manifest.json", b"{}")]),
    )
    .unwrap();
    fs::write(dir.path().join("Text.curvyo"), b"hello").unwrap();
    fs::write(
        dir.path().join("Other.curvyo"),
        zip_with(&[("readme.txt", b"x")]),
    )
    .unwrap();
    let mut cut = zip_with(&[("manifest.json", b"{}")]);
    cut.truncate(cut.len() - 30);
    fs::write(dir.path().join("Cut.curvyo"), cut).unwrap();
    let before = snapshot(dir.path());

    assert_eq!(
        probe_document(dir.path(), &p("With.curvyo")).unwrap(),
        DocumentProbe::Readable {
            thumbnail: Some(png.to_vec())
        }
    );
    assert_eq!(
        probe_document(dir.path(), &p("Without.curvyo")).unwrap(),
        DocumentProbe::Readable { thumbnail: None }
    );
    assert_eq!(
        probe_document(dir.path(), &p("Text.curvyo")).unwrap(),
        DocumentProbe::NotCurvyo
    );
    assert_eq!(
        probe_document(dir.path(), &p("Other.curvyo")).unwrap(),
        DocumentProbe::NotCurvyo
    );
    assert_eq!(
        probe_document(dir.path(), &p("Cut.curvyo")).unwrap(),
        DocumentProbe::Damaged
    );
    assert!(matches!(
        probe_document(dir.path(), &p("Gone.curvyo")),
        Err(WorkspaceError::NotFound)
    ));
    assert!(matches!(
        probe_document(dir.path(), &p("../x.curvyo")),
        Err(WorkspaceError::OutsideWorkspace)
    ));
    assert_eq!(snapshot(dir.path()), before);
}

/// Criteria 9 and 13: a document's bytes are read by its path inside the workspace only.
#[test]
fn a_document_is_read_by_its_path_inside_the_workspace() {
    let dir = box_job();
    assert_eq!(
        read_document(dir.path(), &p("Sides/Left.curvyo")).unwrap(),
        fs::read(dir.path().join("Sides/Left.curvyo")).unwrap()
    );
    assert!(matches!(
        read_document(dir.path(), &p("Gone.curvyo")),
        Err(WorkspaceError::NotFound)
    ));
    assert!(matches!(
        read_document(dir.path(), &p("../Lid.curvyo")),
        Err(WorkspaceError::OutsideWorkspace)
    ));
}

/// Criteria 2 and 24: fresh ids are random.
#[test]
fn fresh_workspace_ids_differ() {
    let a = fresh_workspace_id().unwrap();
    let b = fresh_workspace_id().unwrap();
    assert_ne!(a, b);
}
