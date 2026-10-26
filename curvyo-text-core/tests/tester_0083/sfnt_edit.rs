//! Tester helper for `specs/0083-font-info-and-zip`: rewrites the tables of a single-face sfnt
//! (replace or drop a table, a new `name` table) and joins faces into a collection, so the info
//! tests state each field's value exactly.
#![allow(dead_code, clippy::unwrap_used, clippy::cast_possible_truncation)]

/// A table: its tag and bytes.
pub type Table = ([u8; 4], Vec<u8>);

/// The tables of a single-face file: (tag, bytes), in directory order.
pub fn tables(font: &[u8]) -> (u32, Vec<Table>) {
    let version = u32::from_be_bytes(font[0..4].try_into().unwrap());
    let n = usize::from(u16::from_be_bytes([font[4], font[5]]));
    let list = (0..n)
        .map(|i| {
            let r = 12 + 16 * i;
            let tag: [u8; 4] = font[r..r + 4].try_into().unwrap();
            let off = u32::from_be_bytes(font[r + 8..r + 12].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(font[r + 12..r + 16].try_into().unwrap()) as usize;
            (tag, font[off..off + len].to_vec())
        })
        .collect();
    (version, list)
}

fn pad4(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(4) {
        v.push(0);
    }
}

fn directory(version: u32, list: &[Table], base: usize) -> (Vec<u8>, Vec<u8>) {
    let n = list.len() as u16;
    let mut head = Vec::new();
    head.extend_from_slice(&version.to_be_bytes());
    head.extend_from_slice(&n.to_be_bytes());
    head.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    let mut body = Vec::new();
    let dir_len = 12 + 16 * list.len();
    for (tag, data) in list {
        let off = base + dir_len + body.len();
        head.extend_from_slice(tag);
        head.extend_from_slice(&0_u32.to_be_bytes());
        head.extend_from_slice(&(off as u32).to_be_bytes());
        head.extend_from_slice(&(data.len() as u32).to_be_bytes());
        body.extend_from_slice(data);
        pad4(&mut body);
    }
    (head, body)
}

/// A single-face file from `version` and `list`.
pub fn assemble(version: u32, list: &[Table]) -> Vec<u8> {
    let (mut head, body) = directory(version, list, 0);
    head.extend(body);
    head
}

/// `font` with table `tag` replaced by `data` (`None` drops it).
pub fn with_table(font: &[u8], tag: [u8; 4], data: Option<Vec<u8>>) -> Vec<u8> {
    let (version, mut list) = tables(font);
    match data {
        Some(d) => {
            if let Some(slot) = list.iter_mut().find(|(t, _)| *t == tag) {
                slot.1 = d;
            } else {
                list.push((tag, d));
                list.sort_by_key(|(t, _)| *t);
            }
        }
        None => list.retain(|(t, _)| *t != tag),
    }
    assemble(version, &list)
}

/// A `name` table (format 0) of Windows Unicode records: (name ID, language ID, text).
pub fn name_table(records: &[(u16, u16, &str)]) -> Vec<u8> {
    let mut strings = Vec::new();
    let mut recs = Vec::new();
    let mut sorted: Vec<_> = records.to_vec();
    sorted.sort_by_key(|(id, lang, _)| (*lang, *id));
    for (id, lang, text) in sorted {
        let utf16: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        recs.push((
            3_u16,
            1_u16,
            lang,
            id,
            utf16.len() as u16,
            strings.len() as u16,
        ));
        strings.extend(utf16);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&0_u16.to_be_bytes());
    out.extend_from_slice(&(recs.len() as u16).to_be_bytes());
    out.extend_from_slice(&((6 + 12 * recs.len()) as u16).to_be_bytes());
    for (p, e, l, id, len, off) in recs {
        for v in [p, e, l, id, len, off] {
            out.extend_from_slice(&v.to_be_bytes());
        }
    }
    out.extend(strings);
    out
}

/// `font` with OS/2 `fsType` and `achVendID` set.
pub fn with_os2(font: &[u8], fs_type: u16, vendor: [u8; 4]) -> Vec<u8> {
    let (_, list) = tables(font);
    let mut os2 = list.iter().find(|(t, _)| t == b"OS/2").unwrap().1.clone();
    os2[8..10].copy_from_slice(&fs_type.to_be_bytes());
    os2[58..62].copy_from_slice(&vendor);
    with_table(font, *b"OS/2", Some(os2))
}

/// A collection of `faces` (each a single-face file; tables are copied per face).
pub fn collection(faces: &[Vec<u8>]) -> Vec<u8> {
    let header_len = 12 + 4 * faces.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"ttcf");
    out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    out.extend_from_slice(&(faces.len() as u32).to_be_bytes());
    out.resize(header_len, 0);
    for (i, face) in faces.iter().enumerate() {
        let (version, list) = tables(face);
        let base = out.len();
        let at = 12 + 4 * i;
        out[at..at + 4].copy_from_slice(&(base as u32).to_be_bytes());
        let (head, body) = directory(version, &list, base);
        out.extend(head);
        out.extend(body);
        pad4(&mut out);
    }
    out
}

/// The `maxp` glyph count of a single-face file.
pub fn maxp_glyphs(font: &[u8]) -> u32 {
    let (_, list) = tables(font);
    let maxp = &list.iter().find(|(t, _)| t == b"maxp").unwrap().1;
    u32::from(u16::from_be_bytes([maxp[4], maxp[5]]))
}
