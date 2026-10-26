//! Tester helper for `specs/0083-font-info-and-zip`: builds zip archives byte by byte, so a test
//! can state hostile fields exactly (names, flags, methods, CRC, declared sizes, Unix modes,
//! central records pointing at the same data). Shared by the text-core and storage-io tester
//! files through `#[path]`.
#![allow(dead_code, clippy::unwrap_used, clippy::cast_possible_truncation)]

/// One entry as written: the local header and the central record use the same fields unless a
/// `central_*` override is set.
#[derive(Clone)]
pub struct RawEntry {
    pub name: Vec<u8>,
    /// General purpose flags (bit 0 encrypted, bit 11 UTF-8 name).
    pub flags: u16,
    /// 0 stored, 8 deflate, anything else is another method.
    pub method: u16,
    pub crc: u32,
    /// The bytes behind the local header (compressed when `method` is 8).
    pub data: Vec<u8>,
    /// The declared uncompressed size.
    pub plain_size: u32,
    /// High byte 3 = Unix, so `external_attr >> 16` is the mode.
    pub made_by: u16,
    pub external_attr: u32,
    /// The central record points at this entry's local header instead of its own (overlap).
    pub central_offset_of: Option<usize>,
}

impl RawEntry {
    /// A stored regular file.
    pub fn stored(name: &str, bytes: &[u8]) -> Self {
        Self {
            name: name.as_bytes().to_vec(),
            flags: 0,
            method: 0,
            crc: crc32(bytes),
            data: bytes.to_vec(),
            plain_size: bytes.len() as u32,
            made_by: 0x0314,
            external_attr: 0o100_644 << 16,
            central_offset_of: None,
        }
    }

    /// A deflated file of `zeros` zero bytes (fixed Huffman, length-258 matches), with the
    /// correct CRC and size.
    pub fn deflated_zeros(name: &str, zeros: usize) -> Self {
        Self {
            name: name.as_bytes().to_vec(),
            flags: 0,
            method: 8,
            crc: crc32_zeros(zeros),
            data: deflate_zeros(zeros),
            plain_size: zeros as u32,
            made_by: 0x0314,
            external_attr: 0o100_644 << 16,
            central_offset_of: None,
        }
    }

    /// A deflated file of `bytes` (fixed Huffman literals only: valid, not small).
    pub fn deflated(name: &str, bytes: &[u8]) -> Self {
        Self {
            data: deflate_literals(bytes),
            method: 8,
            ..Self::stored(name, bytes)
        }
    }

    pub fn with_mode(mut self, mode: u32) -> Self {
        self.external_attr = mode << 16;
        self
    }

    pub fn raw_name(mut self, name: &[u8]) -> Self {
        self.name = name.to_vec();
        self
    }
}

/// The archive bytes of `entries`; `declared_count` overrides the end record's entry count.
pub fn build(entries: &[RawEntry]) -> Vec<u8> {
    build_with_count(entries, None)
}

pub fn build_with_count(entries: &[RawEntry], declared_count: Option<u16>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets = Vec::new();
    for e in entries {
        offsets.push(out.len() as u32);
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&e.flags.to_le_bytes());
        out.extend_from_slice(&e.method.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes()); // time, date
        out.extend_from_slice(&e.crc.to_le_bytes());
        out.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&e.plain_size.to_le_bytes());
        out.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&e.name);
        out.extend_from_slice(&e.data);
    }
    let cd_start = out.len() as u32;
    for (i, e) in entries.iter().enumerate() {
        let offset = offsets[e.central_offset_of.unwrap_or(i)];
        out.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        out.extend_from_slice(&e.made_by.to_le_bytes());
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&e.flags.to_le_bytes());
        out.extend_from_slice(&e.method.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&e.crc.to_le_bytes());
        out.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&e.plain_size.to_le_bytes());
        out.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes()); // extra
        out.extend_from_slice(&0_u16.to_le_bytes()); // comment
        out.extend_from_slice(&0_u16.to_le_bytes()); // disk
        out.extend_from_slice(&0_u16.to_le_bytes()); // internal attr
        out.extend_from_slice(&e.external_attr.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&e.name);
    }
    let cd_size = out.len() as u32 - cd_start;
    let count = declared_count.unwrap_or(entries.len().min(0xFFFF) as u16);
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_start.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out
}

fn crc_table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    table
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let table = crc_table();
    let mut c = 0xFFFF_FFFF_u32;
    for &b in bytes {
        c = table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn crc32_zeros(n: usize) -> u32 {
    let table = crc_table();
    let mut c = 0xFFFF_FFFF_u32;
    for _ in 0..n {
        c = table[(c & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

/// LSB-first bit writer of a deflate stream.
struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            acc: 0,
            n: 0,
        }
    }
    fn put(&mut self, value: u32, bits: u32) {
        self.acc |= value << self.n;
        self.n += bits;
        while self.n >= 8 {
            self.out.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// A Huffman code, most significant bit first.
    fn code(&mut self, code: u32, bits: u32) {
        let mut rev = 0;
        for i in 0..bits {
            rev |= ((code >> i) & 1) << (bits - 1 - i);
        }
        self.put(rev, bits);
    }
    fn literal(&mut self, byte: u8) {
        let b = u32::from(byte);
        if b < 144 {
            self.code(0x30 + b, 8);
        } else {
            self.code(0x190 + b - 144, 9);
        }
    }
    fn finish(mut self) -> Vec<u8> {
        self.code(0, 7); // end of block (256)
        if self.n > 0 {
            self.out.push((self.acc & 0xFF) as u8);
        }
        self.out
    }
}

fn deflate_zeros(n: usize) -> Vec<u8> {
    let mut bits = Bits::new();
    bits.put(1, 1); // final
    bits.put(1, 2); // fixed Huffman
    if n == 0 {
        return bits.finish();
    }
    bits.literal(0);
    let mut left = n - 1;
    while left >= 258 {
        bits.code(0xC5, 8); // length code 285 = 258
        bits.code(0, 5); // distance 1
        left -= 258;
    }
    for _ in 0..left {
        bits.literal(0);
    }
    bits.finish()
}

fn deflate_literals(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Bits::new();
    bits.put(1, 1);
    bits.put(1, 2);
    for &b in bytes {
        bits.literal(b);
    }
    bits.finish()
}
