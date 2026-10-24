//! Builds small TrueType faces in memory for the glyph list tests (`specs/0082-glyph-table`):
//! chosen `cmap` subtables, square or empty glyphs and `post` glyph names, so a test states
//! exactly which characters a face maps.
#![allow(dead_code)]

/// One `cmap` subtable: platform, encoding, format (4 or 12) and its `(code, glyph)` pairs.
pub struct Subtable {
    pub platform: u16,
    pub encoding: u16,
    pub format: u16,
    pub map: Vec<(u32, u16)>,
}

/// A face to build: its glyphs (name, drawn as a square or empty) and its subtables. Glyph 0 is
/// `.notdef` and is added in front.
pub struct TestFace {
    pub glyphs: Vec<(&'static str, bool)>,
    pub subtables: Vec<Subtable>,
    pub post_names: bool,
}

fn be16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn be32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn square() -> Vec<u8> {
    let mut g = Vec::new();
    be16(&mut g, 1); // contours
    for v in [50_i16, 0, 550, 700] {
        g.extend_from_slice(&v.to_be_bytes());
    }
    be16(&mut g, 3); // end point of the contour
    be16(&mut g, 0); // instructions
    g.extend_from_slice(&[1, 1, 1, 1]); // on-curve, 16-bit deltas
    for dx in [50_i16, 500, 0, -500] {
        g.extend_from_slice(&dx.to_be_bytes());
    }
    for dy in [0_i16, 0, 700, 0] {
        g.extend_from_slice(&dy.to_be_bytes());
    }
    g
}

fn cmap(subtables: &[Subtable]) -> Vec<u8> {
    let bodies: Vec<Vec<u8>> = subtables.iter().map(subtable).collect();
    let mut out = Vec::new();
    be16(&mut out, 0);
    be16(&mut out, u16::try_from(subtables.len()).unwrap());
    let mut offset = 4 + 8 * subtables.len();
    for (table, body) in subtables.iter().zip(&bodies) {
        be16(&mut out, table.platform);
        be16(&mut out, table.encoding);
        be32(&mut out, u32::try_from(offset).unwrap());
        offset += body.len();
    }
    for body in bodies {
        out.extend(body);
    }
    out
}

fn subtable(table: &Subtable) -> Vec<u8> {
    let mut map = table.map.clone();
    map.sort_unstable();
    let mut out = Vec::new();
    if table.format == 12 {
        be16(&mut out, 12);
        be16(&mut out, 0);
        be32(&mut out, u32::try_from(16 + 12 * map.len()).unwrap());
        be32(&mut out, 0);
        be32(&mut out, u32::try_from(map.len()).unwrap());
        for (code, glyph) in map {
            be32(&mut out, code);
            be32(&mut out, code);
            be32(&mut out, u32::from(glyph));
        }
        return out;
    }
    // Format 4: one segment per pair, then the closing 0xFFFF segment.
    let mut segments: Vec<(u16, u16)> = map
        .iter()
        .map(|&(code, glyph)| (u16::try_from(code).unwrap(), glyph))
        .collect();
    segments.push((0xFFFF, 0));
    let count = u16::try_from(segments.len()).unwrap();
    let length = 16 + 8 * usize::from(count);
    be16(&mut out, 4);
    be16(&mut out, u16::try_from(length).unwrap());
    be16(&mut out, 0);
    be16(&mut out, count * 2);
    let mut search = 1_u16;
    let mut selector = 0_u16;
    while search * 2 <= count {
        search *= 2;
        selector += 1;
    }
    be16(&mut out, search * 2);
    be16(&mut out, selector);
    be16(&mut out, count * 2 - search * 2);
    for (code, _) in &segments {
        be16(&mut out, *code);
    }
    be16(&mut out, 0);
    for (code, _) in &segments {
        be16(&mut out, *code);
    }
    for (code, glyph) in &segments {
        let delta = if *code == 0xFFFF {
            1
        } else {
            glyph.wrapping_sub(*code)
        };
        be16(&mut out, delta);
    }
    for _ in &segments {
        be16(&mut out, 0);
    }
    out
}

fn post(names: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0002_0000);
    out.extend_from_slice(&[0; 28]);
    be16(&mut out, u16::try_from(names.len() + 1).unwrap());
    be16(&mut out, 0); // .notdef, a standard Macintosh name
    for i in 0..names.len() {
        be16(&mut out, 258 + u16::try_from(i).unwrap());
    }
    for name in names {
        out.push(u8::try_from(name.len()).unwrap());
        out.extend_from_slice(name.as_bytes());
    }
    out
}

/// The bytes of `face` as a TrueType file.
pub fn build(face: &TestFace) -> Vec<u8> {
    let count = u16::try_from(face.glyphs.len() + 1).unwrap();
    let mut glyf = square();
    let mut loca = vec![0_u32, u32::try_from(glyf.len()).unwrap()];
    for (_, drawn) in &face.glyphs {
        if *drawn {
            glyf.extend(square());
        }
        loca.push(u32::try_from(glyf.len()).unwrap());
    }
    let mut head = Vec::new();
    be32(&mut head, 0x0001_0000);
    be32(&mut head, 0x0001_0000);
    be32(&mut head, 0);
    be32(&mut head, 0x5F0F_3CF5);
    be16(&mut head, 0);
    be16(&mut head, 1000);
    head.extend_from_slice(&[0; 16]);
    for v in [0_i16, -200, 1000, 800] {
        head.extend_from_slice(&v.to_be_bytes());
    }
    be16(&mut head, 0);
    be16(&mut head, 8);
    be16(&mut head, 2);
    be16(&mut head, 1); // long loca
    be16(&mut head, 0);
    let mut hhea = Vec::new();
    be32(&mut hhea, 0x0001_0000);
    for v in [800_i16, -200, 0] {
        hhea.extend_from_slice(&v.to_be_bytes());
    }
    be16(&mut hhea, 600);
    hhea.extend_from_slice(&[0; 6]);
    be16(&mut hhea, 1);
    hhea.extend_from_slice(&[0; 14]);
    be16(&mut hhea, count);
    let mut maxp = Vec::new();
    be32(&mut maxp, 0x0001_0000);
    be16(&mut maxp, count);
    be16(&mut maxp, 4);
    be16(&mut maxp, 1);
    maxp.extend_from_slice(&[0; 22]);
    let mut hmtx = Vec::new();
    for _ in 0..count {
        be16(&mut hmtx, 600);
        be16(&mut hmtx, 50);
    }
    let mut loca_bytes = Vec::new();
    for offset in loca {
        be32(&mut loca_bytes, offset);
    }
    let names: Vec<&str> = face.glyphs.iter().map(|(name, _)| *name).collect();
    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"cmap", cmap(&face.subtables)),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca_bytes),
        (b"maxp", maxp),
    ];
    if face.post_names {
        tables.push((b"post", post(&names)));
    }
    tables.sort_by_key(|(tag, _)| **tag);
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be16(&mut out, u16::try_from(tables.len()).unwrap());
    out.extend_from_slice(&[0; 6]);
    let mut offset = 12 + 16 * tables.len();
    for (tag, body) in &tables {
        out.extend_from_slice(*tag);
        be32(&mut out, 0);
        be32(&mut out, u32::try_from(offset).unwrap());
        be32(&mut out, u32::try_from(body.len()).unwrap());
        offset += body.len().next_multiple_of(4);
    }
    for (_, body) in tables {
        out.extend(body);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    out
}

/// The spec's four-character face (criterion 5): "A", "ä", "♥" (U+2665), U+E000 and the space
/// mapped, "ü" drawn but unmapped, U+0007 (a control character) mapped.
pub fn four_char_face() -> Vec<u8> {
    build(&TestFace {
        glyphs: vec![
            ("space", false),
            ("A", true),
            ("adieresis", true),
            ("heart", true),
            ("uniE000", true),
            ("udieresis", true),
            ("bell", true),
        ],
        subtables: vec![Subtable {
            platform: 3,
            encoding: 1,
            format: 4,
            map: vec![
                (0x20, 1),
                (0x41, 2),
                (0xE4, 3),
                (0x2665, 4),
                (0xE000, 5),
                (0x07, 7),
            ],
        }],
        post_names: true,
    })
}

/// A symbol-encoded face (`(3, 0)` subtable): its codes in `U+F020..U+F0FF`, no `post` names.
pub fn symbol_face() -> Vec<u8> {
    build(&TestFace {
        glyphs: vec![("a", true), ("b", true), ("c", true)],
        subtables: vec![Subtable {
            platform: 3,
            encoding: 0,
            format: 4,
            map: vec![(0xF041, 1), (0xF0A9, 2), (0xF100, 3)],
        }],
        post_names: false,
    })
}

/// A face whose two Unicode subtables disagree: the format 12 table (the one a shaper picks)
/// maps U+1F600 and "B"; the format 4 table maps "A" and "B".
pub fn two_subtable_face() -> Vec<u8> {
    build(&TestFace {
        glyphs: vec![("A", true), ("B", true), ("smile", true)],
        subtables: vec![
            Subtable {
                platform: 3,
                encoding: 1,
                format: 4,
                map: vec![(0x41, 1), (0x42, 2)],
            },
            Subtable {
                platform: 3,
                encoding: 10,
                format: 12,
                map: vec![(0x42, 2), (0x1F600, 3)],
            },
        ],
        post_names: true,
    })
}
