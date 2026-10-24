#!/usr/bin/env python3
"""Generates the glyph table's Unicode data (specs/0082-glyph-table, adrs.md decision 2).

Reads UnicodeData.txt, Blocks.txt and NameAliases.txt of Unicode 17.0.0 from the folder given as
the first argument, refuses files whose SHA-256 differs from the recorded one, and writes

- frontend/public/unicode/names-17.0.0.bin (the character names, decoded by
  curvyo-text-core/src/unicode_names.rs) and
- curvyo-text-core/src/unicode_blocks.rs (the block table).

Python 3 standard library only. Run from anywhere:

    python3 -I tools/unicode-data/generate.py <folder with the three files>
"""

import collections
import hashlib
import pathlib
import re
import sys

VERSION = (17, 0, 0)
SOURCES = {
    "UnicodeData.txt": "2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c",
    "Blocks.txt": "c0edefaf1a19771e830a82735472716af6bf3c3975f6c2a23ffbe2580fbbcb15",
    "NameAliases.txt": "793f6f1e4d15fd90f05ae66460191dc4d75d1fea90136a25f30dd6a4cb950eac",
}
MAGIC = b"CVYN"
FORMAT = 1
# The <..., First> ranges of UnicodeData.txt whose names are the prefix and the hex code point
# (Unicode's name rule NR2). Hangul syllables (NR1) are computed by the decoder itself.
RANGE_PREFIXES = {
    "CJK Ideograph": "CJK UNIFIED IDEOGRAPH-",
    "Tangut Ideograph": "TANGUT IDEOGRAPH-",
}
HANGUL = (0xAC00, 0xD7A3)

REPO = pathlib.Path(__file__).resolve().parents[2]


def read(folder, name):
    data = (folder / name).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != SOURCES[name]:
        sys.exit(f"{name}: SHA-256 {digest}, expected {SOURCES[name]}")
    return data.decode("utf-8")


def varint(value):
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def text(value):
    raw = value.encode("ascii")
    return varint(len(raw)) + raw


def parse_names(source):
    """Explicit names by code point, and the NR2 ranges as (start, end, prefix)."""
    names = {}
    ranges = []
    first = None
    for line in source.splitlines():
        fields = line.split(";")
        code, name = int(fields[0], 16), fields[1]
        if name.endswith(", First>"):
            first = (code, name[1:-8])
            continue
        if name.endswith(", Last>"):
            start, label = first
            for key, prefix in RANGE_PREFIXES.items():
                if label.startswith(key):
                    ranges.append((start, code, prefix))
            continue
        if name.startswith("<"):
            continue
        names[code] = name
    # Runs of explicit names that follow NR2 (CJK compatibility ideographs, Nushu, Khitan,
    # Egyptian hieroglyphs ...) become ranges too.
    pattern = re.compile(r"^(.*-)([0-9A-F]{4,6})$")
    run = None
    for code in sorted(names):
        match = pattern.match(names[code])
        prefix = match.group(1) if match and int(match.group(2), 16) == code else None
        if prefix and run and run[2] == prefix and run[1] == code - 1:
            run[1] = code
            continue
        if run and run[1] - run[0] >= 15:
            ranges.append(tuple(run))
        run = [code, code, prefix] if prefix else None
    if run and run[1] - run[0] >= 15:
        ranges.append(tuple(run))
    for start, end, _ in ranges:
        for code in range(start, end + 1):
            names.pop(code, None)
    ranges.sort()
    return names, ranges


def parse_abbreviations(source):
    found = {}
    for line in source.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        code, alias, kind = line.split(";")
        if kind == "abbreviation":
            found[int(code, 16)] = alias  # the last one listed
    return found


def names_file(names, ranges, abbreviations):
    counts = collections.Counter(word for name in names.values() for word in name.split(" "))
    words = sorted(counts, key=lambda w: (-counts[w], w))
    index = {word: i for i, word in enumerate(words)}
    out = bytearray(MAGIC)
    out += bytes([FORMAT, *VERSION])
    out += varint(len(words))
    for word in words:
        out += text(word)
    out += varint(len(ranges))
    for start, end, prefix in ranges:
        out += varint(start) + varint(end - start) + text(prefix)
    out += varint(len(names))
    last = 0
    for code in sorted(names):
        parts = names[code].split(" ")
        out += varint(code - last) + varint(len(parts))
        for part in parts:
            out += varint(index[part])
        last = code
    out += varint(len(abbreviations))
    last = 0
    for code in sorted(abbreviations):
        out += varint(code - last) + text(abbreviations[code])
        last = code
    return bytes(out)


def hex_literal(code):
    digits = f"{code:X}"
    if len(digits) > 4:
        digits = digits[:-4] + "_" + digits[-4:]
    return "0x" + digits


def blocks_module(source):
    rows = []
    for line in source.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        span, name = line.split(";")
        start, end = span.split("..")
        rows.append((int(start, 16), int(end, 16), name.strip()))
    version = ".".join(map(str, VERSION))
    body = "\n".join(
        f'    ({hex_literal(start)}, {hex_literal(end)}, "{name}"),' for start, end, name in rows
    )
    return f"""//! The Unicode {version} blocks (`Blocks.txt`) for the glyph table's groups
//! (`specs/0082-glyph-table` criterion 6, `adrs.md` decision 2). Generated by
//! `tools/unicode-data/generate.py`; do not edit.

/// The Unicode version of [`BLOCKS`] and of the shipped character names.
pub const UNICODE_VERSION: &str = "{version}";

/// Every block: first and last code point and its name, in code point order.
#[rustfmt::skip]
pub const BLOCKS: &[(u32, u32, &str)] = &[
{body}
];

/// The name of the block `c` lies in; `None` for a code point in no block.
#[must_use]
pub fn block_of(c: char) -> Option<&'static str> {{
    let code = u32::from(c);
    let at = BLOCKS.partition_point(|(_, end, _)| *end < code);
    BLOCKS
        .get(at)
        .filter(|(start, _, _)| *start <= code)
        .map(|(_, _, name)| *name)
}}
"""


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: generate.py <folder with UnicodeData.txt, Blocks.txt, NameAliases.txt>")
    folder = pathlib.Path(sys.argv[1])
    names, ranges = parse_names(read(folder, "UnicodeData.txt"))
    for code in range(HANGUL[0], HANGUL[1] + 1):
        names.pop(code, None)
    abbreviations = parse_abbreviations(read(folder, "NameAliases.txt"))
    version = ".".join(map(str, VERSION))
    binary = names_file(names, ranges, abbreviations)
    (REPO / f"frontend/public/unicode/names-{version}.bin").write_bytes(binary)
    (REPO / "curvyo-text-core/src/unicode_blocks.rs").write_text(blocks_module(read(folder, "Blocks.txt")))
    print(f"{len(names)} names, {len(ranges)} ranges, {len(abbreviations)} abbreviations, {len(binary)} bytes")


main()
