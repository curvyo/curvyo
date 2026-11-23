//! The centre of a gear: the bore with its keyway and the rounded windows between the spokes.

use curvyo_document_core::Point;
use curvyo_extension_core::ClosedPath;

use crate::outline::{Builder, polar, reversed};

/// The bore circle, wound clockwise (a hole), with a keyway slot at the top when `keyway` holds
/// its width and the depth the slot reaches beyond the bore circle.
pub(crate) fn bore(radius: f64, keyway: Option<(f64, f64)>) -> ClosedPath {
    let origin = Point::new(0.0, 0.0);
    let Some((width, depth)) = keyway else {
        let mut builder = Builder::new(polar(origin, radius, 0.0));
        builder.arc(origin, radius, 0.0, -std::f64::consts::TAU);
        builder.snap_end();
        return builder.finish();
    };
    let half = (width / 2.0).min(radius * 0.999);
    let foot = (radius * radius - half * half).sqrt();
    let angle = libm::atan2(foot, half);
    let mut builder = Builder::new(Point::new(half, foot));
    builder.arc(
        origin,
        radius,
        angle,
        angle - std::f64::consts::PI - 2.0 * angle,
    );
    // The walk came round to the left foot of the slot.
    builder.line_to(Point::new(-half, radius + depth));
    builder.line_to(Point::new(half, radius + depth));
    builder.finish()
}

/// What the windows between spokes are built from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Spokes {
    pub(crate) count: u32,
    pub(crate) spoke_width: f64,
    pub(crate) hub_radius: f64,
    pub(crate) rim_inner_radius: f64,
    pub(crate) fillet: f64,
}

impl Spokes {
    /// The smallest hub radius at which the windows still open (the spoke edges meet at the hub
    /// otherwise).
    pub(crate) fn min_hub_radius(count: u32, spoke_width: f64, fillet: f64) -> f64 {
        let half = std::f64::consts::PI / f64::from(count);
        (spoke_width / 2.0 + fillet) / libm::sin(half) - fillet
    }

    /// The window outlines, clockwise holes, or none when no window fits.
    pub(crate) fn windows(&self) -> Vec<ClosedPath> {
        let f = self.fillet;
        let d = self.spoke_width / 2.0 + f;
        let half = std::f64::consts::PI / f64::from(self.count);
        let (rho_in, rho_out) = (self.hub_radius + f, self.rim_inner_radius - f);
        if rho_out - rho_in < 1e-3 || rho_in * libm::sin(half) < d - 1e-9 {
            return Vec::new();
        }
        let window = self.window(d, rho_in, rho_out);
        (0..self.count)
            .map(|index| {
                let turn = half + 2.0 * half * f64::from(index);
                reversed(&rotated(&window, turn))
            })
            .collect()
    }

    /// One window with its bisector along +x, counter-clockwise.
    #[allow(clippy::many_single_char_names)] // the four corners a, b, c, d of the window
    fn window(&self, d: f64, rho_in: f64, rho_out: f64) -> ClosedPath {
        let f = self.fillet;
        let half = std::f64::consts::PI / f64::from(self.count);
        let (sin, cos) = (libm::sin(half), libm::cos(half));
        // Upper spoke edge: direction u, the normal n pointing into the window.
        let (u, n) = ((cos, sin), (sin, -cos));
        let corner = |rho: f64, mirrored: bool| {
            let along = (rho * rho - d * d).max(0.0).sqrt();
            let x = d * n.0 + along * u.0;
            let y = d * n.1 + along * u.1;
            Point::new(x, if mirrored { -y } else { y })
        };
        let (a, b) = (corner(rho_in, false), corner(rho_out, false));
        let (dd, c) = (corner(rho_in, true), corner(rho_out, true));
        let angle = |p: Point| libm::atan2(p.y, p.x);
        let (ang_a, ang_b) = (angle(a), angle(b));
        let origin = Point::new(0.0, 0.0);
        let pi = std::f64::consts::PI;
        let (rim_r, hub_r) = (rho_out + f, rho_in - f);
        let mut builder = Builder::new(polar(origin, rim_r, -ang_b));
        // Outer arc up to the upper corner.
        builder.arc(origin, rim_r, -ang_b, ang_b);
        // Upper outer corner round to the upper spoke edge.
        let edge_normal = pi / 2.0 + half;
        builder.arc(b, f, ang_b, edge_normal);
        // Along the upper spoke edge to the hub.
        builder.line_to(Point::new(a.x - f * n.0, a.y - f * n.1));
        // Upper inner corner round to the hub circle.
        builder.arc(a, f, edge_normal, ang_a + pi);
        // The hub circle, clockwise, across to the lower corner.
        builder.arc(origin, hub_r, ang_a, -ang_a);
        // Lower inner corner round to the lower spoke edge.
        builder.arc(dd, f, pi - ang_a, 3.0 * pi / 2.0 - half);
        // Along the lower spoke edge to the rim.
        builder.line_to(Point::new(c.x - f * n.0, c.y + f * n.1));
        // Lower outer corner back to the outer arc.
        builder.arc(c, f, 3.0 * pi / 2.0 - half, 2.0 * pi - ang_b);
        builder.snap_end();
        builder.finish()
    }
}

fn rotated(path: &ClosedPath, angle: f64) -> ClosedPath {
    let (sin, cos) = (libm::sin(angle), libm::cos(angle));
    let turn = |p: Point| Point::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
    ClosedPath {
        start: turn(path.start),
        segments: path
            .segments
            .iter()
            .map(|s| curvyo_extension_core::Cubic {
                c1: turn(s.c1),
                c2: turn(s.c2),
                to: turn(s.to),
            })
            .collect(),
    }
}
