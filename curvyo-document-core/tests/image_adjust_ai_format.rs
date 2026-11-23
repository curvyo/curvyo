//! Format 53 (`specs/0135-image-optimisation` slice 3, criteria 48, 59, 66; ADR 0077): the AI
//! kinds `upscale`, `ai_denoise`, `remove_background` with their `result` record, and the stored
//! results as `assets/derived/<sha256>.png`. The golden `image_adjust_ai_v53.curvyo` holds one
//! stored result per kind (the stand-in PNGs of `curvyo-image-core`, criterion 54); the damaged
//! golden `image_adjust_ai_damaged_v53.curvyo` a wrong hash, a missing member and a malformed
//! `result`. `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, AssetError, AssetHash,
    CURRENT_FORMAT_VERSION, Document, MAX_DERIVED_ASSET_BYTES, NewImage, NodeId, ObjectSnapshot,
    RUN_UPSCALE_COMMIT_LABEL, RanStep, StepResult, pack, unpack,
};
use serde_json::Value;

const GOLDEN: &str = "image_adjust_ai_v53.curvyo";
const DAMAGED: &str = "image_adjust_ai_damaged_v53.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
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
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in files {
        writer.start_file(name.as_str(), options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn png() -> Arc<[u8]> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
    Arc::from(std::fs::read(path).unwrap().as_slice())
}

/// The stand-in raw outputs of criterion 54.
fn stand_in(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/adjust")
        .join(format!("{name}.png"));
    std::fs::read(path).unwrap()
}

fn step(kind: &str, fields: &[(&str, i64)]) -> AdjustStep {
    AdjustStep {
        kind: kind.to_owned(),
        enabled: true,
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    }
}

fn ran(step: AdjustStep, asset: AssetHash, model: &str, size: (u32, u32)) -> AdjustEntry {
    AdjustEntry::Ran(RanStep {
        step,
        result: StepResult {
            asset,
            model: model.to_owned(),
            model_version: "1".to_owned(),
            input_hash: AssetHash::of(model.as_bytes()),
            width_px: size.0,
            height_px: size.1,
        },
    })
}

fn image(d: &Document, x: f64, name: &str) -> NodeId {
    d.add_images(&[NewImage {
        bytes: png(),
        placement: Affine::new(40.0, 0.0, 0.0, 30.0, x, 20.0),
        name: name.to_owned(),
    }])
    .unwrap()[0]
}

/// One stored result per kind, in pipeline order, and an AI step not run yet.
fn ai_stack(d: &Document) -> Vec<AdjustEntry> {
    let up = d
        .insert_derived_asset(&stand_in("upscale_stored_2x"))
        .unwrap();
    let dn = d
        .insert_derived_asset(&stand_in("denoise_ai_stored"))
        .unwrap();
    let mask = d.insert_derived_asset(&stand_in("mask_stored")).unwrap();
    vec![
        ran(
            step("upscale", &[("factor", 2)]),
            up,
            "realesr-general-x4v3",
            (128, 128),
        ),
        ran(
            step("ai_denoise", &[("strength_pct", 50)]),
            dn,
            "scunet",
            (64, 64),
        ),
        AdjustEntry::Step(step(
            "levels",
            &[("black_pct", 0), ("white_pct", 100), ("gamma_centi", 100)],
        )),
        ran(
            step(
                "remove_background",
                &[("softness_pct", 5), ("threshold_pct", 50)],
            ),
            mask,
            "u2netp",
            (20, 20),
        ),
    ]
}

fn write(d: &Document, id: NodeId, entries: Vec<AdjustEntry>) {
    let write = AdjustmentsWrite {
        steps: Some(entries),
        ..AdjustmentsWrite::default()
    };
    assert!(d.edit_image_adjustments(id, &write).unwrap());
}

fn golden_document() -> Document {
    let d = Document::new(1);
    let picture = image(&d, 10.0, "AI");
    let stack = ai_stack(&d);
    write(&d, picture, stack);
    let pending = image(&d, 60.0, "Not run");
    write(
        &d,
        pending,
        vec![AdjustEntry::Step(step("upscale", &[("factor", 4)]))],
    );
    d
}

fn read_or_write(name: &str, make: impl Fn() -> Vec<u8>) -> Vec<u8> {
    let path = fixture_path(name);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let bytes = make();
        let comparable = |bytes: &[u8]| {
            let mut files = members(bytes);
            files.remove("document.loro");
            files
        };
        let unchanged = std::fs::read(&path).is_ok_and(|old| {
            zip::ZipArchive::new(Cursor::new(&old)).is_ok()
                && comparable(&old) == comparable(&bytes)
        });
        if !unchanged {
            std::fs::write(&path, &bytes).unwrap();
        }
    }
    std::fs::read(path).unwrap()
}

/// The golden's `document.json` as this build writes it: the golden is a format 53 file, the
/// save says 54 (`specs/0141-parametric-generators`); everything else must be equal.
fn document_json_at_54(bytes: &[u8]) -> Vec<u8> {
    String::from_utf8(members(bytes)["document.json"].clone())
        .unwrap()
        .replacen(
            "\"format_version\": 53",
            &format!("\"format_version\": {CURRENT_FORMAT_VERSION}"),
            1,
        )
        .into_bytes()
}

fn golden() -> Vec<u8> {
    read_or_write(GOLDEN, || pack(&golden_document(), "0.1.0").unwrap())
}

/// The golden with a wrong-hash member (the mask's bytes under the denoise hash), the upscale
/// member missing, and a `result` without its `model`.
fn damaged() -> Vec<u8> {
    read_or_write(DAMAGED, || {
        let mut files = members(&golden());
        let names: Vec<String> = files
            .keys()
            .filter(|n| n.starts_with("assets/derived/"))
            .cloned()
            .collect();
        let d = unpack(3, &golden()).unwrap();
        let ai = by_name(&d, "AI");
        let hash = |i: usize| {
            ai.adjustments.steps[i]
                .result()
                .unwrap()
                .asset
                .as_str()
                .to_owned()
        };
        let member = |h: &str| names.iter().find(|n| n.contains(h)).unwrap().clone();
        let mask_bytes = files[&member(&hash(3))].clone();
        files.insert(member(&hash(1)), mask_bytes);
        files.remove(&member(&hash(0)));
        let json = String::from_utf8(files["document.json"].clone()).unwrap();
        // The loro snapshot is the truth on open; a malformed result is written through it.
        let d = unpack(4, &rezip(&files)).unwrap();
        let mut steps = by_name(&d, "AI").adjustments.steps;
        steps.push(AdjustEntry::Unreadable(serde_json::json!({
            "kind": "upscale", "enabled": true, "fields": {"factor": 2},
            "result": {"asset": hash(0), "model_version": "1"}
        })));
        let id = by_id(&d, "AI");
        // A Run's label, so the history has no two adjacent steps of one label (`0020` c36).
        assert!(
            d.run_image_ai_step(
                id,
                &AdjustmentsWrite {
                    steps: Some(steps),
                    ..AdjustmentsWrite::default()
                },
                curvyo_document_core::RUN_UPSCALE_COMMIT_LABEL,
            )
            .unwrap()
        );
        let mut out = members(&pack(&d, "0.1.0").unwrap());
        drop(json);
        // Put the wrong-hash member back: the save left out what this build could not read.
        out.insert(member(&hash(1)), files[&member(&hash(1))].clone());
        rezip(&out)
    })
}

fn by_id(d: &Document, name: &str) -> NodeId {
    d.read_objects()
        .objects
        .iter()
        .find(|o| d.node_registers(o.id()).unwrap().name.as_deref() == Some(name))
        .unwrap()
        .id()
}

fn by_name(d: &Document, name: &str) -> curvyo_document_core::ImageSnapshot {
    match d
        .read_objects()
        .objects
        .iter()
        .find(|o| d.node_registers(o.id()).unwrap().name.as_deref() == Some(name))
    {
        Some(ObjectSnapshot::Image(image)) => image.clone(),
        other => panic!("an image, got {other:?}"),
    }
}

#[test]
fn the_golden_is_format_53_and_reads_back_every_stored_result() {
    let bytes = golden();
    let files = members(&bytes);
    let manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest["format_version"], 53);
    let derived: Vec<&String> = files
        .keys()
        .filter(|n| n.starts_with("assets/derived/"))
        .collect();
    assert_eq!(derived.len(), 3, "one stored result per kind: {derived:?}");
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let ai = by_name(&d, "AI");
    assert_eq!(ai.adjustments.steps, ai_stack(&Document::new(9)));
    for entry in &ai.adjustments.steps {
        if let Some(result) = entry.result() {
            assert!(d.has_derived_asset(&result.asset));
            let bytes = d.asset(&result.asset).unwrap();
            assert_eq!(
                AssetHash::of(&bytes),
                result.asset,
                "the member is its hash"
            );
        }
    }
    assert_eq!(
        by_name(&d, "Not run").adjustments.steps[0].result(),
        None,
        "a step not run yet has no result"
    );
    let total: u64 = derived.iter().map(|n| files[*n].len() as u64).sum();
    assert_eq!(d.derived_asset_bytes(), total);
}

#[test]
fn the_golden_saves_back_unchanged_and_derived_members_come_after_the_originals() {
    let bytes = golden();
    let d = unpack(2, &bytes).unwrap();
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        members(&saved)["document.json"],
        document_json_at_54(&bytes)
    );
    assert_eq!(saved, pack(&d, "0.1.0").unwrap(), "a save is deterministic");
    let mut archive = zip::ZipArchive::new(Cursor::new(&saved)).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().unwrap().to_string())
        .collect();
    let first_derived = names
        .iter()
        .position(|n| n.starts_with("assets/derived/"))
        .unwrap();
    assert!(
        names[..first_derived]
            .iter()
            .any(|n| n.starts_with("assets/"))
    );
    assert!(
        names[first_derived..]
            .iter()
            .all(|n| n.starts_with("assets/derived/"))
    );
    let mut sorted = names[first_derived..].to_vec();
    sorted.sort();
    assert_eq!(sorted, names[first_derived..], "in hash order");
    for i in first_derived..names.len() {
        let file = archive.by_index(i).unwrap();
        assert_eq!(file.compression(), zip::CompressionMethod::Stored);
    }
}

/// ADR 0077 §1, §9: a wrong-hash or missing member is a missing result; a malformed `result`
/// keeps the entry as stored. The file opens and keeps every stored value.
#[test]
fn the_damaged_golden_opens_and_keeps_what_it_cannot_read() {
    let bytes = damaged();
    let d = unpack(2, &bytes).unwrap();
    let ai = by_name(&d, "AI");
    let steps = &ai.adjustments.steps;
    assert_eq!(steps.len(), 5);
    assert!(
        !d.has_derived_asset(&steps[0].result().unwrap().asset),
        "missing member"
    );
    assert!(
        !d.has_derived_asset(&steps[1].result().unwrap().asset),
        "wrong hash"
    );
    assert!(d.has_derived_asset(&steps[3].result().unwrap().asset));
    assert!(
        matches!(steps[4], AdjustEntry::Unreadable(_)),
        "malformed result"
    );
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        members(&saved)["document.json"],
        document_json_at_54(&bytes),
        "never repaired"
    );
}

/// Criterion 66, ADR 0077 §6: one result at most 64 MiB, refused before anything is stored.
#[test]
fn a_result_over_64_mib_is_refused() {
    let d = Document::new(1);
    let big = vec![0_u8; usize::try_from(MAX_DERIVED_ASSET_BYTES).unwrap() + 1];
    assert_eq!(
        d.insert_derived_asset(&big),
        Err(AssetError::ResultTooLarge {
            bytes: MAX_DERIVED_ASSET_BYTES + 1
        })
    );
    assert_eq!(d.derived_asset_bytes(), 0);
    assert_eq!(d.asset_bytes(), 0, "counted apart from the pictures");
}

/// ADR 0077 §5: a save keeps what the tree or this session refers to and drops the rest; a file
/// opened with an unreferenced member drops it at its first save.
#[test]
fn a_save_prunes_results_nothing_refers_to() {
    let d = Document::new(1);
    let id = image(&d, 0.0, "P");
    let first = d.insert_derived_asset(&stand_in("mask_stored")).unwrap();
    let mask = step(
        "remove_background",
        &[("softness_pct", 5), ("threshold_pct", 50)],
    );
    write(
        &d,
        id,
        vec![ran(mask.clone(), first.clone(), "u2netp", (20, 20))],
    );
    // A re-run replaces the result; the first stays for this session's undo.
    let second = d
        .insert_derived_asset(&stand_in("denoise_ai_stored"))
        .unwrap();
    assert!(
        d.run_image_ai_step(
            id,
            &AdjustmentsWrite {
                steps: Some(vec![ran(mask, second.clone(), "u2netp", (64, 64))]),
                ..AdjustmentsWrite::default()
            },
            RUN_UPSCALE_COMMIT_LABEL,
        )
        .unwrap()
    );
    let saved = members(&pack(&d, "0.1.0").unwrap());
    let has = |h: &AssetHash| saved.contains_key(&format!("assets/derived/{}.png", h.as_str()));
    assert!(has(&first) && has(&second), "this session's results stay");
    // Another session opens the file: the first result is referred to by nothing.
    let reopened = unpack(2, &rezip(&saved)).unwrap();
    let again = members(&pack(&reopened, "0.1.0").unwrap());
    let kept = |h: &AssetHash| again.contains_key(&format!("assets/derived/{}.png", h.as_str()));
    assert!(!kept(&first), "pruned at save");
    assert!(kept(&second));
}

/// ADR 0077 §5 (tester F2): a file opened with a referenced result keeps it through a save made
/// after the reference was removed, so this session's undo can bring it back.
#[test]
fn a_result_referenced_at_open_survives_a_save_after_its_removal() {
    let bytes = golden();
    let d = unpack(2, &bytes).unwrap();
    let id = by_id(&d, "AI");
    let steps = by_name(&d, "AI").adjustments.steps;
    let asset = steps[3].result().unwrap().asset.clone();
    let mut without = steps.clone();
    without[3] = AdjustEntry::Step(without[3].step().unwrap().clone());
    write(&d, id, without);
    let saved = members(&pack(&d, "0.1.0").unwrap());
    assert!(
        saved.contains_key(&format!("assets/derived/{}.png", asset.as_str())),
        "referred to at open: kept for the undo"
    );
}

/// Criterion 66 (architect review 2026-11-23): the 512 MiB limit and the budget line count only
/// the stored results a step refers to; a result kept only for undo (or not referred to yet)
/// neither counts nor blocks a new one.
#[test]
fn only_referenced_results_count_against_512_mib() {
    const MIB: usize = 1024 * 1024;
    let d = Document::new(1);
    let picture = image(&d, 10.0, "AI");
    let mut bytes = vec![0_u8; 64 * MIB];
    let mut hashes = Vec::new();
    for i in 0..8_u8 {
        bytes[0] = i;
        hashes.push(d.insert_derived_asset(&bytes).unwrap());
    }
    assert_eq!(d.derived_asset_bytes(), 0, "nothing refers to them yet");
    assert!(
        d.insert_derived_asset(&[1]).is_ok(),
        "unreferenced results do not fill the budget"
    );
    let entries: Vec<AdjustEntry> = hashes
        .iter()
        .map(|h| {
            ran(
                step("ai_denoise", &[("strength_pct", 50)]),
                h.clone(),
                "ffdnet-color",
                (1, 1),
            )
        })
        .collect();
    write(&d, picture, entries);
    assert_eq!(d.derived_asset_bytes(), 512 * MIB as u64);
    assert!(matches!(
        d.insert_derived_asset(&[2]),
        Err(AssetError::ResultsFull { .. })
    ));
    // The results removed from the steps stay for undo but no longer count.
    write(&d, picture, Vec::new());
    assert_eq!(d.derived_asset_bytes(), 0);
    assert!(
        hashes.iter().all(|h| d.has_derived_asset(h)),
        "kept for undo"
    );
    assert!(d.insert_derived_asset(&[2]).is_ok());
}
