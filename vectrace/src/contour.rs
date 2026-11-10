//! Walks the crack edges around one pixel shape into its closed boundary chain
//! (`specs/0089` adrs.md §5): integer pixel corners, one per unit edge, replacing visioncortex's
//! walk and its `u16` limit.

/// A pixel corner: `x` right, `y` down, in pixels of the shape's box.
pub(crate) type Vertex = (i32, i32);

/// The unit steps right, down, left, up; turning right on screen adds one.
const STEPS: [Vertex; 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// The outer boundary of `shape` (`width` x `height`, row-major), clockwise on screen (the shape
/// on the right of the walk, `y` down), one vertex per unit crack edge, starting at the top left
/// corner of the first set pixel in scan order; empty when no pixel is set. Where two set pixels
/// touch only diagonally the walk turns into the diagonal pixel when `eight` (a foreground
/// shape, 8-connected like `regions.rs`) and away from it otherwise (a hole, 4-connected).
pub(crate) fn outer_loop(shape: &[bool], width: usize, height: usize, eight: bool) -> Vec<Vertex> {
    let Some(first) = shape.iter().position(|&s| s) else {
        return Vec::new();
    };
    let set = |x: i32, y: i32| -> bool {
        match (usize::try_from(x), usize::try_from(y)) {
            (Ok(x), Ok(y)) => x < width && y < height && shape[y * width + x],
            _ => false,
        }
    };
    // invariant: boxes are at most 2 x 8192 pixels wide (`raster.rs`), so they fit `i32`.
    let start = (
        i32::try_from(first % width).unwrap_or(i32::MAX),
        i32::try_from(first / width).unwrap_or(i32::MAX),
    );
    let mut chain = Vec::new();
    let (mut at, mut dir) = (start, 0_usize);
    loop {
        chain.push(at);
        at = (at.0 + STEPS[dir].0, at.1 + STEPS[dir].1);
        // The two pixels ahead of `at` in direction `dir`, left and right of the walk.
        let (ahead_left, ahead_right) = match dir {
            0 => ((at.0, at.1 - 1), (at.0, at.1)),
            1 => ((at.0, at.1), (at.0 - 1, at.1)),
            2 => ((at.0 - 1, at.1), (at.0 - 1, at.1 - 1)),
            _ => ((at.0 - 1, at.1 - 1), (at.0, at.1 - 1)),
        };
        let (left, right) = (
            set(ahead_left.0, ahead_left.1),
            set(ahead_right.0, ahead_right.1),
        );
        dir = match (left, right) {
            (true, true) => (dir + 3) % 4,
            (true, false) if eight => (dir + 3) % 4,
            (_, false) => (dir + 1) % 4,
            (false, true) => dir,
        };
        if (at, dir) == (start, 0) {
            break;
        }
    }
    chain
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    fn shape(rows: &[&str]) -> (Vec<bool>, usize, usize) {
        let bits = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        (bits, rows[0].len(), rows.len())
    }

    fn walk(rows: &[&str]) -> Vec<Vertex> {
        let (bits, w, h) = shape(rows);
        outer_loop(&bits, w, h, false)
    }

    /// Twice the signed area of a closed vertex chain (positive clockwise on screen).
    fn area2(chain: &[Vertex]) -> i32 {
        (0..chain.len())
            .map(|i| {
                let (a, b) = (chain[i], chain[(i + 1) % chain.len()]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum()
    }

    #[test]
    fn a_pixel_is_four_edges_clockwise() {
        assert_eq!(walk(&["#"]), [(0, 0), (1, 0), (1, 1), (0, 1)]);
        assert_eq!(walk(&[".."]), Vec::<Vertex>::new());
    }

    #[test]
    fn a_rectangle_has_its_perimeter_and_area() {
        let chain = walk(&["....", ".###", ".###"]);
        assert_eq!(chain.len(), 10);
        assert_eq!(chain[0], (1, 1));
        assert_eq!(area2(&chain), 12);
    }

    #[test]
    fn eight_connected_walks_into_the_diagonal_pixel() {
        let (bits, w, h) = shape(&["#.", ".#"]);
        let chain = outer_loop(&bits, w, h, true);
        assert_eq!(chain.len(), 8);
        assert_eq!(chain.iter().filter(|&&v| v == (1, 1)).count(), 2);
        assert_eq!(area2(&chain), 4);
    }

    #[test]
    fn diagonal_pixels_are_not_joined() {
        // Only the first pixel's square: the diagonal one is another shape.
        let chain = walk(&["#.", ".#"]);
        assert_eq!(chain, [(0, 0), (1, 0), (1, 1), (0, 1)]);
    }

    #[test]
    fn a_saddle_inside_one_shape_is_passed_twice() {
        // The shape is 4-connected around; its two arms touch diagonally at (2, 2), where the
        // walk turns into the empty pixel (1, 1) (`trace.rs` fills such pockets first).
        let chain = walk(&["###", "#.#", "##.", "..."]);
        assert_eq!(chain.iter().filter(|&&v| v == (2, 2)).count(), 2);
        // 7 pixels, every unit edge once.
        assert_eq!(area2(&chain), 14);
        assert_eq!(chain.len(), 16);
    }

    #[test]
    fn a_ring_walks_only_its_outside() {
        let chain = walk(&["###", "#.#", "###"]);
        assert_eq!(chain.len(), 12);
        assert_eq!(area2(&chain), 18);
    }
}
