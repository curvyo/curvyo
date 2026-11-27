//! Format 51 (`specs/0135-image-optimisation` slice 1, criteria 4, 8, 9, 24, 27, 46, 59, 61;
//! ADR 0072 §1 to §8): the three adjustment registers on an image, the `own_adjustments` marker on
//! a clone (the nearest own on the chain is the source), the commands "Edit adjustments",
//! "Restore original" and "Follow original", Unlink, Duplicate and the removal repair; the goldens
//! `image_adjust_v51.curvyo` (a preset picture, a manual stack, a clone with own adjustments, a
//! clone chain, a picture in two Image operations) and `image_adjust_damaged_v51.curvyo` (one image
//! per damage reason of ADR 0072 §5) read back as plain values and save back unchanged; a file of
//! the next format is refused. The laser extension's reading of the same goldens is in
//! `curvyo-laser-core/tests/image_adjust_goldens.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites the two
//! goldens of this file only, each from the one test that owns it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustDamage, AdjustEntry, AdjustError, AdjustOwner, AdjustStep, AdjustmentsWrite, Affine,
    CURRENT_FORMAT_VERSION, CopySource, Document, DocumentSize, FillPaint, FixtureParams,
    ImageAdjustments, Length, NewImage, NewOperation, NewOutputPage, NodeId, ObjectSnapshot,
    OpenError, Point, RectBounds, SettingsStep, SettingsWrite, StoredDither, StyleEdit, Vec2, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

const GOLDEN: &str = "image_adjust_v51.curvyo";
const DAMAGED: &str = "image_adjust_damaged_v51.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn stored(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

/// Under `CURVYO_WRITE_FIXTURES`, rewrites the golden `name` from `build` when a member other than
/// `document.loro` (its bytes differ per build) changed, atomically. The stored golden.
fn write_if_asked(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let path = fixture_path(name);
        let fresh = build();
        let comparable = |bytes: &[u8]| {
            let mut files = members(bytes);
            files.remove("document.loro");
            files
        };
        let unchanged =
            std::fs::read(&path).is_ok_and(|old| comparable(&old) == comparable(&fresh));
        if !unchanged {
            let temp = path.with_extension(format!("curvyo.{}", std::process::id()));
            std::fs::write(&temp, &fresh).unwrap();
            std::fs::rename(&temp, &path).unwrap();
        }
    }
    stored(name)
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn rezip(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

fn png() -> Arc<[u8]> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
    Arc::from(std::fs::read(path).unwrap().as_slice())
}

fn image(d: &Document, x: f64, name: &str) -> NodeId {
    d.add_images(&[NewImage {
        bytes: png(),
        placement: Affine::new(40.0, 0.0, 0.0, 30.0, x, 20.0),
        name: name.to_owned(),
    }])
    .unwrap()[0]
}

fn clone_of(d: &Document, of: NodeId) -> NodeId {
    let clone = d.linked_clone(&[of], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(0.0, 40.0)).unwrap();
    clone
}

fn step(kind: &str, enabled: bool, fields: &[(&str, i64)]) -> AdjustEntry {
    AdjustEntry::Step(AdjustStep {
        kind: kind.to_owned(),
        enabled,
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    })
}

fn levels() -> AdjustEntry {
    step(
        "levels",
        true,
        &[("black_pct", 3), ("white_pct", 97), ("gamma_centi", 110)],
    )
}

fn sharpen() -> AdjustEntry {
    step(
        "sharpen",
        false,
        &[
            ("radius_um", 100),
            ("amount_pct", 150),
            ("threshold_pct", 0),
        ],
    )
}

fn preset(name: &str) -> AdjustmentsWrite {
    AdjustmentsWrite {
        preset: Some(name.to_owned()),
        ..AdjustmentsWrite::default()
    }
}

fn steps(entries: Vec<AdjustEntry>) -> AdjustmentsWrite {
    AdjustmentsWrite {
        steps: Some(entries),
        ..AdjustmentsWrite::default()
    }
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

/// The image a clone draws, as the job and the Select view read it (criterion 61).
fn drawn_image(d: &Document, id: NodeId) -> ImageAdjustments {
    match d.object(id).unwrap() {
        ObjectSnapshot::Image(image) => image.adjustments,
        ObjectSnapshot::Clone(clone) => match clone.chain.picture.as_deref() {
            Some([ObjectSnapshot::Image(image)]) => image.adjustments.clone(),
            other => panic!("a clone of an image, got {other:?}"),
        },
        other => panic!("an image, got {other:?}"),
    }
}

fn laser(d: &Document) -> NodeId {
    let profile: FixtureParams = map(json!({
        "id": "generic-laser", "name": "Generic laser", "origin": "top_left",
        "max_feed_mm_per_min": 6000, "max_power": 1000, "laser_mode": true,
        "motorised_z": false,
    }));
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(&profile),
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn image_op(d: &Document, mfg: NodeId, name: &str, members: &[NodeId]) -> NodeId {
    let op = d
        .add_operation(
            mfg,
            &NewOperation {
                kind: "image",
                name,
                settings: None,
            },
            99,
            &[],
        )
        .unwrap();
    if !members.is_empty() {
        assign(d, op, members);
    }
    op
}

fn assign(d: &Document, op: NodeId, members: &[NodeId]) {
    let rows: Vec<Vec<NodeId>> = members.iter().map(|m| vec![*m]).collect();
    assert!(d.set_operation_members(op, &rows, true).unwrap());
}

fn interval(d: &Document, operation: NodeId, mm: f64) {
    let write = [SettingsWrite {
        operation,
        value: Some(map(json!({"main": {
            "power_pct": 40.0, "speed_mm_min": 6000, "passes": 1, "interval_mm": mm,
        }}))),
    }];
    assert!(
        d.write_operation_settings(SettingsStep::EditSettings, &write)
            .unwrap()
    );
}

fn options_of(d: &Document, op: NodeId) -> Option<Value> {
    let mfg = d.manufacturings()[0].id;
    d.operations(mfg)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op)
        .unwrap()
        .options
}

// ----- registers and the first clone edit ------------------------------------------------------

/// Criteria 1, 4, 8: a new picture has Expert, Strength 50 and no stack; an edit writes the
/// registers in one step "Edit adjustments" and undo brings back exactly what was there.
#[test]
fn an_edit_is_one_step_edit_adjustments_and_undo_restores_the_registers() {
    let d = Document::new(1);
    let picture = image(&d, 0.0, "Photo");
    assert_eq!(shown(&d, picture), ImageAdjustments::default());
    assert_eq!(owner(&d, picture), AdjustOwner::Image);
    assert!(
        d.edit_image_adjustments(picture, &steps(vec![levels(), sharpen()]))
            .unwrap()
    );
    assert_eq!(last_label(&d), "edit_adjustments");
    assert_eq!(shown(&d, picture).steps, vec![levels(), sharpen()]);
    assert!(
        !d.edit_image_adjustments(picture, &steps(vec![levels(), sharpen()]))
            .unwrap(),
        "nothing changes, no step"
    );
    let count = d.own_step_count();
    assert!(
        d.edit_image_adjustments(
            picture,
            &AdjustmentsWrite {
                strength: Some(80),
                ..preset("photo")
            }
        )
        .unwrap()
    );
    assert_eq!(
        d.own_step_count(),
        count + 1,
        "Preset and Strength in one step"
    );
    undo_last(&d);
    let back = shown(&d, picture);
    assert_eq!((back.preset, back.strength), (None, 50));
    assert_eq!(back.steps, vec![levels(), sharpen()]);
    assert_eq!(
        d.edit_image_adjustments(
            picture,
            &AdjustmentsWrite {
                strength: Some(101),
                ..AdjustmentsWrite::default()
            }
        ),
        Err(AdjustError::StrengthOutOfRange)
    );
}

/// Criterion 9: a clone shows its original's adjustments until its first edit, which copies them
/// whole (all or nothing) and then edits the copy; the original is unchanged by everything done
/// on the clone, and later edits of the original do not reach the clone.
#[test]
fn the_first_clone_edit_copies_the_originals_adjustments_whole() {
    let d = Document::new(1);
    let picture = image(&d, 0.0, "Photo");
    let clone = clone_of(&d, picture);
    d.edit_image_adjustments(
        picture,
        &AdjustmentsWrite {
            preset: Some("photo".into()),
            strength: Some(30),
            steps: Some(vec![levels()]),
            dither: None,
        },
    )
    .unwrap();
    assert_eq!(owner(&d, clone), AdjustOwner::CloneFollowing);
    assert_eq!(
        drawn_image(&d, clone),
        shown(&d, picture),
        "follows the original"
    );
    let original = pack(&d, "t").unwrap();
    let original_json = |bytes: &[u8]| -> Value {
        let json: Value = serde_json::from_slice(&members(bytes)["document.json"]).unwrap();
        json["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["shape"] == "image")
            .unwrap()
            .clone()
    };
    assert!(
        d.edit_image_adjustments(
            clone,
            &AdjustmentsWrite {
                strength: Some(90),
                ..AdjustmentsWrite::default()
            }
        )
        .unwrap()
    );
    assert_eq!(last_label(&d), "edit_adjustments");
    let first_clone_edit = d.own_steps().last().unwrap().step;
    assert_eq!(owner(&d, clone), AdjustOwner::CloneOwn);
    let own = drawn_image(&d, clone);
    assert_eq!(own.preset.as_deref(), Some("photo"));
    assert_eq!(own.strength, 90);
    assert_eq!(own.steps, vec![levels()], "the stack came with the copy");
    assert_eq!(
        original_json(&pack(&d, "t").unwrap()),
        original_json(&original),
        "the original's registers are byte-identical"
    );
    d.edit_image_adjustments(picture, &steps(vec![sharpen()]))
        .unwrap();
    assert_eq!(drawn_image(&d, clone).steps, vec![levels()], "independent");
    d.undo_step(first_clone_edit).unwrap();
    assert_eq!(
        owner(&d, clone),
        AdjustOwner::CloneFollowing,
        "undo of the first edit"
    );
}

/// Criterion 9, ADR 0072 §2: Original's (Follow original) removes the marker and the clone's
/// registers in one step "Follow original"; an image follows nothing.
#[test]
fn follow_original_drops_the_clones_own_adjustments() {
    let d = Document::new(1);
    let picture = image(&d, 0.0, "Photo");
    let clone = clone_of(&d, picture);
    d.edit_image_adjustments(clone, &preset("graphics"))
        .unwrap();
    assert!(d.follow_original_adjustments(clone).unwrap());
    assert_eq!(last_label(&d), "follow_original");
    assert_eq!(owner(&d, clone), AdjustOwner::CloneFollowing);
    assert_eq!(drawn_image(&d, clone), ImageAdjustments::default());
    assert!(!d.follow_original_adjustments(clone).unwrap());
    assert_eq!(
        d.follow_original_adjustments(picture),
        Err(AdjustError::NotAClone)
    );
    let json: Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let clone_json = json["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["shape"] == "clone")
        .unwrap();
    assert!(clone_json.get("own_adjustments").is_none());
    assert!(clone_json.get("adjustments").is_none(), "no residue");
}

/// ADR 0072 §2: the adjustment source is the nearest own on the chain, the clone itself first.
#[test]
fn a_chain_reads_the_nearest_own_adjustments() {
    let d = Document::new(1);
    let o = image(&d, 0.0, "O");
    let a = clone_of(&d, o);
    let b = clone_of(&d, a);
    d.edit_image_adjustments(o, &preset("photo")).unwrap();
    d.edit_image_adjustments(a, &preset("line_art")).unwrap();
    assert_eq!(drawn_image(&d, b).preset.as_deref(), Some("line_art"));
    assert_eq!(owner(&d, b), AdjustOwner::CloneFollowing);
    let ObjectSnapshot::Clone(read) = d.object(b).unwrap() else {
        panic!()
    };
    assert_eq!(read.chain.adjust_source, a);
    d.edit_image_adjustments(b, &preset("newspaper")).unwrap();
    assert_eq!(drawn_image(&d, b).preset.as_deref(), Some("newspaper"));
    assert_eq!(drawn_image(&d, a).preset.as_deref(), Some("line_art"));
    assert_eq!(drawn_image(&d, o).preset.as_deref(), Some("photo"));
}

/// Criterion 59: Restore original sets Expert and an empty stack, Strength kept, one step;
/// greyed (nothing written) when there is nothing to restore; on a clone it empties the clone's
/// own adjustments and the original keeps its improvements.
#[test]
fn restore_original_empties_the_picture_and_on_a_clone_its_own_set() {
    let d = Document::new(1);
    let picture = image(&d, 0.0, "Photo");
    let clone = clone_of(&d, picture);
    assert!(!d.restore_original(picture).unwrap(), "nothing to restore");
    d.edit_image_adjustments(
        picture,
        &AdjustmentsWrite {
            preset: Some("photo".into()),
            strength: Some(70),
            steps: Some(vec![levels()]),
            dither: None,
        },
    )
    .unwrap();
    assert!(d.restore_original(clone).unwrap());
    assert_eq!(last_label(&d), "restore_original");
    assert_eq!(owner(&d, clone), AdjustOwner::CloneOwn);
    let restored = drawn_image(&d, clone);
    assert!(!restored.is_adjusted());
    assert_eq!(restored.strength, 70, "Strength kept");
    assert!(
        shown(&d, picture).is_adjusted(),
        "the original keeps its improvements"
    );
    assert!(d.restore_original(picture).unwrap());
    let restored = shown(&d, picture);
    assert!(!restored.is_adjusted());
    assert_eq!((restored.strength, restored.steps.len()), (70, 0));
    undo_last(&d);
    assert_eq!(shown(&d, picture).steps, vec![levels()]);
}

fn dither(mode: &str, size_um: i64) -> StoredDither {
    StoredDither {
        mode: mode.to_owned(),
        fields: [
            ("threshold_pct", 50),
            ("screen_size_um", size_um),
            ("screen_angle_deg", 45),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect(),
    }
}

fn dither_write(mode: &str, size_um: i64) -> AdjustmentsWrite {
    AdjustmentsWrite {
        dither: Some(dither(mode, size_um)),
        ..AdjustmentsWrite::default()
    }
}

/// Criterion 27, superseded by the Dither on the picture (2026-11-20): a preset pick writes the
/// Preset and the picture's Dither in one step "Edit adjustments" and never an operation (ADR 0072
/// amendment A3; the old test wrote the operations' Dither in the same step).
#[test]
fn a_preset_pick_writes_the_pictures_dither_and_no_operation() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let picture = image(&d, 0.0, "Photo");
    let one = image_op(&d, mfg, "Image 1", &[picture]);
    let count = d.own_step_count();
    let pick = AdjustmentsWrite {
        preset: Some("photo".into()),
        dither: Some(dither("jarvis", 800)),
        ..AdjustmentsWrite::default()
    };
    assert!(d.edit_image_adjustments(picture, &pick).unwrap());
    assert_eq!(d.own_step_count(), count + 1, "one step");
    assert_eq!(last_label(&d), "edit_adjustments");
    assert_eq!(shown(&d, picture).dither, Some(dither("jarvis", 800)));
    assert_eq!(options_of(&d, one), None, "the operation is untouched");
    assert!(
        !d.edit_image_adjustments(picture, &pick).unwrap(),
        "no change, no step"
    );
    undo_last(&d);
    assert_eq!(shown(&d, picture).dither, None);
    assert_eq!(shown(&d, picture).preset, None);
}

/// Amendment A3: Restore original removes the Dither and is enabled by a stored Dither alone; on a
/// clone the own set it leaves has no Dither.
#[test]
fn restore_original_removes_the_dither_and_a_dither_alone_enables_it() {
    let d = Document::new(1);
    let picture = image(&d, 0.0, "Photo");
    let clone = clone_of(&d, picture);
    d.edit_image_adjustments(picture, &dither_write("halftone", 1_600))
        .unwrap();
    assert!(
        !shown(&d, picture).is_adjusted(),
        "the pixels are not adjusted"
    );
    assert!(shown(&d, picture).is_restorable());
    assert!(d.restore_original(clone).unwrap());
    assert_eq!(
        drawn_image(&d, clone).dither,
        None,
        "an own set without a Dither"
    );
    assert_eq!(owner(&d, clone), AdjustOwner::CloneOwn);
    assert!(d.restore_original(picture).unwrap());
    assert_eq!(last_label(&d), "restore_original");
    assert_eq!(shown(&d, picture).dither, None);
    assert!(!shown(&d, picture).is_restorable());
    assert!(
        !d.restore_original(picture).unwrap(),
        "nothing left to restore"
    );
    undo_last(&d);
    assert_eq!(shown(&d, picture).dither, Some(dither("halftone", 1_600)));
}

/// Amendment A2: the first Dither edit of a following clone copies all four registers, Follow
/// original removes the four, a following clone burns its source's Dither.
#[test]
fn a_clones_dither_is_part_of_its_own_adjustments() {
    let d = Document::new(1);
    let picture = image(&d, 0.0, "Photo");
    let clone = clone_of(&d, picture);
    d.edit_image_adjustments(
        picture,
        &AdjustmentsWrite {
            preset: Some("photo".into()),
            steps: Some(vec![levels()]),
            dither: Some(dither("jarvis", 800)),
            ..AdjustmentsWrite::default()
        },
    )
    .unwrap();
    assert_eq!(
        drawn_image(&d, clone).dither,
        Some(dither("jarvis", 800)),
        "follows"
    );
    d.edit_image_adjustments(clone, &dither_write("line_screen", 1_200))
        .unwrap();
    assert_eq!(owner(&d, clone), AdjustOwner::CloneOwn);
    let own = drawn_image(&d, clone);
    assert_eq!(own.dither, Some(dither("line_screen", 1_200)));
    assert_eq!(
        (own.preset.as_deref(), own.steps.clone()),
        (Some("photo"), vec![levels()])
    );
    assert_eq!(
        shown(&d, picture).dither,
        Some(dither("jarvis", 800)),
        "original kept"
    );
    d.edit_image_adjustments(picture, &dither_write("stucki", 800))
        .unwrap();
    assert_eq!(
        drawn_image(&d, clone).dither,
        Some(dither("line_screen", 1_200))
    );
    assert!(d.follow_original_adjustments(clone).unwrap());
    assert_eq!(drawn_image(&d, clone).dither, Some(dither("stucki", 800)));
    let json: Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let image_json = json["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["shape"] == "image")
        .unwrap();
    assert_eq!(
        image_json["adjustments"]["dither"],
        json!({"mode": "stucki", "threshold_pct": 50, "screen_size_um": 800, "screen_angle_deg": 45}),
        "document.json lists the Dither"
    );
}

/// Amendment A6: two peers, last writer wins per register. A Dither change against a stack edit
/// keeps both; a preset pick against another peer's Dither change keeps the preset and one whole
/// Dither.
#[test]
fn two_peers_merge_the_dither_per_register() {
    let a = Document::new(1);
    let picture = image(&a, 0.0, "Photo");
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    a.edit_image_adjustments(picture, &dither_write("halftone", 1_600))
        .unwrap();
    b.edit_image_adjustments(picture, &steps(vec![levels()]))
        .unwrap();
    let sync = |from: &Document, to: &Document| {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    };
    sync(&b, &a);
    sync(&a, &b);
    for d in [&a, &b] {
        let read = shown(d, picture);
        assert_eq!(read.dither, Some(dither("halftone", 1_600)));
        assert_eq!(read.steps, vec![levels()]);
    }
    a.edit_image_adjustments(
        picture,
        &AdjustmentsWrite {
            preset: Some("newspaper".into()),
            dither: Some(dither("halftone", 1_000)),
            ..AdjustmentsWrite::default()
        },
    )
    .unwrap();
    b.edit_image_adjustments(picture, &dither_write("grey", 800))
        .unwrap();
    sync(&b, &a);
    sync(&a, &b);
    let (ra, rb) = (shown(&a, picture), shown(&b, picture));
    assert_eq!(ra, rb, "both peers agree");
    assert_eq!(
        ra.preset.as_deref(),
        Some("newspaper"),
        "the preset survives"
    );
    assert!(
        [Some(dither("halftone", 1_000)), Some(dither("grey", 800))].contains(&ra.dither),
        "one Dither wins whole: {:?}",
        ra.dither
    );
}

/// Criterion 9: Unlink gives the new image the adjustments it showed; Duplicate copies the
/// clone's own set; a clone that loses its adjustment source to a removal keeps what it showed.
#[test]
fn unlink_duplicate_and_repair_keep_what_the_clone_showed() {
    let d = Document::new(1);
    let o = image(&d, 0.0, "O");
    let a = clone_of(&d, o);
    let b = clone_of(&d, a);
    d.edit_image_adjustments(
        a,
        &AdjustmentsWrite {
            dither: Some(dither("atkinson", 800)),
            ..steps(vec![levels()])
        },
    )
    .unwrap();
    let copy = d
        .duplicate_objects(
            &[CopySource {
                id: a,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 0.0),
        )
        .unwrap()[0];
    assert_eq!(owner(&d, copy), AdjustOwner::CloneOwn);
    assert_eq!(drawn_image(&d, copy).steps, vec![levels()]);
    assert_eq!(
        drawn_image(&d, copy).dither,
        Some(dither("atkinson", 800)),
        "Duplicate"
    );
    d.delete_objects(&[a]).unwrap();
    assert_eq!(
        owner(&d, b),
        AdjustOwner::CloneOwn,
        "repair: a copy of what it showed"
    );
    assert_eq!(drawn_image(&d, b).steps, vec![levels()]);
    assert_eq!(
        drawn_image(&d, b).dither,
        Some(dither("atkinson", 800)),
        "repair"
    );
    d.unlink_clones(&[CopySource {
        id: b,
        anchor_ids: Vec::new(),
    }])
    .unwrap();
    assert_eq!(owner(&d, b), AdjustOwner::Image);
    assert_eq!(
        shown(&d, b).steps,
        vec![levels()],
        "the effective adjustments"
    );
    assert_eq!(shown(&d, b).dither, Some(dither("atkinson", 800)), "Unlink");
    let picture = image(&d, 100.0, "P");
    d.edit_image_adjustments(picture, &preset("graphics"))
        .unwrap();
    let follower = clone_of(&d, picture);
    d.unlink_clones(&[CopySource {
        id: follower,
        anchor_ids: Vec::new(),
    }])
    .unwrap();
    assert_eq!(shown(&d, follower).preset.as_deref(), Some("graphics"));
}

// ----- goldens ----------------------------------------------------------------------------------

struct Golden {
    document: Document,
    preset_picture: NodeId,
    manual: NodeId,
    own_clone: NodeId,
    chain: NodeId,
}

fn angled(mut dither: StoredDither, degrees: i64) -> StoredDither {
    dither.fields.insert("screen_angle_deg".into(), degrees);
    dither
}

/// A rectangle filled with the first gradient, a non-picture raster leaf (amendment A4).
fn gradient_rect(d: &Document, x: f64, y: f64) -> NodeId {
    let rect = d.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(40.0),
        height: Length::from_mm(20.0),
    });
    d.rename_node(rect, "Gradient").unwrap();
    d.edit_style(&[rect], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    rect
}

/// The golden: a preset picture in two Image operations (criterion 60) holding the halftone the
/// coarse operation held, a manual stack holding Jarvis, a clone with own adjustments and its own
/// line screen and a clone of that clone (a chain); the coarse operation keeps its Dither keys
/// over one gradient rectangle (superseded by the Dither on the picture, 2026-11-20: the
/// operations held the pictures' Dither, ADR 0072 amendment A8).
fn golden() -> Golden {
    let d = Document::new(1);
    let mfg = laser(&d);
    let o = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing() == Some(mfg))
        .unwrap()
        .origin;
    let preset_picture = image(&d, o.x + 10.0, "Photo");
    d.edit_image_adjustments(
        preset_picture,
        &AdjustmentsWrite {
            preset: Some("photo".into()),
            strength: Some(65),
            steps: None,
            dither: Some(angled(dither("halftone", 1_600), 30)),
        },
    )
    .unwrap();
    let manual = image(&d, o.x + 60.0, "Manual");
    d.edit_image_adjustments(
        manual,
        &AdjustmentsWrite {
            dither: Some(dither("jarvis", 800)),
            ..steps(vec![
                step("denoise", true, &[("strength_pct", 30), ("detail_pct", 60)]),
                levels(),
                step(
                    "local_contrast",
                    true,
                    &[("radius_um", 3_000), ("amount_pct", 50)],
                ),
                step(
                    "contours",
                    true,
                    &[("width_um", 300), ("sensitivity_pct", 50)],
                ),
                sharpen(),
            ])
        },
    )
    .unwrap();
    let own_clone = clone_of(&d, preset_picture);
    d.edit_image_adjustments(
        own_clone,
        &AdjustmentsWrite {
            preset: Some("dot_raster".into()),
            dither: Some(angled(dither("line_screen", 1_200), 15)),
            ..AdjustmentsWrite::default()
        },
    )
    .unwrap();
    let chain = clone_of(&d, own_clone);
    // No two adjacent steps share a label (`specs/0020` criterion 36): each operation's Interval
    // and options are written around its members.
    let fine = image_op(&d, mfg, "Image 0.1", &[]);
    interval(&d, fine, 0.1);
    assign(&d, fine, &[preset_picture, own_clone, chain]);
    let coarse = image_op(&d, mfg, "Image 0.2", &[]);
    interval(&d, coarse, 0.2);
    let ramp = gradient_rect(&d, o.x + 10.0, o.y + 150.0);
    assign(&d, coarse, &[preset_picture, manual, ramp]);
    d.write_operation_options(
        coarse,
        &map(json!({"dither": "halftone", "screen_size_mm": 1.6, "screen_angle_deg": 30})),
    )
    .unwrap();
    Golden {
        document: d,
        preset_picture,
        manual,
        own_clone,
        chain,
    }
}

fn golden_bytes() -> Vec<u8> {
    pack(&golden().document, "0.1.0").unwrap()
}

/// One image per damage reason of ADR 0072 §5, and a clone whose marker is not a bool.
fn damaged_document() -> Document {
    let d = Document::new(1);
    let unknown = image(&d, 0.0, "Unknown preset");
    d.edit_image_adjustments(unknown, &preset("sepia")).unwrap();
    let entries = image(&d, 50.0, "Damaged entries");
    d.edit_image_adjustments(
        entries,
        &steps(vec![
            step("vignette", true, &[("amount_pct", 5)]),
            step("levels", true, &[("black_pct", 3), ("white_pct", 97)]),
            step(
                "sharpen",
                true,
                &[
                    ("radius_um", 100),
                    ("amount_pct", 501),
                    ("threshold_pct", 0),
                ],
            ),
            AdjustEntry::Unreadable(json!({"kind": "levels", "enabled": "yes", "fields": {}})),
            AdjustEntry::Unreadable(json!(
                {"kind": "denoise", "enabled": true, "fields": {"strength_pct": 2.5}}
            )),
            AdjustEntry::Unreadable(json!(12)),
            levels(),
        ]),
    )
    .unwrap();
    // Amendment A5: the Dither cases the extension reads as Floyd-Steinberg; document-core reads
    // their shape (the `not a map` case is written raw by `damaged_bytes`).
    let mut missing = dither("halftone", 800);
    missing.fields.remove("screen_angle_deg");
    let cases = [
        ("Dither unknown mode", dither("halftone_x", 800)),
        ("Dither missing field", missing),
        ("Dither too large", dither("halftone", 6_000)),
        ("Dither off the step", dither("halftone", 805)),
    ];
    for (x, (name, case)) in (0_u32..).zip(cases) {
        let picture = image(&d, 150.0 + 50.0 * f64::from(x), name);
        d.edit_image_adjustments(
            picture,
            &AdjustmentsWrite {
                dither: Some(case),
                ..AdjustmentsWrite::default()
            },
        )
        .unwrap();
    }
    d
}

/// The damaged golden: `damaged_document` plus an image whose three registers are of the wrong
/// type and a clone of it whose marker is not a bool, written raw into `document.loro` (no
/// command writes them), then opened and saved so every member agrees.
fn damaged_bytes() -> Vec<u8> {
    let d = damaged_document();
    let typed = image(&d, 100.0, "Wrong types");
    let clone = clone_of(&d, typed);
    d.rename_node(clone, "Clone of wrong types").unwrap();
    // After the rename, so no two adjacent steps share a label (`specs/0020` criterion 36).
    image(&d, 400.0, "Dither not a map");
    let bytes = pack(&d, "0.1.0").unwrap();
    let loro = LoroDoc::new();
    loro.import(&members(&bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let raw = |value: Value| -> LoroValue { serde_json::from_value(value).unwrap() };
    let meta = |name: &str| {
        let mut stack = tree.children(TreeParentId::Root).unwrap();
        while let Some(node) = stack.pop() {
            let meta = tree.get_meta(node).unwrap();
            if meta.get("name").and_then(|v| v.into_value().ok()) == Some(LoroValue::from(name)) {
                return meta;
            }
            stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
        }
        panic!("no node {name}")
    };
    let typed = meta("Wrong types");
    typed.insert("image_preset", raw(json!(7))).unwrap();
    typed.insert("image_strength", raw(json!(101))).unwrap();
    typed
        .insert("image_adjustments", raw(json!("levels")))
        .unwrap();
    meta("Dither not a map")
        .insert("image_dither", raw(json!("halftone")))
        .unwrap();
    meta("Clone of wrong types")
        .insert("own_adjustments", raw(json!("true")))
        .unwrap();
    loro.commit();
    let mut files = members(&bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    pack(&unpack(3, &rezip(&files)).unwrap(), "0.1.0").unwrap()
}

fn name_of(d: &Document, id: NodeId) -> String {
    d.node_registers(id).unwrap().name.unwrap_or_default()
}

fn by_name(d: &Document, name: &str) -> NodeId {
    d.read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .find(|id| name_of(d, *id) == name)
        .unwrap()
}

/// A `document.json` saved by this build as the format-51 golden stores it.
fn as_format_51(json: &[u8]) -> String {
    String::from_utf8(json.to_vec()).unwrap().replacen(
        "\"format_version\": 71,",
        "\"format_version\": 51,",
        1,
    )
}

#[test]
fn the_format_is_52() {
    assert_eq!(CURRENT_FORMAT_VERSION, 71);
}

#[test]
fn the_golden_reads_back_every_register() {
    let bytes = write_if_asked(GOLDEN, golden_bytes);
    assert_eq!(format_of(&bytes), 51);
    let g = golden();
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let photo = shown(&d, g.preset_picture);
    assert_eq!(
        (photo.preset.as_deref(), photo.strength),
        (Some("photo"), 65)
    );
    assert_eq!(
        shown(&d, g.manual).steps,
        shown(&g.document, g.manual).steps
    );
    assert_eq!(shown(&d, g.manual).steps.len(), 5);
    assert_eq!(owner(&d, g.own_clone), AdjustOwner::CloneOwn);
    let own = drawn_image(&d, g.own_clone);
    assert_eq!(
        (own.preset.as_deref(), own.strength),
        (Some("dot_raster"), 65)
    );
    assert_eq!(owner(&d, g.chain), AdjustOwner::CloneFollowing);
    assert_eq!(
        drawn_image(&d, g.chain),
        own,
        "the chain reads the nearest own"
    );
    let mfg = d.manufacturings()[0].id;
    let ops = d.operations(mfg).unwrap();
    assert_eq!(ops.len(), 2);
    assert!(
        ops.iter()
            .all(|op| op.members.contains(&vec![g.preset_picture]))
    );
    assert_eq!(
        ops[1].options,
        Some(json!({"dither": "halftone", "screen_size_mm": 1.6, "screen_angle_deg": 30})),
        "the coarse operation keeps its keys for its gradient leaf"
    );
    assert_eq!(
        ops[0].options, None,
        "no operation holds a picture's Dither"
    );
    assert_eq!(photo.dither, Some(angled(dither("halftone", 1_600), 30)));
    assert_eq!(shown(&d, g.manual).dither, Some(dither("jarvis", 800)));
    assert_eq!(own.dither, Some(angled(dither("line_screen", 1_200), 15)));
}

#[test]
fn the_golden_saves_back_unchanged_and_twice_the_same() {
    let bytes = stored(GOLDEN);
    let d = unpack(2, &bytes).unwrap();
    let first = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        first,
        pack(&d, "0.1.0").unwrap(),
        "no adjusted pixel is stored"
    );
    // Saved by a build of format 52 (slice 2): the same content under its own version.
    assert_eq!(
        as_format_51(&members(&first)["document.json"]),
        as_format_51(&members(&bytes)["document.json"])
    );
    // ADR 0084 §1: a save since format 58 adds the page list of the library.
    assert_eq!(
        members(&first)
            .keys()
            .filter(|name| *name != "library.json")
            .collect::<Vec<_>>(),
        members(&bytes).keys().collect::<Vec<_>>(),
        "the picture is stored once, in assets/"
    );
}

/// ADR 0072 §5, one image per reason: the file opens, each damaged register reads as its default
/// and is named, every entry is kept, a non-bool marker reads false, and nothing is repaired.
#[test]
fn the_damaged_golden_opens_and_keeps_every_damaged_value() {
    let bytes = write_if_asked(DAMAGED, damaged_bytes);
    assert_eq!(format_of(&bytes), 51);
    let d = unpack(2, &bytes).unwrap();
    let unknown = shown(&d, by_name(&d, "Unknown preset"));
    assert_eq!(
        unknown.preset.as_deref(),
        Some("sepia"),
        "unknown text is the extension's"
    );
    let entries = shown(&d, by_name(&d, "Damaged entries")).steps;
    assert_eq!(entries.len(), 7);
    assert_eq!(entries[0].step().unwrap().kind, "vignette");
    assert!(entries[3..6].iter().all(|e| e.step().is_none()));
    assert_eq!(entries[6], levels());
    let typed = shown(&d, by_name(&d, "Wrong types"));
    assert_eq!(
        typed.damage,
        AdjustDamage {
            preset: true,
            strength: true,
            stack: true,
            dither: false,
        }
    );
    assert_eq!((typed.preset, typed.strength), (None, 50));
    let not_a_map = shown(&d, by_name(&d, "Dither not a map"));
    assert!(not_a_map.damage.dither && not_a_map.dither.is_none());
    for name in [
        "Dither unknown mode",
        "Dither missing field",
        "Dither too large",
        "Dither off the step",
    ] {
        let read = shown(&d, by_name(&d, name));
        assert!(
            read.dither.is_some() && !read.damage.dither,
            "{name}: the extension's"
        );
    }
    let clone = d
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.id()),
            _ => None,
        })
        .unwrap();
    assert_eq!(owner(&d, clone), AdjustOwner::CloneFollowing, "reads false");
    let saved = pack(&d, "0.1.0").unwrap();
    // Saved by a build of format 52 (slice 2).
    assert_eq!(
        as_format_51(&members(&saved)["document.json"]),
        as_format_51(&members(&bytes)["document.json"]),
        "never repaired on read"
    );
}

#[test]
fn a_file_of_the_next_format_is_refused() {
    let bytes = stored(GOLDEN);
    let mut files = members(&bytes);
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["format_version"] = json!(CURRENT_FORMAT_VERSION + 1);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    assert!(matches!(
        unpack(2, &rezip(&files)),
        Err(OpenError::FormatTooNew {
            found: 72,
            supported: 71
        })
    ));
}
