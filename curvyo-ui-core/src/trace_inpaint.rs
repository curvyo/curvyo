//! Fills masked pixels from the ring around them (`specs/0091-trace-text-recognition` `adrs.md`
//! decision 3).
//!
//! Every 8-connected component of the mask takes, channel by channel, the median of the unmasked
//! pixels 8-adjacent to it (the lower of the two middle values for an even count), so a text on a
//! filled shape leaves the shape whole and the text on white leaves white. Integers, fixed scan
//! order, no hash maps: the same input gives the same pixels natively and in wasm.

/// Fills every pixel of `rgba` (`width` x `height`, four bytes each) whose `mask` entry is set
/// from its component's ring; a component without a ring (the whole picture) becomes white.
/// Does nothing when the sizes do not match.
pub fn inpaint(rgba: &mut [u8], width: usize, height: usize, mask: &[bool]) {
    let n = width * height;
    if mask.len() != n || rgba.len() != n * 4 {
        return;
    }
    let mut seen = vec![false; n];
    // The ring pixels of the component being filled, marked by its number to count each once.
    let mut ring_of = vec![usize::MAX; n];
    let mut stack = Vec::new();
    for start in 0..n {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut component = Vec::new();
        let mut ring = Vec::new();
        while let Some(i) = stack.pop() {
            component.push(i);
            let (x, y) = (i % width, i / width);
            for dy in -1_isize..=1 {
                for dx in -1_isize..=1 {
                    let (Some(nx), Some(ny)) = (x.checked_add_signed(dx), y.checked_add_signed(dy))
                    else {
                        continue;
                    };
                    if (dx, dy) == (0, 0) || nx >= width || ny >= height {
                        continue;
                    }
                    let j = ny * width + nx;
                    if mask[j] {
                        if !seen[j] {
                            seen[j] = true;
                            stack.push(j);
                        }
                    } else if ring_of[j] != start {
                        ring_of[j] = start;
                        ring.push(j);
                    }
                }
            }
        }
        let fill = median(rgba, &mut ring);
        for i in component {
            rgba[i * 4..i * 4 + 4].copy_from_slice(&fill);
        }
    }
}

/// The per-channel lower median of the pixels `ring`; white when it is empty.
fn median(rgba: &[u8], ring: &mut [usize]) -> [u8; 4] {
    if ring.is_empty() {
        return [255; 4];
    }
    let mut out = [0_u8; 4];
    let mut values: Vec<u8> = Vec::with_capacity(ring.len());
    for (c, value) in out.iter_mut().enumerate() {
        values.clear();
        values.extend(ring.iter().map(|&i| rgba[i * 4 + c]));
        values.sort_unstable();
        *value = values[(values.len() - 1) / 2];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hole_takes_the_median_of_its_ring() {
        // 5 x 3: grey shape with a black letter in the middle and one white pixel on the ring.
        let grey = [112, 112, 112, 255];
        let mut rgba: Vec<u8> = (0..15).flat_map(|_| grey).collect();
        rgba[7 * 4..7 * 4 + 4].copy_from_slice(&[0, 0, 0, 255]);
        rgba[0..4].copy_from_slice(&[255, 255, 255, 255]);
        let mut mask = vec![false; 15];
        mask[7] = true;
        inpaint(&mut rgba, 5, 3, &mask);
        assert_eq!(&rgba[7 * 4..7 * 4 + 4], &grey);
        assert_eq!(&rgba[0..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn a_fully_masked_picture_is_white_and_a_size_mismatch_is_ignored() {
        let mut rgba = vec![0_u8; 8];
        inpaint(&mut rgba, 2, 1, &[true, true]);
        assert_eq!(rgba, [255; 8]);
        let mut other = vec![1_u8; 8];
        inpaint(&mut other, 2, 1, &[true]);
        assert_eq!(other, [1; 8]);
    }
}
