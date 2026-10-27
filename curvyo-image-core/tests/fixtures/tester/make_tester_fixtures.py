#!/usr/bin/env python3
"""Builds the tester's extra fixtures of `specs/0058-bitmap-import` in this folder.

Every file is generated here (no third-party pictures; see SOURCES.md). Needs Pillow (MIT-CMU)
with JPEG and WebP support. Run from anywhere: `python3 -I make_tester_fixtures.py`.

- jpeg_cmyk_inverted.jpg: a CMYK JPEG with the Adobe marker, transform 0 (plain CMYK, stored
  inverted as Photoshop writes it), 16 x 16 in four 8 x 8 quadrants (criterion 7).
- png_ztxt_bomb.png, png_iccp_bomb.png: an 8 x 6 PNG with a zTXt / iCCP chunk that inflates to
  256 MiB (criteria 7, 20: metadata is read past, memory stays bounded).
- png_zeros_7000x7000.png: a valid 49 Mpx RGB picture of zeros in about 150 KB (criterion 19:
  under the header ceiling, decoded within the memory cap).
- jpeg_progressive_scan_bomb.jpg: a progressive 4000 x 3000 grey JPEG whose last AC scan
  (26 bytes, 187,500 blocks) repeats 5,000 times (criterion 20: a nested or recursive structure fails or decodes in bounded time).
- jpeg_thumbnail_truncated.jpg: a JPEG with an EXIF thumbnail (its own SOS and EOI inside APP1),
  cut in the main image's scan data (criterion 21).
- webp_2000_frames.webp: an animation of 2,000 1 x 1 frames (criterion 6: the first frame,
  bounded time).
"""

import io
import os
import struct
import zlib

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))


def write(name, data):
    with open(os.path.join(HERE, name), "wb") as f:
        f.write(data)


def png_chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)


def png_chunks(data):
    at, out = 8, []
    while at < len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        out.append((data[at + 4 : at + 8], data[at + 8 : at + 8 + length]))
        at += 12 + length
    return out


def png_with(extra_before_idat):
    buf = io.BytesIO()
    Image.new("RGB", (8, 6), (10, 20, 30)).save(buf, "PNG")
    out = b"\x89PNG\r\n\x1a\n"
    for kind, data in png_chunks(buf.getvalue()):
        if kind == b"IDAT":
            out += extra_before_idat
        out += png_chunk(kind, data)
    return out


def big_zlib(size):
    compressor = zlib.compressobj(9)
    block = b"\x00" * (1 << 20)
    out = b""
    for _ in range(size >> 20):
        out += compressor.compress(block)
    return out + compressor.flush()


def jpeg_segments(data):
    """(marker, bytes) of each segment up to and including the scans; scan data stays with its SOS."""
    at, out = 2, []
    while at < len(data):
        marker = data[at + 1]
        if marker == 0xD9:
            out.append((marker, data[at : at + 2]))
            break
        (length,) = struct.unpack(">H", data[at + 2 : at + 4])
        end = at + 2 + length
        if marker == 0xDA:
            while not (data[end] == 0xFF and data[end + 1] not in (0x00,) and not 0xD0 <= data[end + 1] <= 0xD7):
                end += 1
        out.append((marker, data[at:end]))
        at = end
    return out


def main():
    cmyk = Image.new("CMYK", (16, 16))
    quadrants = [(0, 255, 255, 0), (255, 0, 255, 0), (255, 255, 0, 0), (0, 0, 0, 128)]
    for i, value in enumerate(quadrants):
        cmyk.paste(value, ((i % 2) * 8, (i // 2) * 8, (i % 2) * 8 + 8, (i // 2) * 8 + 8))
    buf = io.BytesIO()
    cmyk.save(buf, "JPEG", quality=100, subsampling=0)
    data = buf.getvalue()
    app14 = data.find(b"\xff\xeeAdobe"[:2] + b"\x00\x0eAdobe")
    assert app14 > 0 and data[app14 + 15] == 0, "Adobe marker with transform 0 expected"
    write("jpeg_cmyk_inverted.jpg", data)

    huge = big_zlib(256 << 20)
    write("png_ztxt_bomb.png", png_with(png_chunk(b"zTXt", b"Comment\x00\x00" + huge)))
    write("png_iccp_bomb.png", png_with(png_chunk(b"iCCP", b"profile\x00\x00" + huge)))

    side = 7000
    row = b"\x00" + b"\x00" * (side * 3)
    compressor = zlib.compressobj(9)
    idat = b"".join(compressor.compress(row) for _ in range(side)) + compressor.flush()
    ihdr = struct.pack(">IIBBBBB", side, side, 8, 2, 0, 0, 0)
    write("png_zeros_7000x7000.png", b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", idat) + png_chunk(b"IEND", b""))

    buf = io.BytesIO()
    Image.new("L", (4000, 3000), 128).save(buf, "JPEG", progressive=True, quality=90)
    whole = buf.getvalue()
    last_scan = [seg for marker, seg in jpeg_segments(whole) if marker == 0xDA][-1]
    assert len(last_scan) < 64, "an AC scan of a flat picture is a few end-of-band runs"
    write("jpeg_progressive_scan_bomb.jpg", whole[:-2] + last_scan * 5000 + b"\xff\xd9")

    buf = io.BytesIO()
    Image.new("RGB", (16, 16), (200, 0, 0)).save(buf, "JPEG", quality=90)
    thumb = buf.getvalue()
    tiff = b"MM\x00\x2a\x00\x00\x00\x08" + struct.pack(">H", 0) + struct.pack(">I", 0)
    app1_payload = b"Exif\x00\x00" + tiff + thumb
    app1 = b"\xff\xe1" + struct.pack(">H", len(app1_payload) + 2) + app1_payload
    buf = io.BytesIO()
    Image.new("RGB", (64, 64), (0, 100, 200)).save(buf, "JPEG", quality=95)
    main_jpeg = buf.getvalue()
    whole = main_jpeg[:2] + app1 + main_jpeg[2:]
    scan = whole.rindex(b"\xff\xda")
    write("jpeg_thumbnail_truncated.jpg", whole[: scan + (len(whole) - scan) // 2])

    buf = io.BytesIO()
    Image.new("RGBA", (1, 1), (255, 0, 0, 255)).save(buf, "WEBP", lossless=True)
    data = buf.getvalue()
    vp8l_at = data.index(b"VP8L")
    (length,) = struct.unpack("<I", data[vp8l_at + 4 : vp8l_at + 8])
    vp8l = data[vp8l_at : vp8l_at + 8 + length + (length & 1)]

    def chunk(kind, payload):
        return kind + struct.pack("<I", len(payload)) + payload + (b"\x00" if len(payload) & 1 else b"")

    def u24(v):
        return struct.pack("<I", v)[:3]

    vp8x = chunk(b"VP8X", bytes([0x12, 0, 0, 0]) + u24(0) + u24(0))
    anim = chunk(b"ANIM", struct.pack("<IH", 0, 0))
    frame = chunk(b"ANMF", u24(0) + u24(0) + u24(0) + u24(0) + u24(10) + b"\x00" + vp8l)
    body = b"WEBP" + vp8x + anim + frame * 2000
    write("webp_2000_frames.webp", b"RIFF" + struct.pack("<I", len(body)) + body)


if __name__ == "__main__":
    main()
