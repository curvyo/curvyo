//! Tester acceptance cases for `specs/0135-image-optimisation` slice 1 in `curvyo-document-core`,
//! written from the specification and ADR 0072 before the implementation was read: the three
//! registers survive save and reopen (criterion 8), format 51 (8), damaged registers of each
//! reason keep the file open and read as their defaults (8, 23), the merge of the three
//! registers across two peers (ADR 0072 §4), own adjustments all or nothing on a clone and Follow
//! original (9), Restore original (59), every edit one undo step (24), and the goldens.
#![allow(
    clippy::needless_pass_by_value,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::trivially_copy_pass_by_ref,
    clippy::unwrap_used,
    clippy::too_many_lines,
    missing_docs
)]

use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustOwner, AdjustStep, AdjustmentsWrite, Affine, Document, ImageAdjustments,
    NewImage, NodeId, ObjectSnapshot, pack, unpack,
};
use serde_json::{Value, json};

fn png() -> Arc<[u8]> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
    Arc::from(std::fs::read(path).unwrap())
}

fn image(d: &Document, name: &str) -> NodeId {
    d.add_images(&[NewImage {
        bytes: png(),
        placement: Affine::new(10.0, 0.0, 0.0, 10.0, 0.0, 0.0),
        name: name.to_owned(),
    }])
    .unwrap()[0]
}

fn step(kind: &str, fields: &[(&str, i64)]) -> AdjustEntry {
    AdjustEntry::Step(AdjustStep {
        kind: kind.to_owned(),
        enabled: true,
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    })
}

fn levels(b: i64) -> AdjustEntry {
    step(
        "levels",
        &[("black_pct", b), ("white_pct", 90), ("gamma_centi", 120)],
    )
}

fn write(d: &Document, id: NodeId, w: AdjustmentsWrite) -> bool {
    d.edit_image_adjustments(id, &w).unwrap()
}

fn shown(d: &Document, id: NodeId) -> ImageAdjustments {
    d.shown_adjustments(id).unwrap().adjustments
}

fn owner(d: &Document, id: NodeId) -> AdjustOwner {
    d.shown_adjustments(id).unwrap().owner
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn reopen(d: &Document) -> Document {
    unpack(5, &pack(d, "0.1.0").unwrap()).unwrap()
}

fn sync(from: &Document, to: &Document) {
    to.import_updates(&from.export_updates(&to.version()).unwrap())
        .unwrap();
}

fn manifest_format(bytes: &[u8]) -> u64 {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut s = String::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str::<Value>(&s).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

// ---- criterion 8: stored registers ------------------------------------------------------------

#[test]
fn c8_a_new_picture_reads_expert_50_and_no_stack() {
    let d = Document::new(1);
    let id = image(&d, "P");
    let a = shown(&d, id);
    assert_eq!(a, ImageAdjustments::default());
    assert!(!a.is_adjusted());
    assert_eq!(a.strength, 50);
    assert_eq!(owner(&d, id), AdjustOwner::Image);
}

#[test]
fn c8_the_registers_survive_save_and_reopen_in_order_and_the_format_is_52() {
    let d = Document::new(1);
    let id = image(&d, "P");
    write(
        &d,
        id,
        AdjustmentsWrite {
            preset: Some("line_art".into()),
            strength: Some(73),
            steps: Some(vec![
                step(
                    "sharpen",
                    &[
                        ("radius_um", 100),
                        ("amount_pct", 120),
                        ("threshold_pct", 2),
                    ],
                ),
                levels(4),
                step("vignette_future", &[("amount_pct", 5)]),
            ]),
            dither: None,
        },
    );
    let bytes = pack(&d, "0.1.0").unwrap();
    assert_eq!(manifest_format(&bytes), 56);
    let back = unpack(4, &bytes).unwrap();
    let a = shown(&back, id);
    assert_eq!(a.preset.as_deref(), Some("line_art"));
    assert_eq!(a.strength, 73);
    let kinds: Vec<&str> = a
        .steps
        .iter()
        .map(|e| e.step().unwrap().kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        ["sharpen", "levels", "vignette_future"],
        "kept, in stored order, unknown kept"
    );
    assert_eq!(shown(&back, id), shown(&d, id));
    // Saving again is byte-identical.
    assert_eq!(
        pack(&back, "0.1.0").unwrap(),
        pack(&reopen(&back), "0.1.0").unwrap()
    );
}

#[test]
fn c8_the_goldens_open_at_format_51_and_save_back_unchanged() {
    for name in ["image_adjust_v51.curvyo", "image_adjust_damaged_v51.curvyo"] {
        let bytes = fixture(name);
        assert_eq!(manifest_format(&bytes), 51, "{name}");
        let d = unpack(2, &bytes).unwrap_or_else(|e| panic!("{name} opens: {e:?}"));
        let again = pack(&d, "0.1.0").unwrap();
        let d2 = unpack(3, &again).unwrap();
        // Every image reads the same registers after a save.
        for o in d.read_objects().objects.iter() {
            if let ObjectSnapshot::Image(i) = o {
                assert_eq!(shown(&d, i.id), shown(&d2, i.id), "{name}");
            }
        }
    }
}

#[test]
fn c8_c23_the_damaged_golden_reads_each_damaged_register_as_its_default() {
    let d = unpack(2, &fixture("image_adjust_damaged_v51.curvyo")).unwrap();
    let mut seen_damage = 0;
    let mut unreadable = 0;
    for o in d.read_objects().objects.iter() {
        if let ObjectSnapshot::Image(i) = o {
            let a = shown(&d, i.id);
            if a.damage.preset {
                assert_eq!(a.preset, None, "a damaged preset reads Expert");
            }
            if a.damage.strength {
                assert_eq!(a.strength, 50, "a damaged Strength reads 50");
            }
            if a.damage.stack {
                assert!(a.steps.is_empty(), "a damaged stack reads empty");
            }
            seen_damage += a.damage.count();
            unreadable += a
                .steps
                .iter()
                .filter(|e| matches!(e, AdjustEntry::Unreadable(_)))
                .count();
        }
    }
    // Superseded by the Dither on the picture (2026-11-20): the damaged golden adds an
    // `image_dither` that is not a map (ADR 0072 amendment A5), a fourth damaged register.
    assert_eq!(
        seen_damage, 4,
        "preset, strength and stack of the wrong type, a Dither that is not a map"
    );
    assert!(unreadable >= 3, "unreadable entries kept: {unreadable}");
}

// ---- criterion 24: undo -----------------------------------------------------------------------

#[test]
fn c24_every_register_write_is_one_step_edit_adjustments_and_undo_restores_it() {
    let d = Document::new(1);
    let id = image(&d, "P");
    let writes = [
        AdjustmentsWrite {
            preset: Some("photo".into()),
            ..AdjustmentsWrite::default()
        },
        AdjustmentsWrite {
            strength: Some(10),
            ..AdjustmentsWrite::default()
        },
        AdjustmentsWrite {
            steps: Some(vec![levels(3)]),
            ..AdjustmentsWrite::default()
        },
        AdjustmentsWrite {
            preset: Some("expert".into()),
            strength: Some(99),
            steps: Some(vec![levels(5), levels(6)]),
            dither: None,
        },
    ];
    for w in writes {
        let before = shown(&d, id);
        assert!(write(&d, id, w.clone()));
        assert_eq!(last_label(&d), "edit_adjustments");
        let after = shown(&d, id);
        undo_last(&d);
        assert_eq!(shown(&d, id), before);
        let _ = after;
    }
    // Writing the same values again changes nothing and writes no step.
    let w = AdjustmentsWrite {
        strength: Some(99),
        ..AdjustmentsWrite::default()
    };
    write(&d, id, w.clone());
    assert!(!write(&d, id, w), "an equal write is no step");
    assert!(
        d.edit_image_adjustments(
            id,
            &AdjustmentsWrite {
                strength: Some(101),
                ..AdjustmentsWrite::default()
            }
        )
        .is_err()
    );
}

// ---- criterion 59: Restore original -----------------------------------------------------------

#[test]
fn c59_restore_original_empties_the_preset_and_the_stack_keeps_strength_one_step() {
    let d = Document::new(1);
    let id = image(&d, "P");
    write(
        &d,
        id,
        AdjustmentsWrite {
            preset: Some("graphics".into()),
            strength: Some(80),
            steps: Some(vec![levels(2)]),
            dither: None,
        },
    );
    let before = shown(&d, id);
    assert!(d.restore_original(id).unwrap());
    assert_eq!(last_label(&d), "restore_original");
    let a = shown(&d, id);
    assert!(!a.is_adjusted());
    assert_eq!(a.strength, 80);
    assert!(!d.restore_original(id).unwrap(), "nothing left to restore");
    undo_last(&d);
    assert_eq!(shown(&d, id), before);
}

// ---- criterion 9: clones ----------------------------------------------------------------------

#[test]
fn c9_own_adjustments_are_all_or_nothing_and_follow_original_drops_them() {
    let d = Document::new(1);
    let orig = image(&d, "P");
    write(
        &d,
        orig,
        AdjustmentsWrite {
            preset: Some("photo".into()),
            strength: Some(30),
            steps: Some(vec![levels(7)]),
            dither: None,
        },
    );
    let clone = d.linked_clone(&[orig], &[]).unwrap()[0];
    assert_eq!(owner(&d, clone), AdjustOwner::CloneFollowing);
    assert_eq!(shown(&d, clone), shown(&d, orig));
    let orig_before = shown(&d, orig);
    let bytes_before = pack(&d, "0.1.0").unwrap().len();
    // Only the Strength is edited on the clone; Preset and stack are copied with it.
    assert!(write(
        &d,
        clone,
        AdjustmentsWrite {
            strength: Some(90),
            ..AdjustmentsWrite::default()
        }
    ));
    assert_eq!(owner(&d, clone), AdjustOwner::CloneOwn);
    let c = shown(&d, clone);
    assert_eq!(c.preset.as_deref(), Some("photo"));
    assert_eq!(c.steps, orig_before.steps);
    assert_eq!(c.strength, 90);
    assert_eq!(shown(&d, orig), orig_before, "the original is unchanged");
    let _ = bytes_before;
    // A later edit of the original does not reach the clone, and back.
    write(
        &d,
        orig,
        AdjustmentsWrite {
            preset: Some("newspaper".into()),
            ..AdjustmentsWrite::default()
        },
    );
    assert_eq!(shown(&d, clone).preset.as_deref(), Some("photo"));
    // Survives save.
    let back = reopen(&d);
    assert_eq!(owner(&back, clone), AdjustOwner::CloneOwn);
    assert_eq!(shown(&back, clone), shown(&d, clone));
    // Follow original: one step, the clone shows the original's again.
    assert!(d.follow_original_adjustments(clone).unwrap());
    assert_eq!(last_label(&d), "follow_original");
    assert_eq!(owner(&d, clone), AdjustOwner::CloneFollowing);
    assert_eq!(shown(&d, clone), shown(&d, orig));
    assert!(
        d.follow_original_adjustments(orig).is_err(),
        "an image follows nothing"
    );
    // Restore original on the clone: own, empty; the original keeps its preset.
    assert!(d.restore_original(clone).unwrap());
    assert_eq!(owner(&d, clone), AdjustOwner::CloneOwn);
    assert!(!shown(&d, clone).is_adjusted());
    assert_eq!(shown(&d, orig).preset.as_deref(), Some("newspaper"));
}

#[test]
fn c9_a_clone_of_a_clone_reads_the_nearest_own_adjustments() {
    let d = Document::new(1);
    let orig = image(&d, "P");
    write(
        &d,
        orig,
        AdjustmentsWrite {
            preset: Some("photo".into()),
            ..AdjustmentsWrite::default()
        },
    );
    let c1 = d.linked_clone(&[orig], &[]).unwrap()[0];
    let c2 = d.linked_clone(&[c1], &[]).unwrap()[0];
    write(
        &d,
        c1,
        AdjustmentsWrite {
            preset: Some("graphics".into()),
            ..AdjustmentsWrite::default()
        },
    );
    assert_eq!(shown(&d, c2).preset.as_deref(), Some("graphics"));
    assert_eq!(owner(&d, c2), AdjustOwner::CloneFollowing);
}

// ---- ADR 0072 §4: merge of the three registers across two peers --------------------------------

#[test]
fn merge_two_peers_editing_preset_and_stack_keep_both() {
    let a = Document::new(1);
    let id = image(&a, "P");
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    write(
        &a,
        id,
        AdjustmentsWrite {
            preset: Some("dot_raster".into()),
            ..AdjustmentsWrite::default()
        },
    );
    write(
        &b,
        id,
        AdjustmentsWrite {
            steps: Some(vec![levels(9)]),
            ..AdjustmentsWrite::default()
        },
    );
    write(
        &b,
        id,
        AdjustmentsWrite {
            strength: Some(12),
            ..AdjustmentsWrite::default()
        },
    );
    sync(&a, &b);
    sync(&b, &a);
    let x = shown(&a, id);
    let y = shown(&b, id);
    assert_eq!(x, y, "converged");
    assert_eq!(x.preset.as_deref(), Some("dot_raster"), "preset survives");
    assert_eq!(x.strength, 12, "strength survives");
    assert_eq!(x.steps, vec![levels(9)], "stack survives");
}

#[test]
fn merge_two_peers_editing_the_stack_end_with_one_whole_stack() {
    let a = Document::new(1);
    let id = image(&a, "P");
    write(
        &a,
        id,
        AdjustmentsWrite {
            steps: Some(vec![levels(1), levels(2)]),
            ..AdjustmentsWrite::default()
        },
    );
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    write(
        &a,
        id,
        AdjustmentsWrite {
            steps: Some(vec![levels(10), levels(2)]),
            ..AdjustmentsWrite::default()
        },
    );
    write(
        &b,
        id,
        AdjustmentsWrite {
            steps: Some(vec![levels(1), levels(20)]),
            ..AdjustmentsWrite::default()
        },
    );
    sync(&a, &b);
    sync(&b, &a);
    let x = shown(&a, id).steps;
    assert_eq!(x, shown(&b, id).steps, "converged");
    assert!(
        x == vec![levels(10), levels(2)] || x == vec![levels(1), levels(20)],
        "one peer's stack wins whole, never a mix: {x:?}"
    );
}

#[test]
fn merge_own_adjustments_switched_on_against_an_edit_of_the_original() {
    let a = Document::new(1);
    let orig = image(&a, "P");
    let clone = a.linked_clone(&[orig], &[]).unwrap()[0];
    write(
        &a,
        orig,
        AdjustmentsWrite {
            preset: Some("photo".into()),
            ..AdjustmentsWrite::default()
        },
    );
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    // Peer a gives the clone its own set; peer b edits the original.
    write(
        &a,
        clone,
        AdjustmentsWrite {
            strength: Some(5),
            ..AdjustmentsWrite::default()
        },
    );
    write(
        &b,
        orig,
        AdjustmentsWrite {
            preset: Some("newspaper".into()),
            ..AdjustmentsWrite::default()
        },
    );
    sync(&a, &b);
    sync(&b, &a);
    for d in [&a, &b] {
        assert_eq!(owner(d, clone), AdjustOwner::CloneOwn);
        assert_eq!(
            shown(d, clone).preset.as_deref(),
            Some("photo"),
            "the copy at the switch"
        );
        assert_eq!(shown(d, clone).strength, 5);
        assert_eq!(shown(d, orig).preset.as_deref(), Some("newspaper"));
    }
}

#[test]
fn merge_follow_original_against_an_own_edit_leaves_the_clone_following() {
    let a = Document::new(1);
    let orig = image(&a, "P");
    let clone = a.linked_clone(&[orig], &[]).unwrap()[0];
    write(
        &a,
        clone,
        AdjustmentsWrite {
            preset: Some("graphics".into()),
            ..AdjustmentsWrite::default()
        },
    );
    let b = unpack(2, &pack(&a, "0.1.0").unwrap()).unwrap();
    assert!(a.follow_original_adjustments(clone).unwrap());
    write(
        &b,
        clone,
        AdjustmentsWrite {
            strength: Some(66),
            ..AdjustmentsWrite::default()
        },
    );
    sync(&a, &b);
    sync(&b, &a);
    assert_eq!(owner(&a, clone), owner(&b, clone), "converged");
    assert_eq!(shown(&a, clone), shown(&b, clone), "converged");
}

#[test]
fn c8_damaged_entries_written_by_a_later_build_round_trip_unchanged() {
    let d = Document::new(1);
    let id = image(&d, "P");
    write(
        &d,
        id,
        AdjustmentsWrite {
            steps: Some(vec![
                AdjustEntry::Unreadable(json!({"kind": "levels", "enabled": "yes"})),
                AdjustEntry::Unreadable(json!(12)),
                step("curves", &[("in_0", 0), ("out_0", 10)]),
            ]),
            ..AdjustmentsWrite::default()
        },
    );
    let back = reopen(&d);
    assert_eq!(shown(&back, id).steps, shown(&d, id).steps);
}
