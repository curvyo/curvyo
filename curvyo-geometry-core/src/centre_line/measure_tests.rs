//! The measurement of ADR 0063's amendment of 2026-11-18 (B): per region and per path of a traced
//! drawing, role, length, width quartiles, nodes, width points, the node budget of criterion 20,
//! and the axis's junctions and spurs. Run by hand, before and after each fix:
//!
//! ```text
//! cargo test -p curvyo-geometry-core --lib measure_traced -- --ignored --nocapture
//! ```
//!
//! `CURVYO_CENTRE_INPUT` measures another traced fixture file (same format),
//! `CURVYO_CENTRE_SVG` writes the result as an SVG picture there (paths over their budget in
//! orange), and `CURVYO_CENTRE_PATH` lists the anchors of one path.

use std::fmt::Write as _;

use super::chains::ChainEnd;
use super::fit::{Budget, tolerance};
use super::{CentreLines, CentreOptions, CentreRole, CentreSkeleton, RegionSkeleton};
use crate::{Outline, OutlineTriple, PathMeasure};

/// The subjects of a traced fixture text: per subject its outlines.
pub(super) fn traced_subjects(text: &str) -> Vec<Vec<(Vec<OutlineTriple>, bool)>> {
    let mut subjects: Vec<Vec<(Vec<OutlineTriple>, bool)>> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") => subjects.push(Vec::new()),
            Some(&"outline") => subjects
                .last_mut()
                .unwrap()
                .push((Vec::new(), words[1] == "true")),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                subjects.last_mut().unwrap().last_mut().unwrap().0.push((
                    curvyo_document_core::Point::new(v[0], v[1]),
                    curvyo_document_core::Vec2::new(v[2], v[3]),
                    curvyo_document_core::Vec2::new(v[4], v[5]),
                ));
            }
            None => {}
        }
    }
    subjects
}

/// The centre lines of every subject of a traced fixture text.
pub(super) fn traced_lines(text: &str, options: &CentreOptions) -> Vec<CentreLines> {
    traced_subjects(text)
        .iter()
        .map(|subject| {
            let outlines: Vec<Outline<'_>> =
                subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
            CentreSkeleton::build(&outlines)
                .unwrap()
                .finish(options, &[])
        })
        .collect()
}

/// The node budget of criterion 20 for a path of length `l` mm: nodes, width points.
pub(super) fn budget(l: f64) -> (usize, usize) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let per = (0.12 * l).ceil() as usize;
    (2 + per, 6 + per)
}

/// One path's numbers.
pub(super) struct PathStats {
    pub(super) role: CentreRole,
    pub(super) length: f64,
    /// Width quartiles (25, 50, 75 %) and the largest, in mm.
    pub(super) widths: [f64; 4],
    pub(super) nodes: usize,
    pub(super) points: usize,
}

pub(super) fn path_stats(lines: &CentreLines) -> Vec<PathStats> {
    lines
        .paths
        .iter()
        .map(|path| {
            let measure = PathMeasure::new(&path.anchors, path.closed);
            let mut w: Vec<f64> = (0..=100)
                .map(|i| path.width.as_mm() * path.profile.sample(f64::from(i) / 100.0).unwrap())
                .collect();
            w.sort_by(f64::total_cmp);
            PathStats {
                role: path.role,
                length: measure.length().as_mm(),
                widths: [w[25], w[50], w[75], w[100]],
                nodes: path.anchors.len(),
                points: path.profile.positions().count(),
            }
        })
        .collect()
}

/// The axis numbers of hypotheses 2 to 5 for the chains of `region` (its thin parts when split)
/// whose median width is at least `band` mm: per chain its length, width quartiles, σ of the
/// width around a running median over one width, the curve's `T` at the default, its corners,
/// and per region the junctions and the short branches (shorter than `4 r_J`).
fn axis_stats(region: &RegionSkeleton, band: f64, out: &mut String) {
    let mut junctions: Vec<usize> = Vec::new();
    let mut short = 0;
    for chain in &region.chains {
        let mut w: Vec<f64> = chain.samples.iter().map(|s| 2.0 * s.radius).collect();
        for end in [chain.start, chain.end] {
            if let ChainEnd::Junction {
                junction, radius, ..
            } = end
            {
                junctions.push(junction);
                if super::is_branch(chain) && chain.length() < 4.0 * radius {
                    short += 1;
                }
            }
        }
        let widths = w.clone();
        w.sort_by(f64::total_cmp);
        let median = w[w.len() / 2];
        if median < band {
            continue;
        }
        // σ around a running median over one local width of arc length.
        let mut along = vec![0.0];
        for pair in chain.samples.windows(2) {
            along.push(along[along.len() - 1] + (pair[1].point - pair[0].point).hypot());
        }
        let mut sq = 0.0;
        for i in 0..widths.len() {
            let mut window: Vec<f64> = (0..widths.len())
                .filter(|&k| (along[k] - along[i]).abs() <= 0.5 * widths[i])
                .map(|k| widths[k])
                .collect();
            window.sort_by(f64::total_cmp);
            let d = widths[i] - window[window.len() / 2];
            sq += d * d;
        }
        #[allow(clippy::cast_precision_loss)]
        let sigma = (sq / widths.len() as f64).sqrt();
        let narrow = w[w.len() / 4];
        let _ = writeln!(
            out,
            "    chain {:6} L {:6.1} w {:.2}/{:.2}/{:.2} σ {:.3} vs T/2 {:.3}, T used {:.3}, {} samples, {} corners",
            if super::is_branch(chain) {
                "branch"
            } else {
                "main"
            },
            chain.length(),
            narrow,
            median,
            w[3 * w.len() / 4],
            sigma,
            tolerance(median) / 2.0,
            tolerance(narrow) * Budget::of(super::DEFAULT_OPTIMISE).share,
            chain.samples.len(),
            super::fit::corners(&chain.samples).len(),
        );
    }
    junctions.sort_unstable();
    junctions.dedup();
    let _ = writeln!(
        out,
        "  region: {} chains, {} junctions, {short} short branches",
        region.chains.len(),
        junctions.len()
    );
}

fn svg(lines: &[CentreLines], subjects: &[Vec<(Vec<OutlineTriple>, bool)>]) -> String {
    let mut out = String::from(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 240 300' width='1600' height='2000'>\
         <rect width='240' height='300' fill='white'/>\n",
    );
    let bez = |anchors: &[OutlineTriple], closed: bool| {
        let mut d = String::new();
        let n = anchors.len();
        let _ = write!(d, "M{} {}", anchors[0].0.x, anchors[0].0.y);
        let count = if closed { n } else { n - 1 };
        for i in 0..count {
            let (a, b) = (anchors[i], anchors[(i + 1) % n]);
            let _ = write!(
                d,
                " C{} {} {} {} {} {}",
                a.0.x + a.2.x,
                a.0.y + a.2.y,
                b.0.x + b.1.x,
                b.0.y + b.1.y,
                b.0.x,
                b.0.y
            );
        }
        if closed {
            d.push('Z');
        }
        d
    };
    for subject in subjects {
        let d: String = subject.iter().map(|(a, c)| bez(a, *c)).collect();
        let _ = writeln!(out, "<path d='{d}' fill='#ddd' fill-rule='nonzero'/>");
    }
    for result in lines {
        for region in &result.filled {
            let d: String = region.iter().map(|ring| bez(ring, true)).collect();
            let _ = writeln!(
                out,
                "<path d='{d}' fill='#7c7' fill-opacity='0.6' fill-rule='nonzero'/>"
            );
            for a in region.iter().flatten() {
                let _ = writeln!(
                    out,
                    "<circle cx='{}' cy='{}' r='0.3' fill='#070'/>",
                    a.0.x, a.0.y
                );
            }
        }
        for path in &result.paths {
            let length = PathMeasure::new(&path.anchors, path.closed)
                .length()
                .as_mm();
            let (bn, bw) = budget(length);
            let over = path.anchors.len() > bn || path.profile.positions().count() > bw;
            let colour = match (over, path.role) {
                (true, _) => "#f80",
                (false, CentreRole::Main) => "#d22",
                (false, CentreRole::Branch) => "#22d",
            };
            let _ = writeln!(
                out,
                "<path d='{}' fill='none' stroke='{colour}' stroke-width='0.25'/>",
                bez(&path.anchors, path.closed)
            );
            for a in &path.anchors {
                let _ = writeln!(
                    out,
                    "<circle cx='{}' cy='{}' r='0.35' fill='{colour}'/>",
                    a.0.x, a.0.y
                );
            }
        }
    }
    out.push_str("</svg>\n");
    out
}

/// The anchors of `path` with the gaps between them, and its profile positions.
fn dump(path: &super::CentrePath) {
    let mut last: Option<curvyo_document_core::Point> = None;
    for a in &path.anchors {
        let gap = last.map_or(0.0, |l| (a.0.x - l.x).hypot(a.0.y - l.y));
        eprintln!(
            "  anchor ({:.3}, {:.3}) gap {gap:.3} in {:.3} out {:.3}",
            a.0.x,
            a.0.y,
            a.1.x.hypot(a.1.y),
            a.2.x.hypot(a.2.y)
        );
        last = Some(a.0);
    }
    let positions: Vec<String> = path
        .profile
        .positions()
        .map(|p| format!("{p:.3}"))
        .collect();
    eprintln!("  profile at {}", positions.join(" "));
}

#[test]
#[ignore = "a measurement, run by hand"]
fn measure_traced() {
    let text = std::env::var("CURVYO_CENTRE_INPUT").map_or_else(
        |_| include_str!("../../tests/fixtures/centre_lines/dolphin_traced.txt").to_owned(),
        |path| std::fs::read_to_string(path).unwrap(),
    );
    let subjects = traced_subjects(&text);
    let input: usize = subjects
        .iter()
        .flatten()
        .map(|(anchors, _)| anchors.len())
        .sum();
    let options = CentreOptions::default();
    let mut axis = String::new();
    for subject in &subjects {
        let outlines: Vec<Outline<'_>> = subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
        for region in &CentreSkeleton::build(&outlines).unwrap().regions {
            axis_stats(region, 1.5, &mut axis);
        }
    }
    eprintln!("{axis}");
    let lines = traced_lines(&text, &options);
    let mut over = 0;
    let mut table = String::new();
    let (mut nodes, mut points, mut filled_nodes) = (0, 0, 0);
    for (s, result) in lines.iter().enumerate() {
        let filled: usize = result.filled.iter().flatten().map(Vec::len).sum();
        filled_nodes += filled;
        let _ = writeln!(
            table,
            "subject {s}: {} paths, {} filled parts ({filled} nodes)",
            result.paths.len(),
            result.filled.len()
        );
        for (i, p) in path_stats(result).iter().enumerate() {
            let (bn, bw) = budget(p.length);
            let fits = p.nodes <= bn && p.points <= bw;
            over += usize::from(!fits);
            nodes += p.nodes;
            points += p.points;
            let _ = writeln!(
                table,
                "  {i:3} {:6} L {:7.1} w {:.2}/{:.2}/{:.2} max {:.2}  nodes {:3}/{bn:3} points {:3}/{bw:3}{}",
                format!("{:?}", p.role),
                p.length,
                p.widths[0],
                p.widths[1],
                p.widths[2],
                p.widths[3],
                p.nodes,
                p.points,
                if fits { "" } else { "  OVER" }
            );
        }
    }
    eprintln!("{table}");
    let paths: usize = lines.iter().map(|l| l.paths.len()).sum();
    eprintln!(
        "input {input} nodes; result {paths} paths, {nodes} nodes, {points} width points, \
         {filled_nodes} kept-filled vertices, {over} paths over budget"
    );
    for optimise in [0.0, 0.5, 0.75, 1.0] {
        let lines = traced_lines(
            &text,
            &CentreOptions {
                optimise,
                ..options
            },
        );
        let stats: Vec<PathStats> = lines.iter().flat_map(path_stats).collect();
        eprintln!(
            "optimise {optimise}: {} nodes, {} width points",
            stats.iter().map(|p| p.nodes).sum::<usize>(),
            stats.iter().map(|p| p.points).sum::<usize>()
        );
    }
    if let Ok(index) = std::env::var("CURVYO_CENTRE_PATH") {
        dump(&lines[0].paths[index.parse::<usize>().unwrap()]);
    }
    if let Ok(path) = std::env::var("CURVYO_CENTRE_SVG") {
        std::fs::write(path, svg(&lines, &subjects)).unwrap();
    }
}
