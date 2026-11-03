//! Tests of `workspace_glue`: rows, plans, a shared workspace's row states and copies.
use super::*;

fn scan() -> &'static str {
    r#"[
        {"path":["Lid.curvyo"],"kind":"document","size":10,"modified":1793707380},
        {"path":["Sides"],"kind":"folder","size":0,"modified":0},
        {"path":["Sides","Left.curvyo"],"kind":"document","size":10,"modified":0},
        {"path":["Sides","Inner"],"kind":"folder","size":0,"modified":0}
    ]"#
}

fn rows(glue: &WorkspaceGlue) -> Value {
    serde_json::from_str(&glue.rows_json(1_793_707_380, 0)).unwrap()
}

#[test]
fn a_scan_gives_rows_and_a_repeated_scan_changes_nothing() {
    let mut glue = WorkspaceGlue::new("Box job");
    assert!(glue.set_scan(scan()));
    assert!(!glue.set_scan(scan()), "the same scan is no change");
    let view = rows(&glue);
    assert_eq!(view["header"], "2 documents");
    assert_eq!(view["rows"][0]["name"], "Sides");
    assert_eq!(
        view["rows"][0]["label"],
        "Sides, folder, 1 document, collapsed"
    );
    assert_eq!(view["rows"][1]["name"], "Lid");
    assert_eq!(view["rows"][1]["time"], "12:03");
    assert_eq!(view["rows"].as_array().unwrap().len(), 2);

    assert!(glue.set_expanded(r#"["Sides"]"#, true));
    assert!(
        !glue.set_expanded(r#"["Lid.curvyo"]"#, true),
        "a document does not expand"
    );
    let view = rows(&glue);
    assert_eq!(view["rows"][1]["name"], "Inner");
    assert_eq!(view["rows"][2]["name"], "Left");
    assert!(glue.contains(r#"["Sides","Left.curvyo"]"#));
    assert!(!glue.contains(r#"["Gone.curvyo"]"#));
}

/// Criteria 13, 14, 16, 17, 19: each request gives the host's operation and its words, or the
/// reason it is refused.
#[test]
fn plans_give_the_operation_and_its_words_or_the_refusal() {
    let mut glue = WorkspaceGlue::new("Box job");
    glue.set_scan(scan());
    let plan = |request: &str| -> Value { serde_json::from_str(&glue.plan(request)).unwrap() };

    let created = plan(r#"{"kind":"new-document","folder":[],"name":"lid"}"#);
    assert_eq!(created["op"]["kind"], "create-document");
    assert_eq!(created["op"]["path"], json!(["lid 2.curvyo"]));
    assert_eq!(created["status"], "Created lid 2.");

    let refused = plan(r#"{"kind":"rename","path":["Lid.curvyo"],"name":"a/b"}"#);
    assert_eq!(refused["refusal"], "A name cannot contain / or \\.");

    let moved = plan(r#"{"kind":"move","path":["Lid.curvyo"],"folder":["Sides"]}"#);
    assert_eq!(
        moved["op"],
        json!({"kind":"move","from":["Lid.curvyo"],"to":["Sides","Lid.curvyo"]})
    );
    assert_eq!(moved["status"], "Moved Lid to Sides.");

    let into_itself = plan(r#"{"kind":"move","path":["Sides"],"folder":["Sides","Inner"]}"#);
    assert_eq!(into_itself["refusal"], "Cannot move Sides into itself.");

    let deleted = plan(r#"{"kind":"delete","path":["Sides"]}"#);
    assert_eq!(
        deleted["confirm"],
        "Move Sides and its 1 document to the trash?"
    );
    assert_eq!(deleted["op"], json!({"kind":"trash","path":["Sides"]}));

    let copy = plan(r#"{"kind":"duplicate","path":["Lid.curvyo"]}"#);
    assert_eq!(copy["op"]["path"], json!(["Lid copy.curvyo"]));
    assert_eq!(plan("not json"), json!({}));
}

/// Criteria 40 and 41: a shared workspace's rows say which file is outside it and which
/// listed document is missing or still arriving; Delete asks for everyone.
#[test]
fn shared_rows_carry_their_state_and_delete_is_for_everyone() {
    let mut glue = WorkspaceGlue::new("Box job");
    glue.set_scan(scan());
    assert!(glue.set_shared_states(
        r#"{"shared":true,"outside":[["Lid.curvyo"]],"missing":[["Gone.curvyo"]],"receiving":[["New.curvyo"]]}"#
    ));
    assert!(!glue.set_shared_states(
        r#"{"shared":true,"outside":[["Lid.curvyo"]],"missing":[["Gone.curvyo"]],"receiving":[["New.curvyo"]]}"#
    ));
    let view = rows(&glue);
    let row = |name: &str| {
        view["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .cloned()
            .unwrap()
    };
    assert_eq!(row("Lid")["state"], "outside");
    assert_eq!(row("Lid")["time"], "Not in the shared workspace");
    assert_eq!(row("Gone")["time"], "Missing on this computer");
    assert_eq!(row("New")["state"], "receiving");
    let deleted: Value =
        serde_json::from_str(&glue.plan(r#"{"kind":"delete","path":["Sides"]}"#)).unwrap();
    assert!(
        deleted["confirm"]
            .as_str()
            .unwrap()
            .starts_with("Delete Sides and its 1 document for everyone?")
    );
}

#[test]
fn a_moved_folder_stays_expanded() {
    let mut glue = WorkspaceGlue::new("Box job");
    glue.set_scan(scan());
    glue.set_expanded(r#"["Sides"]"#, true);
    glue.moved(r#"["Sides"]"#, r#"["Walls"]"#);
    assert!(glue.expanded.contains(&vec!["Walls".to_owned()]));
    assert!(!glue.expanded.contains(&vec!["Sides".to_owned()]));
}

/// Criterion 19: the copy opens, and it carries no share record.
#[test]
fn a_copy_is_the_same_drawing_without_the_share_record() {
    let original = curvyo_document_core::Document::new(7);
    original.set_share(Some(curvyo_document_core::ShareRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [3; 16],
        server_seq: 4,
        synced_version: curvyo_document_core::DocumentVersion::never(),
        keyring_log: vec![1, 2, 3],
    }));
    let bytes = curvyo_document_core::pack(&original, "0.1.0").unwrap();
    let copy = workspace_copy_bytes(&bytes, "0.1.0").unwrap();
    let reopened = curvyo_document_core::unpack(9, &copy).unwrap();
    assert!(
        curvyo_document_core::unpack(8, &bytes)
            .unwrap()
            .share()
            .is_some()
    );
    assert!(reopened.share().is_none());
    assert!(workspace_copy_bytes(b"not a zip", "0.1.0").is_err());
}
