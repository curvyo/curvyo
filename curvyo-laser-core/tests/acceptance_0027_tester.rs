//! Independent acceptance tests for `specs/0027-manufacturing-roles` at the laser extension
//! (ADR 0040): the three stored values and their spelling (criteria 2, 5), the key form with the
//! reserved `outside` word (5, 11), the read of absent, known, unknown and non-string values
//! (3, 17, 18), the output rule that fails closed (ADR 0040 §6, criteria 8 f, 13, 14), the notice
//! lines (15) and the Layers block below the Machine block in the manifest (1).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use curvyo_document_core::{Document, NodeId};
use curvyo_extension_core::StoredParams;
use curvyo_laser_core::{
    PROCESS_ID, Role, RoleRead, RoleTarget, SET_ROLE_COMMIT_LABEL, is_output, manifest, read_role,
    role_notices, role_value,
};
use serde_json::{Value, json};

const NOTICE_MAX: usize = 70;

fn layer_id(peer: u64) -> NodeId {
    Document::new(peer).new_layer(None)
}

fn stored(pairs: &[(&str, Value)]) -> StoredParams {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

// ---- criterion 2 and 5: the three values ------------------------------------------------------

#[test]
fn ac5_the_stored_values_are_exactly_cut_engrave_ignore() {
    assert_eq!(Role::Cut.as_str(), "cut");
    assert_eq!(Role::Engrave.as_str(), "engrave");
    assert_eq!(Role::Ignore.as_str(), "ignore");
    for r in [Role::Cut, Role::Engrave, Role::Ignore] {
        assert_eq!(Role::parse(r.as_str()), Some(r));
    }
}

#[test]
fn ac18_parse_is_exact_no_case_folding_no_trimming_no_synonyms() {
    for bad in [
        "Cut",
        "CUT",
        " cut",
        "cut ",
        "engrave\n",
        "score",
        "line",
        "fill",
        "",
        "none",
        "unassigned",
        "outside",
        "cut,engrave",
    ] {
        assert_eq!(Role::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn ac2_role_value_writes_the_string_or_deletes_for_no_role() {
    assert_eq!(role_value(Some(Role::Cut)), Some(Value::from("cut")));
    assert_eq!(
        role_value(Some(Role::Engrave)),
        Some(Value::from("engrave"))
    );
    assert_eq!(role_value(Some(Role::Ignore)), Some(Value::from("ignore")));
    assert_eq!(role_value(None), None, "clearing deletes the register");
}

#[test]
fn ac4_the_commit_label_is_the_one_the_history_names_set_role() {
    assert_eq!(SET_ROLE_COMMIT_LABEL, "set_role");
}

// ---- criterion 5 and 11: key form ------------------------------------------------------------

#[test]
fn ac5_the_key_is_x_curvyo_laser_role_peer_counter() {
    let layer = layer_id(3);
    let text = layer.to_register_text();
    assert!(text.starts_with("3:"), "{text}");
    assert_eq!(
        RoleTarget::Layer(layer).key(),
        format!("x.curvyo.laser.role.{text}")
    );
    assert_eq!(RoleTarget::Layer(layer).key_text(), text);
    assert!(
        RoleTarget::Layer(layer)
            .key()
            .starts_with(&format!("x.{PROCESS_ID}.")),
        "inside the laser's own namespace"
    );
}

#[test]
fn ac11_the_outside_key_is_reserved_and_never_a_node_id() {
    assert_eq!(RoleTarget::Outside.key(), "x.curvyo.laser.role.outside");
    assert_eq!(RoleTarget::Outside.key_text(), "outside");
    assert_eq!(
        RoleTarget::from_key_text("outside"),
        Some(RoleTarget::Outside)
    );
    assert_eq!(NodeId::from_register_text("outside"), None);
}

#[test]
fn ac5_key_text_round_trips_and_rejects_junk() {
    for peer in [0_u64, 1, 7, u64::from(u32::MAX), 1 << 40] {
        let layer = layer_id(peer);
        let t = RoleTarget::Layer(layer);
        assert_eq!(
            RoleTarget::from_key_text(&t.key_text()),
            Some(t),
            "peer {peer}"
        );
    }
    for junk in [
        "",
        ":",
        "1:",
        ":1",
        "a:b",
        "1:2:3",
        "-1:2",
        " 1:2",
        "1:2 ",
        "Outside",
        "OUTSIDE",
        "1:99999999999999999999999",
    ] {
        assert_eq!(RoleTarget::from_key_text(junk), None, "{junk:?}");
    }
}

// ---- criterion 3, 17, 18: reading ------------------------------------------------------------

#[test]
fn ac3_absent_reads_as_unassigned() {
    let layer = layer_id(1);
    assert_eq!(
        read_role(&StoredParams::new(), RoleTarget::Layer(layer)),
        RoleRead::Unassigned
    );
    assert_eq!(
        read_role(&StoredParams::new(), RoleTarget::Outside),
        RoleRead::Unassigned
    );
    assert_eq!(RoleRead::Unassigned.role(), None);
}

#[test]
fn ac5_each_layer_reads_its_own_register_only() {
    let a = layer_id(1);
    let d = Document::new(2);
    let b = d.new_layer(None);
    let c = d.new_layer(None);
    let p = stored(&[
        (&RoleTarget::Layer(a).key(), json!("cut")),
        (&RoleTarget::Layer(b).key(), json!("engrave")),
        (&RoleTarget::Outside.key(), json!("ignore")),
        ("x.curvyo.laser.something_else", json!("cut")),
    ]);
    assert_eq!(
        read_role(&p, RoleTarget::Layer(a)),
        RoleRead::Known(Role::Cut)
    );
    assert_eq!(
        read_role(&p, RoleTarget::Layer(b)),
        RoleRead::Known(Role::Engrave)
    );
    assert_eq!(
        read_role(&p, RoleTarget::Outside),
        RoleRead::Known(Role::Ignore)
    );
    assert_eq!(read_role(&p, RoleTarget::Layer(c)), RoleRead::Unassigned);
}

#[test]
fn ac18_unknown_strings_and_non_strings_read_as_unknown() {
    let layer = layer_id(1);
    let k = RoleTarget::Layer(layer).key();
    for v in [
        json!("score"),
        json!("Cut"),
        json!(""),
        json!(1),
        json!(0.5),
        json!(true),
        json!(null),
    ] {
        let p = stored(&[(&k, v.clone())]);
        let r = read_role(&p, RoleTarget::Layer(layer));
        assert_eq!(r, RoleRead::Unknown, "{v}");
        assert_eq!(r.role(), None, "{v}: unknown has no role");
    }
}

#[test]
fn ac18_a_role_register_of_another_process_is_not_read() {
    let layer = layer_id(1);
    let foreign = format!("x.curvyo.plotter.role.{}", layer.to_register_text());
    let p = stored(&[(&foreign, json!("cut"))]);
    assert_eq!(
        read_role(&p, RoleTarget::Layer(layer)),
        RoleRead::Unassigned
    );
}

// ---- ADR 0040 §6: what is output -------------------------------------------------------------

#[test]
fn adr6_output_rule_fails_closed() {
    use RoleRead::{Known, Unassigned, Unknown};
    let table = [
        (Known(Role::Cut), false, true),
        (Known(Role::Engrave), false, true),
        (Known(Role::Ignore), false, false),
        (Unassigned, false, false),
        (Unknown, false, false),
        (Known(Role::Cut), true, false),
        (Known(Role::Engrave), true, true),
        (Known(Role::Ignore), true, false),
        (Unassigned, true, false),
        (Unknown, true, false),
    ];
    for (role, image, want) in table {
        assert_eq!(is_output(role, image), want, "{role:?} image={image}");
    }
}

// ---- criterion 15: notice lines --------------------------------------------------------------

#[test]
fn ac15_no_notice_when_both_counts_are_zero() {
    assert!(role_notices(0, 0).is_empty());
}

#[test]
fn ac15_the_spec_texts_with_singular_and_plural() {
    assert_eq!(
        role_notices(2, 0),
        vec!["2 layers have no role and are not output.".to_owned()]
    );
    assert_eq!(
        role_notices(0, 1),
        vec!["1 image in a Cut layer is not output.".to_owned()]
    );
    let one = role_notices(1, 0);
    assert_eq!(one.len(), 1);
    assert_eq!(one[0], "1 layer has no role and is not output.");
    let images = role_notices(0, 2);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0], "2 images in Cut layers are not output.");
}

#[test]
fn ac15_both_lines_in_order_and_each_within_70_characters_even_for_huge_counts() {
    for (a, b) in [
        (1, 1),
        (3, 7),
        (usize::MAX, usize::MAX),
        (1_000_000, 999_999),
    ] {
        let lines = role_notices(a, b);
        assert_eq!(lines.len(), 2, "{a} {b}");
        assert!(lines[0].contains("role"), "layers first: {lines:?}");
        assert!(lines[1].contains("image"), "images second: {lines:?}");
        for l in &lines {
            assert!(l.chars().count() <= NOTICE_MAX, "{l:?}");
        }
    }
}

// ---- criterion 1: the Layers block below Machine ---------------------------------------------

#[test]
fn ac1_the_laser_tab_has_a_layers_block_below_the_machine_block() {
    let m = serde_json::to_value(manifest()).unwrap();
    let text = m.to_string();
    let machine = text
        .find("\"machine_profile\"")
        .expect("a machine block in the manifest");
    let layers = text
        .find("\"layers\"")
        .expect("a layers block in the manifest");
    assert!(machine < layers, "Layers is below Machine: {text}");
    assert_eq!(
        text.matches("\"kind\":\"layers\"").count(),
        1,
        "exactly one Layers block"
    );
    assert!(text.contains("\"label\":\"Layers\""), "{text}");
}
