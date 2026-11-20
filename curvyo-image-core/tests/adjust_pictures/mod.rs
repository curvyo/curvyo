//! The three golden pictures of the adjustments (`specs/0135` criterion 54), made from integers
//! only: a colour photo with soft shapes and grain, a flat graphic with JPEG-like speckle, and a
//! pen drawing on unevenly lit paper. 64 x 64 pixels each.

use curvyo_image_core::WorkingImage;

/// The side of every golden picture.
pub const SIDE: u32 = 64;

/// A small deterministic generator, -`spread` to `spread`.
fn grain(seed: &mut u32, spread: i32) -> i32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    i32::try_from(*seed >> 16).unwrap() % (2 * spread + 1) - spread
}

fn byte(v: i32) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap()
}

fn picture(mut pixel: impl FnMut(i32, i32) -> [u8; 4]) -> WorkingImage {
    let mut pixels = Vec::new();
    for y in 0..i32::try_from(SIDE).unwrap() {
        for x in 0..i32::try_from(SIDE).unwrap() {
            pixels.extend_from_slice(&pixel(x, y));
        }
    }
    WorkingImage {
        width: SIDE,
        height: SIDE,
        pixels,
        original_size: (SIDE, SIDE),
        truncated: false,
        animated: false,
    }
}

/// A photo: a warm background gradient, a lighter "face" disc with a dark "eye", grain of +-10.
#[must_use]
pub fn photo() -> WorkingImage {
    let mut seed = 11;
    picture(|x, y| {
        let mut base = 60 + x + y / 2;
        let (dx, dy) = (x - 34, y - 30);
        let d2 = dx * dx + dy * dy;
        if d2 < 400 {
            base += 70 - d2 / 8;
        }
        if (x - 40) * (x - 40) + (y - 26) * (y - 26) < 12 {
            base = 40;
        }
        let n = grain(&mut seed, 10);
        [
            byte(base + 20 + n),
            byte(base + n),
            byte(base - 25 + n),
            255,
        ]
    })
}

/// A flat graphic: four flat greys, a dark bar, speckle of +-3, a transparent corner.
#[must_use]
pub fn graphic() -> WorkingImage {
    let mut seed = 23;
    picture(|x, y| {
        let mut g = match (x < 32, y < 32) {
            (true, true) => 230,
            (false, true) => 170,
            (true, false) => 110,
            (false, false) => 50,
        };
        if (20..44).contains(&x) && (28..36).contains(&y) {
            g = 20;
        }
        let g = byte(g + grain(&mut seed, 3));
        let a = if x + y < 8 { 0 } else { 255 };
        [g, g, g, a]
    })
}

/// A pen drawing: paper from 205 to 245 across, two 2 px strokes and a 1 px curve of ink.
#[must_use]
pub fn pen() -> WorkingImage {
    let mut seed = 5;
    picture(|x, y| {
        let paper = 205 + (x * 40) / 63 + grain(&mut seed, 2);
        let stroke = (x - y).abs() <= 1 && (8..56).contains(&x)
            || (30..32).contains(&y) && (6..58).contains(&x)
            || ((x - 32) * (x - 32) + (y - 32) * (y - 32) - 400).abs() < 20;
        let g = byte(if stroke { 30 } else { paper });
        [g, g, g, 255]
    })
}

/// The golden picture called `name`.
#[must_use]
pub fn by_name(name: &str) -> WorkingImage {
    match name {
        "photo" => photo(),
        "graphic" => graphic(),
        "pen" => pen(),
        _ => panic!("no golden picture {name}"),
    }
}
