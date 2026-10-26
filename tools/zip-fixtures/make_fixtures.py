#!/usr/bin/env python3
"""Builds the zip archive fixtures of specs/0083-font-info-and-zip criterion 16.

Writes curvyo-text-core/tests/fixtures/fonts/zip/*.zip. The fonts inside are unmodified copies of
fonts Curvyo ships (frontend/public/fonts), each with its licence file. The hostile archives are
made by patching the bytes Python's zipfile writes, so every bomb is reproducible and small.

Python 3 standard library only. Run from anywhere:

    python3 -I tools/zip-fixtures/make_fixtures.py
"""

import io
import pathlib
import struct
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
FONTS = ROOT / "frontend" / "public" / "fonts"
OUT = ROOT / "curvyo-text-core" / "tests" / "fixtures" / "fonts" / "zip"
DATE = (2026, 10, 26, 12, 0, 0)
MIB = 1024 * 1024

HERSHEY = (FONTS / "hershey-sans-1" / "HersheySans1.svg").read_bytes()
HERSHEY_NOTICE = (FONTS / "hershey-sans-1" / "NOTICE.txt").read_bytes()


def info(name, method=zipfile.ZIP_DEFLATED, mode=0o100644):
    entry = zipfile.ZipInfo(name, DATE)
    entry.compress_type = method
    entry.create_system = 3
    entry.external_attr = mode << 16
    return entry


def build(entries):
    """A zip of (name, bytes) or (ZipInfo, bytes) pairs."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for name, data in entries:
            archive.writestr(name if isinstance(name, zipfile.ZipInfo) else info(name), data)
    return bytearray(buffer.getvalue())


def zeros(name, size):
    """An entry of `size` zero bytes, deflated as it is written (no `size` bytes in memory)."""

    def write(archive):
        with archive.open(info(name), "w", force_zip64=size > 0x7FFF0000) as out:
            chunk = bytes(MIB)
            for _ in range(size // MIB):
                out.write(chunk)

    return write


def build_streamed(writers):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for write in writers:
            write(archive)
    return bytearray(buffer.getvalue())


def headers(data):
    """Offsets of the local and central headers, in central directory order (read from the end
    record, never by searching for signatures)."""
    eocd_at = data.rfind(b"PK\x05\x06")
    count = struct.unpack_from("<H", data, eocd_at + 10)[0]
    at = struct.unpack_from("<I", data, eocd_at + 16)[0]
    local, central = [], []
    for _ in range(count):
        name, extra, comment = struct.unpack_from("<HHH", data, at + 28)
        central.append(at)
        local.append(struct.unpack_from("<I", data, at + 42)[0])
        at += 46 + name + extra + comment
    return local, central


def patch_u16(data, at, value):
    data[at : at + 2] = struct.pack("<H", value)


def patch_u32(data, at, value):
    data[at : at + 4] = struct.pack("<I", value)


def first_entry(data):
    local, central = headers(data)
    return local[0], central[0]


def plain():
    return build(
        [
            ("HersheySans1.svg", HERSHEY),
            ("NOTICE.txt", HERSHEY_NOTICE),
            ("README.md", b"# Hershey Sans 1-stroke\n\nA test archive.\n"),
            ("preview.png", b"\x89PNG not a real image"),
        ]
    )


def nested_folders():
    return build(
        [
            ("Pack/fonts/single-line/svg/HersheySans1.svg", HERSHEY),
            ("Pack/docs/NOTICE.txt", HERSHEY_NOTICE),
        ]
    )


def macosx():
    return build(
        [
            ("HersheySans1.svg", HERSHEY),
            ("__MACOSX/._HersheySans1.svg", b"\x00\x05\x16\x07resource fork"),
            (".DS_Store", b"\x00\x00\x00\x01Bud1"),
            ("._notes.txt", b"resource fork"),
        ]
    )


def same_ofl():
    allure = FONTS / "ems-allure"
    readability = FONTS / "ems-readability"
    return build(
        [
            ("EMS Allure/EMSAllure.svg", (allure / "EMSAllure.svg").read_bytes()),
            ("EMS Allure/OFL.txt", (allure / "OFL.txt").read_bytes()),
            ("EMS Readability/EMSReadability.svg", (readability / "EMSReadability.svg").read_bytes()),
            ("EMS Readability/OFL.txt", (readability / "OFL.txt").read_bytes()),
        ]
    )


def dotdot():
    return build([("../evil.ttf", b"evil"), ("fonts/../../evil.txt", b"evil"), ("HersheySans1.svg", HERSHEY)])


def absolute():
    return build(
        [
            ("/etc/evil.ttf", b"evil"),
            ("C:/Windows/evil.ttf", b"evil"),
            ("\\\\host\\share\\evil.ttf", b"evil"),
            ("HersheySans1.svg", HERSHEY),
        ]
    )


def symlink():
    return build([(info("link.ttf", mode=0o120777), b"/etc/passwd"), (info("fifo.txt", mode=0o010644), b""), ("NOTICE.txt", HERSHEY_NOTICE)])


def nested_bomb():
    """42.zip-style: archives of archives, 16 per layer, three layers; never opened."""
    leaf = build_streamed([zeros("0.ttf", 16 * MIB)])
    layer = leaf
    for _ in range(3):
        layer = build([(f"{n:02}.zip", bytes(layer)) for n in range(16)])
    return layer


def overlap_bomb():
    """One local entry of zeros that every central entry points at (Fifield's overlapping
    bomb): 200 names of 32 MiB each from one data block."""
    base = build_streamed([zeros("a.ttf", 32 * MIB)])
    local, central = headers(base)
    cd_at = central[0]
    eocd_at = base.rfind(b"PK\x05\x06")
    record = bytes(base[cd_at:eocd_at])
    name_len = struct.unpack_from("<H", record, 28)[0]
    out = bytearray(base[:cd_at])
    count = 200
    for n in range(count):
        name = f"{n:03}.ttf".encode()
        entry = bytearray(record[:46]) + name + record[46 + name_len :]
        patch_u16(entry, 28, len(name))
        out += entry
    cd_size = len(out) - cd_at
    eocd = bytearray(base[eocd_at:])
    patch_u16(eocd, 8, count)
    patch_u16(eocd, 10, count)
    patch_u32(eocd, 12, cd_size)
    patch_u32(eocd, 16, cd_at)
    return out + eocd


def ratio_bomb():
    """One font entry of 300 MiB zeros: over its own 64 MiB limit and the 256 MiB total."""
    return build_streamed([zeros("big.ttf", 300 * MIB)])


def total_bomb():
    """Five font entries of 60 MiB zeros: each under 64 MiB, together over 256 MiB."""
    return build_streamed([zeros(f"{n}.ttf", 60 * MIB) for n in range(5)])


def wrong_crc():
    data = build([("HersheySans1.svg", HERSHEY), ("NOTICE.txt", HERSHEY_NOTICE)])
    local, central = first_entry(data)
    patch_u32(data, local + 14, 0x12345678)
    patch_u32(data, central + 16, 0x12345678)
    return data


def short_size():
    """The first entry declares 100 bytes fewer than its data holds."""
    data = build([(info("HersheySans1.svg", zipfile.ZIP_STORED), HERSHEY), ("NOTICE.txt", HERSHEY_NOTICE)])
    local, central = first_entry(data)
    for at in (local + 22, central + 24):
        size = struct.unpack_from("<I", data, at)[0]
        patch_u32(data, at, size - 100)
    return data


def encrypted():
    """The encryption flag set on the first entry (its data stays plain)."""
    data = build([("HersheySans1.svg", HERSHEY), ("NOTICE.txt", HERSHEY_NOTICE)])
    local, central = first_entry(data)
    patch_u16(data, local + 6, struct.unpack_from("<H", data, local + 6)[0] | 1)
    patch_u16(data, central + 8, struct.unpack_from("<H", data, central + 8)[0] | 1)
    return data


def many_entries():
    return build([(info(f"{n}", zipfile.ZIP_STORED), b"") for n in range(10_001)])


def non_utf8_name():
    """`caf\\xe9-1.txt` and `caf\\xe9-2.txt`: with the UTF-8 flag (shown `caf_-1.txt`) and without (CP437)."""
    data = build([("cafX-1.txt", b"flagged"), ("cafX-2.txt", b"cp437"), ("HersheySans1.svg", HERSHEY)])
    local, central = headers(data)
    for header, flags_at, name_at in ((local[0], 6, 30), (central[0], 8, 46)):
        patch_u16(data, header + flags_at, struct.unpack_from("<H", data, header + flags_at)[0] | 0x0800)
        data[header + name_at + 3] = 0xE9
    for header, name_at in ((local[1], 30), (central[1], 46)):
        data[header + name_at + 3] = 0xE9
    return data


def no_font():
    return build([("README.txt", b"No fonts here.\n"), ("image.png", b"\x89PNG")])


FIXTURES = {
    "plain.zip": plain,
    "nested-folders.zip": nested_folders,
    "macosx.zip": macosx,
    "same-ofl.zip": same_ofl,
    "dotdot.zip": dotdot,
    "absolute.zip": absolute,
    "symlink.zip": symlink,
    "nested-bomb.zip": nested_bomb,
    "overlap-bomb.zip": overlap_bomb,
    "ratio-bomb.zip": ratio_bomb,
    "total-bomb.zip": total_bomb,
    "wrong-crc.zip": wrong_crc,
    "short-size.zip": short_size,
    "encrypted.zip": encrypted,
    "many-entries.zip": many_entries,
    "non-utf8-name.zip": non_utf8_name,
    "no-font.zip": no_font,
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, make in FIXTURES.items():
        data = make()
        (OUT / name).write_bytes(bytes(data))
        print(f"{name}: {len(data)} bytes")
    (OUT / "not-a-zip.zip").write_bytes(b"This is plain text, not a zip archive.\n")


if __name__ == "__main__":
    main()
