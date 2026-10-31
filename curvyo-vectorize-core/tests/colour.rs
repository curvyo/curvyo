//! Multicolour tracing (`specs/0089-trace-quality-and-colour` slice M, criteria 26 to 30): the
//! palette, the label map, the shared boundaries and the layers, measured on the label-map
//! fixtures of `common/quality/multi.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::many_single_char_names
)]

mod common;

use std::ops::ControlFlow;

use common::quality::multi::{
    BLUE, GREEN, Inside, MultiScene, RED, WHITE, YELLOW, common_nodes, measure, multi_set,
};
use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{
    ColourParams, ColourTrace, Rgb8, Selection, TraceParams, colour_mask, suggest_background,
    trace_colours,
};

/// Millimetres per pixel of the fixtures: Ignore below 0.5 mm is 5 px.
const MM_PER_PX: f64 = 0.1;

fn rgb(c: [u8; 4]) -> Rgb8 {
    Rgb8::new(c[0], c[1], c[2])
}

/// Remove background with the picture's suggested background (none for a transparent one).
fn params_for(picture: &common::Picture) -> TraceParams {
    TraceParams {
        selection: Selection::RemoveBackground {
            background: suggest_background(picture.raster()),
            tolerance: curvyo_vectorize_core::DEFAULT_BACKGROUND_TOLERANCE,
        },
        ..TraceParams::with_threshold(128)
    }
}

fn run(picture: &common::Picture, params: &ColourParams) -> ColourTrace {
    let mut go = |_| ControlFlow::Continue(());
    trace_colours(
        picture.raster(),
        Affine::scale(MM_PER_PX, MM_PER_PX),
        params,
        &mut go,
    )
    .unwrap()
}

fn scene(name: &str) -> MultiScene {
    multi_set().into_iter().find(|s| s.name == name).unwrap()
}

fn traced(s: &MultiScene) -> (common::quality::multi::MultiDrawn, ColourTrace) {
    let drawn = s.draw();
    let mut p = ColourParams::new(params_for(&drawn.picture));
    p.colours = s.count;
    let result = run(&drawn.picture, &p);
    (drawn, result)
}

/// Criterion 26: the automatic palette finds each fixture's colours (a fixed count for the
/// gradient); the 3-colour logo with anti-aliasing and JPEG quality 75 has exactly 3 colours
/// plus background; the pale accent of 0.2 % is kept; near-greys 0.043 apart stay two colours,
/// colours 0.02 apart merge.
#[test]
fn criterion_26_the_palette_has_the_colours_and_no_seams() {
    for s in multi_set() {
        let (_, result) = traced(&s);
        let found: Vec<String> = result
            .palette
            .iter()
            .map(|e| format!("#{:02X}{:02X}{:02X}", e.colour.r, e.colour.g, e.colour.b))
            .collect();
        assert_eq!(result.palette.len(), s.colours, "{}: {found:?}", s.name);
    }
    let (_, pale) = traced(&scene("pale-accent"));
    assert!(
        pale.palette
            .iter()
            .any(|e| e.colour == Rgb8::new(0xFF, 0xF2, 0xA8)),
        "{:?}",
        pale.palette
    );
}

/// Criterion 26: fixed colours are kept exactly and come first; a count asks for 2 to 16.
#[test]
fn criterion_26_fixed_colours_are_kept_and_the_count_is_clamped() {
    let drawn = scene("adjacent-straight").draw();
    let mut p = ColourParams::new(params_for(&drawn.picture));
    let off_red = Rgb8::new(0xD3, 0x20, 0x2C);
    p.fixed = vec![off_red];
    let result = run(&drawn.picture, &p);
    assert_eq!(result.palette[0].colour, off_red);
    assert!(result.palette[0].fixed);
    assert_eq!(result.palette.len(), 3, "{:?}", result.palette);
    p.fixed.clear();
    p.colours = Some(2);
    assert_eq!(run(&drawn.picture, &p).palette.len(), 2);
    p.colours = Some(1);
    assert_eq!(run(&drawn.picture, &p).palette.len(), 2);
    p.colours = Some(40);
    assert!(run(&drawn.picture, &p).palette.len() <= 16);
}

/// Criterion 27: every pixel belongs to one colour or the background (no sample is covered
/// twice); regions below Ignore below join the neighbour with the longest border and are
/// counted.
#[test]
fn criterion_27_small_regions_join_and_are_counted() {
    let mut s = scene("adjacent-straight");
    let dots: Vec<(f64, f64)> = vec![(100.0, 100.0), (150.0, 250.0), (220.0, 300.0)];
    for &(x, y) in &dots {
        let inside: Inside =
            std::sync::Arc::new(move |px, py| (px - x).abs() < 1.5 && (py - y).abs() < 1.5);
        s.layers.push((inside, GREEN));
    }
    let drawn = s.draw();
    let mut p = ColourParams::new(params_for(&drawn.picture));
    p.fixed = vec![rgb(GREEN)];
    let result = run(&drawn.picture, &p);
    assert!(
        result.diagnostics.joined_regions >= dots.len(),
        "{:?}",
        result.diagnostics
    );
    assert!(result.layers.iter().all(|l| l.sample != rgb(GREEN)));
    let m = measure(&s, &drawn, &result, MM_PER_PX);
    assert_eq!(m.overlap, 0.0, "{m:?}");
}

/// Criterion 28: neighbours share their boundary segment for segment (reversed), a triple
/// junction is one node of all three colours, and at 4x no sample is uncovered or doubly
/// covered beyond 0.01 % of the picture, at the Logo settings and at Silhouette's.
#[test]
fn criterion_28_neighbours_share_one_boundary() {
    for name in [
        "adjacent-straight",
        "adjacent-curved",
        "enclosed",
        "junctions-8",
        "logo3-aa-jpeg75",
        "transparent",
    ] {
        let s = scene(name);
        let drawn = s.draw();
        for (smooth, tolerance) in [(80, 0.5), (100, 1.5)] {
            let mut t = params_for(&drawn.picture);
            t.smooth = smooth;
            t.tolerance = curvyo_vectorize_core::PixelLength::from_px(tolerance);
            let result = run(&drawn.picture, &ColourParams::new(t));
            let m = measure(&s, &drawn, &result, MM_PER_PX);
            assert_eq!(m.bad_shared, 0, "{name} {smooth}: {m:?}");
            assert!(m.shared > 0, "{name}: {m:?}");
            assert!(m.overlap <= 1e-4, "{name} {smooth}: {m:?}");
            assert!(m.gap <= 1e-4, "{name} {smooth}: {m:?}");
            assert!(m.assigned >= 0.99, "{name} {smooth}: {m:?}");
        }
    }
    let (_, result) = traced(&scene("adjacent-straight"));
    let index = |c: [u8; 4]| {
        result
            .layers
            .iter()
            .position(|l| (l.sample.r, l.sample.g) == (c[0], c[1]))
            .unwrap()
    };
    let common = common_nodes(&result, &[index(RED), index(BLUE), index(YELLOW)]);
    let junction = (300.4 * MM_PER_PX, 200.5 * MM_PER_PX);
    assert!(
        common
            .iter()
            .any(|p| (p.x - junction.0).hypot(p.y - junction.1) < 1.5 * MM_PER_PX),
        "{common:?}"
    );
}

/// Criterion 29: one layer per colour, largest area first (ties by colour value); Exclude makes
/// a colour background, Merge into traces it as the target's; the same input gives the same
/// result.
#[test]
fn criterion_29_layers_exclude_and_merge() {
    let s = scene("adjacent-straight");
    let drawn = s.draw();
    let p = ColourParams::new(params_for(&drawn.picture));
    let result = run(&drawn.picture, &p);
    assert_eq!(result.layers.len(), 3);
    assert!(result.layers.windows(2).all(|w| w[0].pixels >= w[1].pixels));
    assert_eq!(result.layers[0].sample.r, RED[0]);
    assert_eq!(result, run(&drawn.picture, &p));
    let blue = result
        .palette
        .iter()
        .find(|e| e.colour.b > 150)
        .unwrap()
        .colour;
    let yellow = result
        .palette
        .iter()
        .find(|e| e.colour.g > 150)
        .unwrap()
        .colour;
    let mut excluded = p.clone();
    excluded.excluded = vec![blue];
    let without = run(&drawn.picture, &excluded);
    assert_eq!(without.layers.len(), 2);
    assert!(without.layers.iter().all(|l| l.sample != blue));
    // The palette still lists it (the strip shows it excluded).
    assert_eq!(without.palette.len(), 3);
    let mut merged = p.clone();
    merged.merged = vec![(yellow, blue)];
    let together = run(&drawn.picture, &merged);
    assert_eq!(together.layers.len(), 2);
    let blue_layer = together.layers.iter().find(|l| l.sample == blue).unwrap();
    let before: u64 = result
        .layers
        .iter()
        .filter(|l| l.sample == blue || l.sample == yellow)
        .map(|l| l.pixels)
        .sum();
    assert_eq!(blue_layer.pixels, before);
}

/// The Mask view of a multicolour trace is the segmented palette: content pixels in their
/// colour, the background transparent; an excluded colour is background there too.
#[test]
fn the_colour_mask_is_the_segmented_palette() {
    let drawn = scene("adjacent-straight").draw();
    let mut p = ColourParams::new(params_for(&drawn.picture));
    let mut go = |_| ControlFlow::Continue(());
    let (mask, palette) = colour_mask(
        drawn.picture.raster(),
        Affine::scale(MM_PER_PX, MM_PER_PX),
        &p,
        &mut go,
    )
    .unwrap();
    assert_eq!((mask.width, mask.height), (600, 400));
    let at = |x: usize, y: usize| &mask.rgba[(y * 600 + x) * 4..(y * 600 + x) * 4 + 4];
    assert_eq!(at(5, 5)[3], 0);
    let red = at(100, 100);
    assert_eq!(red[3], 255);
    assert!(
        palette
            .iter()
            .any(|e| [e.colour.r, e.colour.g, e.colour.b] == red[..3])
    );
    p.excluded = vec![Rgb8::new(red[0], red[1], red[2])];
    let (mask, _) = colour_mask(
        drawn.picture.raster(),
        Affine::scale(MM_PER_PX, MM_PER_PX),
        &p,
        &mut go,
    )
    .unwrap();
    assert_eq!(mask.rgba[(100 * 600 + 100) * 4 + 3], 0);
}

/// Pick colours with Multicolour: the palette is exactly the picked colours (Frage 90 A).
#[test]
fn pick_colours_palette_is_the_picks() {
    let drawn = scene("adjacent-straight").draw();
    let picks = [rgb(RED), rgb(YELLOW)];
    let t = TraceParams {
        selection: Selection::PickColours {
            swatches: curvyo_vectorize_core::Swatches::from_slice(&picks),
            background: Some(rgb(WHITE)),
            tolerance: curvyo_vectorize_core::DEFAULT_PICK_TOLERANCE,
        },
        ..TraceParams::with_threshold(128)
    };
    let mut p = ColourParams::new(t);
    p.colours = Some(9);
    let result = run(&drawn.picture, &p);
    let colours: Vec<Rgb8> = result.palette.iter().map(|e| e.colour).collect();
    assert_eq!(colours, picks);
    assert_eq!(result.layers.len(), 2);
}

/// The 2000 x 2000 px picture of eight colours: discs, rings and stripes.
fn eight_colours_2000() -> common::Picture {
    let colours: [[u8; 4]; 8] = [
        RED,
        BLUE,
        YELLOW,
        GREEN,
        [0x7A, 0x3C, 0x9E, 255],
        [0xF0, 0x7C, 0x22, 255],
        [0x1B, 0xA8, 0xB0, 255],
        [0x55, 0x55, 0x55, 255],
    ];
    let mut noise = common::quality::Noise::new(7);
    let shapes: Vec<(f64, f64, f64, usize)> = (0..240)
        .map(|k| {
            (
                1000.0 + 950.0 * noise.next(),
                1000.0 + 950.0 * noise.next(),
                15.0 + 60.0 * noise.next().abs(),
                k % 8,
            )
        })
        .collect();
    let mut rgba = Vec::with_capacity(2000 * 2000 * 4);
    for y in 0..2000 {
        for x in 0..2000 {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let band = ((px + py) / 250.0) as usize % 8;
            let mut c = if ((py / 125.0) as usize).is_multiple_of(3) {
                colours[band]
            } else {
                WHITE
            };
            for &(cx, cy, r, k) in &shapes {
                if (px - cx).powi(2) + (py - cy).powi(2) <= r * r {
                    c = colours[k];
                }
            }
            rgba.extend_from_slice(&c);
        }
    }
    common::Picture {
        width: 2000,
        height: 2000,
        rgba,
    }
}

/// Criterion 30: 8 colours on a 2000 x 2000 px picture trace in under 4 s of thread CPU time
/// (native; the wasm time is reported by the frontend's timing script).
#[cfg(unix)]
#[test]
#[ignore = "budget: run in release"]
fn criterion_30_eight_colours_2000_budget() {
    let picture = eight_colours_2000();
    let mut p = ColourParams::new(params_for(&picture));
    p.trace.specks = Length::from_mm(0.0);
    let start = common::thread_cpu_seconds();
    let mut go = |_| ControlFlow::Continue(());
    let result = trace_colours(
        picture.raster(),
        Affine::scale(25.4 / 300.0, 25.4 / 300.0),
        &p,
        &mut go,
    )
    .unwrap();
    let total = common::thread_cpu_seconds() - start;
    println!(
        "8 colours, 2000 x 2000 px: {total:.3} s thread CPU, {} colours, {} paths, {} nodes, {} fallbacks",
        result.palette.len(),
        result.counts.paths,
        result.counts.nodes,
        result.diagnostics.fallbacks
    );
    assert_eq!(result.palette.len(), 8);
    assert!(total < 4.0, "{total} s");
}

/// The slice M table (release): per fixture the colours, colour assignment, gaps and overlaps
/// at 4x, shared segments, paths, nodes and thread CPU.
#[cfg(unix)]
#[test]
#[ignore = "table: run in release"]
fn multicolour_table() {
    println!(
        "| Fixture | Colours (want) | Layers | Assigned | Gap | Overlap | Shared / bad | Paths | Nodes | Fallbacks | Joined | CPU s |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    for s in multi_set() {
        let drawn = s.draw();
        let mut p = ColourParams::new(params_for(&drawn.picture));
        p.colours = s.count;
        let t0 = common::thread_cpu_seconds();
        let result = run(&drawn.picture, &p);
        let cpu = common::thread_cpu_seconds() - t0;
        let m = measure(&s, &drawn, &result, MM_PER_PX);
        println!(
            "| {} | {} ({}) | {} | {:.4} | {:.5} % | {:.5} % | {} / {} | {} | {} | {} | {} | {cpu:.3} |",
            s.name,
            m.colours,
            s.colours,
            m.layers,
            m.assigned,
            m.gap * 100.0,
            m.overlap * 100.0,
            m.shared,
            m.bad_shared,
            m.paths,
            m.nodes,
            result.diagnostics.fallbacks,
            result.diagnostics.joined_regions
        );
    }
}

/// The real images of the reference set in Multicolour (Remove background, automatic palette):
/// colours, paths, nodes and thread CPU per image (release).
#[cfg(unix)]
#[test]
#[ignore = "table: run in release"]
fn multicolour_real_table() {
    println!("| Image | Size | Colours | Paths | Nodes | Fallbacks | Joined | CPU s |");
    println!("|---|---|---|---|---|---|---|---|");
    for image in common::quality::real::real_images()
        .into_iter()
        .chain(common::quality::real::customer_images())
    {
        let picture = &image.picture;
        let p = ColourParams::new(params_for(picture));
        let t0 = common::thread_cpu_seconds();
        let mut go = |_| ControlFlow::Continue(());
        let result = trace_colours(picture.raster(), common::at_96_dpi(), &p, &mut go).unwrap();
        let cpu = common::thread_cpu_seconds() - t0;
        println!(
            "| {} | {} x {} | {} | {} | {} | {} | {} | {cpu:.3} |",
            image.id,
            picture.width,
            picture.height,
            result.palette.len(),
            result.counts.paths,
            result.counts.nodes,
            result.diagnostics.fallbacks,
            result.diagnostics.joined_regions
        );
    }
}

/// Criterion 30, the wasm side: writes the 8-colour 2000 x 2000 px picture and the slowest real
/// photo in Multicolour (native thread CPU, every real and local image tried) with their
/// settings to `target/trace-colour-wasm/`, for
/// `CURVYO_TRACE_WASM_DIR=target/trace-colour-wasm node --experimental-strip-types
/// frontend/scripts/trace-reference-wasm.ts`.
#[cfg(unix)]
#[test]
#[ignore = "writer: run in release"]
fn criterion_30_wasm_manifest() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("trace-colour-wasm");
    std::fs::create_dir_all(&dir).unwrap();
    let settings = |picture: &common::Picture, specks_mm: f64| {
        let bg = suggest_background(picture.raster())
            .map(|c| format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b));
        serde_json::json!({
            "method": 1,
            "background": bg,
            "colour_tolerance": curvyo_vectorize_core::DEFAULT_BACKGROUND_TOLERANCE,
            "threshold": 128,
            "specks": specks_mm,
            "output": 1,
            "colours": null,
        })
    };
    let mut slowest: Option<(f64, String, common::Picture)> = None;
    for image in common::quality::real::real_images()
        .into_iter()
        .chain(common::quality::real::customer_images())
    {
        let p = ColourParams::new(params_for(&image.picture));
        let t0 = common::thread_cpu_seconds();
        let mut go = |_| ControlFlow::Continue(());
        trace_colours(image.picture.raster(), common::at_96_dpi(), &p, &mut go).unwrap();
        let cpu = common::thread_cpu_seconds() - t0;
        if slowest.as_ref().is_none_or(|s| cpu > s.0) {
            slowest = Some((cpu, image.id, image.picture));
        }
    }
    let (cpu, id, photo) = slowest.unwrap();
    let eight = eight_colours_2000();
    let default_specks = TraceParams::with_threshold(128).specks.as_mm();
    let mm = 25.4 / 96.0;
    let map_96 = [mm, 0.0, 0.0, mm, 0.0, 0.0];
    let map_300 = [25.4 / 300.0, 0.0, 0.0, 25.4 / 300.0, 0.0, 0.0];
    let mut manifest = Vec::new();
    for (key, picture, map, specks) in [
        ("eight-colours-2000", &eight, map_300, 0.0),
        (id.as_str(), &photo, map_96, default_specks),
    ] {
        let file = format!("{key}.pam");
        let mut bytes = format!(
            "P7\nWIDTH {}\nHEIGHT {}\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n",
            picture.width, picture.height
        )
        .into_bytes();
        bytes.extend_from_slice(&picture.rgba);
        std::fs::write(dir.join(&file), bytes).unwrap();
        manifest.push(serde_json::json!({
            "key": key,
            "file": file,
            "map": map,
            "settings": settings(picture, specks),
        }));
    }
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    println!(
        "slowest photo {id}: {cpu:.3} s native; written to {}",
        dir.display()
    );
}
