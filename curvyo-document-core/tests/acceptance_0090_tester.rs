//! Independent acceptance tests for `specs/0090-grid` criteria 1, 3 (register side), 10, 11
//! (stored settings), 15 (one step per change) and 16 (origin corner), written from the specification before the
//! implementation was read.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::f64::consts::{PI, TAU};
use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, DocumentSize,
    DocumentSnapping, GridMode, GridScale, GridSnapPoint, Length, OpenError, OriginCorner,
    PageGrid, PageGridError, Point, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

const MM_EPS: f64 = 1e-9;

fn reopen(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "acceptance").unwrap()).unwrap()
}

fn custom_grid() -> PageGrid {
    PageGrid {
        shown: true,
        mode: GridMode::Fixed,
        scale: GridScale::Logarithmic,
        spacing: Length::from_mm(2.5),
        subdivision: 10,
        angle: Angle::from_radians(PI / 6.0),
        origin: OriginCorner::BottomRight,
    }
}

fn assert_grid_eq(a: PageGrid, b: PageGrid) {
    assert_eq!(a.shown, b.shown);
    assert_eq!(a.mode, b.mode);
    assert_eq!(a.scale, b.scale);
    assert!(
        (a.spacing.as_mm() - b.spacing.as_mm()).abs() < MM_EPS,
        "{a:?} != {b:?}"
    );
    assert_eq!(a.subdivision, b.subdivision);
    assert!(
        (a.angle.as_radians() - b.angle.as_radians()).abs() < 1e-12,
        "{a:?} != {b:?}"
    );
    assert_eq!(a.origin, b.origin, "{a:?} != {b:?}");
}

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn container_from_loro(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", loro_bytes.to_vec()),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A current file whose root map additionally carries `key = value`.
fn file_with_root(base: &Document, key: &str, value: LoroValue) -> Vec<u8> {
    let loro = loro_of(base);
    loro.set_peer_id(77).unwrap();
    loro.get_map("root").insert(key, value).unwrap();
    loro.commit();
    container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

// ---------------------------------------------------------------- criterion 1

#[test]
fn ac1_new_document_page_has_the_stated_default_grid() {
    let d = Document::new(9001);
    let page = &d.pages()[0];
    let g = page.grid;
    assert!(!g.shown);
    assert_eq!(g.mode, GridMode::Adaptive);
    assert_eq!(g.scale, GridScale::Linear);
    assert!((g.spacing.as_mm() - 5.0).abs() < MM_EPS);
    assert_eq!(g.subdivision, 5);
    assert_eq!(g.angle.as_radians(), 0.0);
    assert_eq!(g.origin, OriginCorner::TopLeft);
    assert_grid_eq(g, PageGrid::DEFAULT);
    assert_grid_eq(d.page_grid(&page.id), PageGrid::DEFAULT);
}

#[test]
fn ac1_format_version_went_up_by_one_to_30() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 30);
}

#[test]
fn ac1_every_field_travels_with_the_file_and_reads_do_not_write() {
    let d = Document::new(9002);
    let first = d.pages()[0].id.clone();
    assert!(d.set_page_grid(&first, custom_grid()).unwrap());
    let r = reopen(&d, 9003);
    let v = r.version();
    assert_grid_eq(r.pages()[0].grid, custom_grid());
    assert_grid_eq(r.page_grid(&first), custom_grid());
    assert_eq!(r.version(), v, "reading a grid must not write");
}

#[test]
fn ac1_each_page_has_its_own_grid() {
    let d = Document::new(9004);
    let first = d.pages()[0].id.clone();
    let second = d.add_page().id;
    d.set_page_grid(&second, custom_grid()).unwrap();
    assert_grid_eq(d.page_grid(&first), PageGrid::DEFAULT);
    assert_grid_eq(d.page_grid(&second), custom_grid());
    let r = reopen(&d, 9005);
    assert_grid_eq(r.page_grid(&first), PageGrid::DEFAULT);
    assert_grid_eq(r.page_grid(&second), custom_grid());
}

#[test]
fn ac1_add_page_copies_the_grid_of_the_page_it_takes_its_size_from() {
    let d = Document::new(9006);
    let first = d.pages()[0].id.clone();
    d.set_page_grid(&first, custom_grid()).unwrap();
    let second = d.add_page();
    assert_grid_eq(second.grid, custom_grid());
    assert_grid_eq(d.page_grid(&second.id), custom_grid());
    // A changed second page is the size source of the third.
    let other = PageGrid {
        spacing: Length::from_mm(10.0),
        mode: GridMode::Adaptive,
        ..custom_grid()
    };
    d.set_page_grid(&second.id, other).unwrap();
    let third = d.add_page();
    assert_grid_eq(third.grid, other);
    // The copy is a copy: changing the source later does not change the copy.
    d.set_page_grid(&second.id, PageGrid::DEFAULT).unwrap();
    assert_grid_eq(d.page_grid(&third.id), other);
}

#[test]
fn ac1_add_page_after_reorder_copies_from_the_last_page_in_list_order() {
    // 0059: the new page takes its size from the last page in list order.
    let d = Document::new(9007);
    let first = d.pages()[0].id.clone();
    d.resize_page(&first, DocumentSize::from_mm(100.0, 100.0), &[])
        .unwrap();
    let second = d.add_page();
    d.resize_page(&second.id, DocumentSize::from_mm(300.0, 50.0), &[])
        .unwrap();
    d.set_page_grid(&first, custom_grid()).unwrap();
    d.reorder_page(&first, 1).unwrap();
    let third = d.add_page();
    assert_eq!(third.size, DocumentSize::from_mm(100.0, 100.0));
    assert_grid_eq(third.grid, custom_grid());
}

#[test]
fn ac16_page_move_and_resize_keep_the_origin_corner() {
    let d = Document::new(9008);
    let first = d.pages()[0].id.clone();
    d.set_page_grid(&first, custom_grid()).unwrap();
    d.move_page(&first, Vec2::new(-321.5, 44.0), &[]).unwrap();
    assert_grid_eq(d.page_grid(&first), custom_grid());
    d.resize_page(&first, DocumentSize::from_mm(17.0, 900.0), &[])
        .unwrap();
    assert_grid_eq(d.page_grid(&first), custom_grid());
    let page = d.pages()[0].clone();
    assert_eq!(page.origin, Point::new(-321.5, 44.0));
    assert_grid_eq(page.grid, custom_grid());
    // The bottom-right zero follows the moved and resized page's corner.
    assert_eq!(page.zero_point(), Point::new(-321.5 + 17.0, 44.0 + 900.0));
}

#[test]
fn ac16_every_origin_corner_travels_with_the_file_and_a_missing_one_is_top_left() {
    let d = Document::new(9030);
    let id = d.pages()[0].id.clone();
    assert_eq!(d.page_grid(&id).origin, OriginCorner::TopLeft);
    for corner in OriginCorner::ALL {
        let grid = PageGrid {
            origin: corner,
            ..PageGrid::DEFAULT
        };
        let _ = d.set_page_grid(&id, grid).unwrap();
        assert_grid_eq(reopen(&d, 9031).page_grid(&id), grid);
    }
    let added = d.add_page();
    assert_eq!(added.grid.origin, OriginCorner::BottomRight);
}

#[test]
fn ac1_opening_a_file_without_grid_registers_uses_defaults_and_does_not_rewrite() {
    for name in ["pages_v26.curvyo", "snapping_v27.curvyo"] {
        let d = unpack(9009, &fixture(name)).unwrap();
        let v = d.version();
        let steps = d.history_len();
        for page in d.pages() {
            assert_grid_eq(page.grid, PageGrid::DEFAULT);
            assert_grid_eq(d.page_grid(&page.id), PageGrid::DEFAULT);
        }
        let s = d.snapping();
        assert!(s.grid, "{name}: grid snapping defaults on");
        assert_eq!(s.grid_point, GridSnapPoint::Centre);
        assert_eq!(d.version(), v, "{name}: reading must not write");
        assert_eq!(d.history_len(), steps);
    }
}

#[test]
fn ac1_a_new_document_writes_no_grid_register() {
    let d = Document::new(9010);
    let root = loro_of(&d).get_map("root");
    let keys: Vec<String> = root.keys().map(|k| k.to_string()).collect();
    assert!(
        keys.iter().all(|k| !k.contains("grid")),
        "unexpected grid registers {keys:?}"
    );
}

#[test]
fn ac1_malformed_grid_registers_are_refused_as_damaged() {
    let base = Document::new(9011);
    let id = base.pages()[0].id.clone();
    let key = |field: &str| format!("page.{id}.{field}");
    let cases: Vec<(String, LoroValue)> = vec![
        (key("grid_shown"), LoroValue::String("yes".into())),
        (key("grid_shown"), LoroValue::I64(1)),
        (key("grid_mode"), LoroValue::String("rigid".into())),
        (key("grid_mode"), LoroValue::Bool(true)),
        (key("grid_scale"), LoroValue::String("logarithmic".into())),
        (key("grid_scale"), LoroValue::String("".into())),
        (key("grid_spacing"), LoroValue::Double(0.0)),
        (key("grid_spacing"), LoroValue::Double(-5.0)),
        (key("grid_spacing"), LoroValue::Double(f64::NAN)),
        (key("grid_spacing"), LoroValue::Double(f64::INFINITY)),
        (key("grid_spacing"), LoroValue::String("5".into())),
        (key("grid_subdivision"), LoroValue::I64(1)),
        (key("grid_subdivision"), LoroValue::I64(0)),
        (key("grid_subdivision"), LoroValue::I64(-5)),
        (key("grid_subdivision"), LoroValue::Double(5.0)),
        (key("grid_angle"), LoroValue::Double(f64::NAN)),
        (key("grid_angle"), LoroValue::Double(f64::NEG_INFINITY)),
        (key("grid_origin"), LoroValue::String("centre".into())),
        (key("grid_origin"), LoroValue::String("".into())),
        (key("grid_origin"), LoroValue::Bool(false)),
        (key("grid_origin"), LoroValue::Double(0.0)),
        ("snapping_grid".to_owned(), LoroValue::String("on".into())),
        (
            "snapping_grid_point".to_owned(),
            LoroValue::String("middle".into()),
        ),
        ("snapping_grid_point".to_owned(), LoroValue::I64(0)),
    ];
    for (k, value) in cases {
        let bytes = file_with_root(&base, &k, value.clone());
        assert!(
            matches!(unpack(9012, &bytes), Err(OpenError::Damaged)),
            "{k} = {value:?} must be refused as damaged"
        );
    }
}

#[test]
fn ac1_valid_hand_made_registers_open() {
    let base = Document::new(9013);
    let id = base.pages()[0].id.clone();
    let key = |field: &str| format!("page.{id}.{field}");
    for (k, value) in [
        (key("grid_shown"), LoroValue::Bool(true)),
        (key("grid_mode"), LoroValue::String("fixed".into())),
        (key("grid_scale"), LoroValue::String("log".into())),
        (key("grid_spacing"), LoroValue::Double(1e-6)),
        (key("grid_subdivision"), LoroValue::I64(2)),
        (key("grid_angle"), LoroValue::Double(-1.0)),
        (key("grid_origin"), LoroValue::String("top_left".into())),
        (key("grid_origin"), LoroValue::String("bottom_left".into())),
        (
            "snapping_grid_point".to_owned(),
            LoroValue::String("box".into()),
        ),
        (
            "snapping_grid_point".to_owned(),
            LoroValue::String("nodes".into()),
        ),
    ] {
        let bytes = file_with_root(&base, &k, value.clone());
        assert!(unpack(9014, &bytes).is_ok(), "{k} = {value:?} must open");
    }
    // A finite angle outside [0, 2π) is permitted and reads normalised.
    let bytes = file_with_root(&base, &key("grid_angle"), LoroValue::Double(-PI / 2.0));
    let d = unpack(9015, &bytes).unwrap();
    let a = d.page_grid(&id).angle.as_radians();
    assert!((a - 1.5 * PI).abs() < 1e-12, "{a}");
}

#[test]
fn ac1_damaged_register_on_a_deleted_page_is_still_refused() {
    let base = Document::new(9016);
    let second = base.add_page().id;
    base.delete_page(&second).unwrap();
    let bytes = file_with_root(
        &base,
        &format!("page.{second}.grid_spacing"),
        LoroValue::Double(-1.0),
    );
    assert!(matches!(unpack(9017, &bytes), Err(OpenError::Damaged)));
}

#[test]
fn ac1_concurrent_edits_of_different_grid_fields_merge_fieldwise() {
    let a = Document::new(9018);
    let id = a.pages()[0].id.clone();
    let b = reopen(&a, 9019);
    a.set_page_grid(
        &id,
        PageGrid {
            spacing: Length::from_mm(7.0),
            ..PageGrid::DEFAULT
        },
    )
    .unwrap();
    b.set_page_grid(
        &id,
        PageGrid {
            shown: true,
            ..PageGrid::DEFAULT
        },
    )
    .unwrap();
    let (av, bv) = (a.version(), b.version());
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    let expected = PageGrid {
        shown: true,
        spacing: Length::from_mm(7.0),
        ..PageGrid::DEFAULT
    };
    assert_grid_eq(a.page_grid(&id), expected);
    assert_grid_eq(b.page_grid(&id), expected);
}

// ------------------------------------------------- criterion 3 (register side)

#[test]
fn ac3_non_finite_zero_negative_spacing_and_low_subdivision_are_refused_without_writing() {
    let d = Document::new(9020);
    let id = d.pages()[0].id.clone();
    d.set_page_grid(&id, custom_grid()).unwrap();
    let v = d.version();
    let h = d.history_len();
    let bad: Vec<(PageGrid, PageGridError)> = vec![
        (
            PageGrid {
                spacing: Length::from_mm(0.0),
                ..custom_grid()
            },
            PageGridError::Spacing,
        ),
        (
            PageGrid {
                spacing: Length::from_mm(-0.0),
                ..custom_grid()
            },
            PageGridError::Spacing,
        ),
        (
            PageGrid {
                spacing: Length::from_mm(-3.0),
                ..custom_grid()
            },
            PageGridError::Spacing,
        ),
        (
            PageGrid {
                spacing: Length::from_mm(f64::NAN),
                ..custom_grid()
            },
            PageGridError::NotFinite,
        ),
        (
            PageGrid {
                spacing: Length::from_mm(f64::INFINITY),
                ..custom_grid()
            },
            PageGridError::NotFinite,
        ),
        (
            PageGrid {
                subdivision: 1,
                ..custom_grid()
            },
            PageGridError::Subdivision,
        ),
        (
            PageGrid {
                subdivision: 0,
                ..custom_grid()
            },
            PageGridError::Subdivision,
        ),
        (
            PageGrid {
                angle: Angle::from_radians(f64::NAN),
                ..custom_grid()
            },
            PageGridError::NotFinite,
        ),
        (
            PageGrid {
                angle: Angle::from_radians(f64::INFINITY),
                ..custom_grid()
            },
            PageGridError::NotFinite,
        ),
    ];
    for (grid, error) in bad {
        assert_eq!(d.set_page_grid(&id, grid), Err(error), "{grid:?}");
        assert_eq!(d.version(), v, "a refusal writes nothing: {grid:?}");
        assert_eq!(d.history_len(), h);
        assert_grid_eq(d.page_grid(&id), custom_grid());
    }
    assert_eq!(
        d.set_page_grid("no-such-page", custom_grid()),
        Err(PageGridError::NotFound)
    );
    assert_eq!(d.version(), v);
}

#[test]
fn ac3_extreme_but_valid_values_are_permitted() {
    let d = Document::new(9021);
    let id = d.pages()[0].id.clone();
    for grid in [
        PageGrid {
            spacing: Length::from_mm(1e-9),
            ..PageGrid::DEFAULT
        },
        PageGrid {
            spacing: Length::from_mm(1e9),
            ..PageGrid::DEFAULT
        },
        PageGrid {
            subdivision: 2,
            ..PageGrid::DEFAULT
        },
        PageGrid {
            subdivision: u32::MAX,
            ..PageGrid::DEFAULT
        },
        PageGrid {
            origin: OriginCorner::TopRight,
            ..PageGrid::DEFAULT
        },
    ] {
        d.set_page_grid(&id, grid).unwrap();
        assert_grid_eq(reopen(&d, 9022).page_grid(&id), grid);
    }
}

#[test]
fn ac3_the_angle_is_taken_modulo_360_degrees() {
    let d = Document::new(9023);
    let id = d.pages()[0].id.clone();
    for (degrees, expected) in [
        (370.0_f64, 10.0_f64),
        (-90.0, 270.0),
        (360.0, 0.0),
        (720.0, 0.0),
        (-360.0, 0.0),
        (-0.0, 0.0),
        (359.5, 359.5),
        (3600.0 + 45.0, 45.0),
        (-1e6, (-1e6_f64).rem_euclid(360.0)),
    ] {
        d.set_page_grid(
            &id,
            PageGrid {
                angle: Angle::from_radians(degrees.to_radians()),
                ..PageGrid::DEFAULT
            },
        )
        .unwrap();
        let stored = d.page_grid(&id).angle.as_radians();
        assert!(
            (0.0..TAU).contains(&stored),
            "{degrees}° stored as {stored} rad"
        );
        assert!(
            (stored.to_degrees() - expected).abs() < 1e-6,
            "{degrees}° -> {}° expected {expected}°",
            stored.to_degrees()
        );
        assert!(!stored.is_sign_negative(), "-0 must store as 0");
        let back = reopen(&d, 9024).page_grid(&id).angle.as_radians();
        assert!((back - stored).abs() < 1e-12);
    }
}

#[test]
fn ac3_ac15_each_change_is_exactly_one_history_step_and_an_equal_write_is_none() {
    let d = Document::new(9025);
    let id = d.pages()[0].id.clone();
    let steps = d.own_steps().len();
    assert!(d.set_page_grid(&id, custom_grid()).unwrap());
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "set_page_grid");
    let v = d.version();
    assert!(!d.set_page_grid(&id, custom_grid()).unwrap());
    assert_eq!(d.version(), v, "an equal grid writes nothing");
    assert_eq!(d.own_steps().len(), steps + 1);
    // An angle + 360° is the same grid: no step, and the stored angle stays 30°.
    let same = PageGrid {
        angle: Angle::from_radians(PI / 6.0 + TAU),
        ..custom_grid()
    };
    assert!(!d.set_page_grid(&id, same).unwrap());
    assert_eq!(d.version(), v, "a full turn more writes nothing");
    assert_eq!(d.own_steps().len(), steps + 1);
    assert!((d.page_grid(&id).angle.as_radians() - PI / 6.0).abs() < 1e-12);
}

// --------------------------------------------- criteria 10 and 11 (storage)

#[test]
fn ac10_ac11_grid_target_defaults_on_with_centre_and_both_travel_with_the_file() {
    let default = DocumentSnapping::DEFAULT;
    assert!(default.grid, "grid target defaults on");
    assert_eq!(DocumentSnapping::DEFAULT.grid_point, GridSnapPoint::Centre);
    let d = Document::new(9026);
    assert!(d.snapping().grid);
    assert_eq!(d.snapping().grid_point, GridSnapPoint::Centre);
    for point in [
        GridSnapPoint::Box,
        GridSnapPoint::Nodes,
        GridSnapPoint::Centre,
    ] {
        for grid in [false, true] {
            let value = DocumentSnapping {
                grid,
                grid_point: point,
                ..DocumentSnapping::DEFAULT
            };
            d.set_snapping(value);
            let r = reopen(&d, 9027);
            assert_eq!(r.snapping().grid, grid);
            assert_eq!(r.snapping().grid_point, point);
        }
    }
}

#[test]
fn ac11_grid_point_is_document_wide_not_per_page() {
    let d = Document::new(9028);
    let _ = d.add_page();
    d.set_snapping(DocumentSnapping {
        grid_point: GridSnapPoint::Nodes,
        ..DocumentSnapping::DEFAULT
    });
    // Grid point and grid target do not touch any page grid.
    for page in d.pages() {
        assert_grid_eq(page.grid, PageGrid::DEFAULT);
    }
}
