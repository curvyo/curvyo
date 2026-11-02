//! The customer's screenshot of the font manager through Trace with text
//! (`specs/0091-trace-text-recognition` criteria 26 and 27): the per-line table of the PR, the
//! profile of the fit, and the fit in shares as the worker pool runs it (round 2).
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --lib trace_text_screenshot -- --ignored --nocapture
//! ```

use curvyo_ui_core::{OcrModel, TraceParams, trace_read_text, trace_text_job_with};

use super::trace_text_tests::{fit, fonts, model, session_with};

/// One read line after the fit: the OCR's string and mean confidence, whether it passes the
/// garbage filter, and the text it became (face file, sure letters, letters, nearest), if any.
type LineReport = (String, f64, bool, Option<(String, usize, usize, bool)>);

/// The whole flow on fixture `file` (OCR, text pass, fit) with every line's outcome.
#[allow(clippy::cast_precision_loss)] // a count of characters
fn fitted_lines(model: &OcrModel, file: &str) -> Vec<LineReport> {
    let (mut session, image) = session_with(file);
    let source = session.trace_source(image, false).unwrap();
    let lines = trace_read_text(
        model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let threshold = curvyo_ui_core::trace_auto_threshold(&source.rgba, source.width, source.height);
    let wire = trace_text_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &TraceParams::with_threshold(threshold),
        &lines,
        &mut |_| {},
    );
    assert!(session.set_trace_result(image, &wire));
    let Some(request) = session.trace_fit_request(0.3, &[]) else {
        return Vec::new();
    };
    let reply = fit(&request);
    let request: serde_json::Value = serde_json::from_str(&request).unwrap();
    let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
    let faces = request["faces"].as_array().unwrap();
    let mean = |text: &str| {
        lines.iter().find(|l| l.text == text).map_or(0.0, |l| {
            l.confidence.iter().sum::<f64>() / l.confidence.len().max(1) as f64
        })
    };
    request["lines"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reply["lines"].as_array().unwrap())
        .map(|(line, out)| {
            let text = line["text"].as_str().unwrap().to_owned();
            let set = out["texts"].as_array().unwrap().first().map(|t| {
                let face = &faces[usize::try_from(t["face"].as_u64().unwrap()).unwrap()];
                (
                    face[0].as_str().unwrap().to_owned(),
                    usize::try_from(t["sure"].as_u64().unwrap()).unwrap(),
                    usize::try_from(t["letters"].as_u64().unwrap()).unwrap(),
                    t["nearest"].as_bool().unwrap(),
                )
            });
            (
                text.clone(),
                mean(&text),
                line["plausible"].as_bool().unwrap(),
                set,
            )
        })
        .collect()
}

fn print_lines(file: &str, lines: &[LineReport]) {
    println!("{file}:");
    for (text, mean, plausible, set) in lines {
        println!("  {text:?} mean {mean:.2} plausible {plausible} -> {set:?}");
    }
}

/// Criteria 26 and 27 on the customer's screenshot of the font manager and on small Source Sans 3
/// labels as Chromium draws them (`small_labels.html`): every read line that passes the garbage
/// filter becomes text, the EMS Allure and EMS Readability lines in their single-line faces.
#[test]
#[ignore = "harness: prints the per-line table of the PR"]
fn the_screenshot_lines_become_text() {
    let model = model();
    let lines = fitted_lines(&model, "customer/font-manager-screenshot.png");
    print_lines("screenshot", &lines);
    // Frage 115 B / 117 B: every line that passes the garbage filter is text.
    for (text, _, plausible, set) in &lines {
        assert!(!*plausible || set.is_some(), "{text:?} {set:?}");
    }
    let set_in = |prefix: &str| {
        lines
            .iter()
            .find(|l| l.0.starts_with(prefix))
            .and_then(|l| l.3.clone())
            .unwrap()
    };
    // Criteria 26 and 27: the EMS Allure line (read "Embeddkd ...") and the EMS Readability
    // line (cut off at "avail") in their single-line faces, with sure letters.
    let allure = set_in("Embeddkd");
    assert!(
        allure.0.starts_with("ems-allure/") && allure.1 > 0,
        "{allure:?}"
    );
    let readability = lines
        .iter()
        .find(|l| l.0 == "Embedded Freelancer avail")
        .and_then(|l| l.3.clone())
        .unwrap();
    assert!(
        readability.0.starts_with("ems-readability/") && readability.1 > 0,
        "{readability:?}"
    );
    let small = fitted_lines(&model, "small_labels.png");
    print_lines("small labels", &small);
    assert!(small.iter().all(|l| l.3.is_some()));
}

/// Where the time of the text pass and the font fit goes on the screenshot (thread CPU).
#[test]
#[ignore = "harness: prints the profile of the fit"]
fn the_fit_profile() {
    use super::trace_text_tests::cpu_seconds;
    let model = model();
    for file in ["customer/font-manager-screenshot.png", "tester/poster.png"] {
        let (mut session, image) = session_with(file);
        let source = session.trace_source(image, false).unwrap();
        let lines = trace_read_text(
            &model,
            &source.rgba,
            source.width,
            source.height,
            &mut |_| std::ops::ControlFlow::Continue(()),
        )
        .unwrap();
        assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
        let params = TraceParams::with_threshold(curvyo_ui_core::trace_auto_threshold(
            &source.rgba,
            source.width,
            source.height,
        ));
        let t = cpu_seconds();
        let regions = curvyo_ui_core::trace_text_regions_with(
            &source.rgba,
            source.width,
            source.height,
            source.map,
            &params,
            &lines,
        );
        let pass = cpu_seconds() - t;
        let t = cpu_seconds();
        let request = session
            .trace_fit_request_for(image, &regions, 0.3, &[])
            .unwrap();
        let req = cpu_seconds() - t;
        let t = cpu_seconds();
        let reply = fit(&request);
        let fitted = cpu_seconds() - t;
        let _ = trace_text_job_with;
        println!(
            "{file}: {} lines; text pass {pass:.2} s, request {req:.2} s, fit {fitted:.2} s ({} bytes)",
            lines.len(),
            reply.len()
        );
    }
}

/// The fit of `request` as `shares` pool workers run it (round 2): the lines dealt round-robin,
/// each share searched apart, every share finished with all shares' nearest faces, the lines
/// put back in their order.
fn fit_in_shares(request: &str, shares: usize) -> String {
    let mut value: serde_json::Value = serde_json::from_str(request).unwrap();
    let lines = value["lines"].as_array().unwrap().clone();
    let deal: Vec<Vec<usize>> = (0..shares)
        .map(|k| (k..lines.len()).step_by(shares).collect())
        .filter(|d: &Vec<usize>| !d.is_empty())
        .collect();
    let mut workers = Vec::new();
    let mut nearest = Vec::new();
    for share in &deal {
        value["lines"] =
            serde_json::Value::Array(share.iter().map(|&i| lines[i].clone()).collect());
        value["share"] = serde_json::json!(share);
        let mut worker = crate::trace_fit_glue::FitWorker::default();
        let mut reply = worker.start(&value.to_string(), &mut |_| {});
        loop {
            let v: serde_json::Value = serde_json::from_str(&reply).unwrap();
            if v["kind"] == "searched" {
                nearest.push(v["nearest"].clone());
                break;
            }
            for face in v["faces"].as_array().unwrap() {
                let file = face[0].as_str().unwrap();
                let bytes = std::fs::read(fonts().join(file)).unwrap();
                assert!(worker.add_face(
                    file,
                    u32::try_from(face[1].as_u64().unwrap()).unwrap(),
                    &bytes
                ));
            }
            reply = worker.step(&mut |_| {});
        }
        workers.push(worker);
    }
    let all = serde_json::Value::Array(nearest.into_iter().filter(|n| !n.is_null()).collect())
        .to_string();
    let mut merged = vec![serde_json::Value::Null; lines.len()];
    for (share, worker) in deal.iter().zip(&mut workers) {
        let done: serde_json::Value = serde_json::from_str(&worker.finish(&all)).unwrap();
        for (&i, line) in share.iter().zip(done["lines"].as_array().unwrap()) {
            merged[i] = line.clone();
        }
    }
    serde_json::json!({ "kind": "finished", "lines": merged }).to_string()
}

/// The request of the fit of `file` (OCR, text pass alone).
fn fit_request(model: &OcrModel, file: &str) -> String {
    let (mut session, image) = session_with(file);
    let source = session.trace_source(image, false).unwrap();
    let lines = trace_read_text(
        model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let regions = curvyo_ui_core::trace_text_regions_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &TraceParams::with_threshold(128),
        &lines,
    );
    session
        .trace_fit_request_for(image, &regions, 0.3, &[])
        .unwrap()
}

/// Round 2: the fit of the tester poster in three shares is the fit in one worker, byte for
/// byte (the same faces, sure and guessed letters, glyphs).
#[test]
fn the_fit_in_shares_is_the_fit_in_one_worker() {
    let request = fit_request(&model(), "tester/poster.png");
    assert_eq!(fit_in_shares(&request, 3), fit(&request));
}

/// Round 2 on the screenshot (78 lines): four shares and one worker fit the same.
#[test]
#[ignore = "release harness: the screenshot's fit takes seconds"]
fn the_screenshot_fit_in_shares_is_the_fit_in_one_worker() {
    let request = fit_request(&model(), "customer/font-manager-screenshot.png");
    assert_eq!(fit_in_shares(&request, 4), fit(&request));
}
