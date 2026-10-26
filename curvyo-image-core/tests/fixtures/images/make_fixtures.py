#!/usr/bin/env python3
"""Builds the image fixtures of `specs/0058-bitmap-import` ("Fixtures") in this folder.

Needs ImageMagick 7 (`magick`, with libpng, libjpeg-turbo and libwebp). Run from anywhere:
`python3 -I make_fixtures.py`. The EXIF blocks, the bombs and the damaged files are written byte by
byte here, so the expectations in `expected.txt` do not depend on how a tool writes metadata.
Rerunning it rewrites every file; lossy files may then differ in a few values, so
`expected.txt` compares JPEG and lossy WebP pixels with a tolerance.
"""

import os
import struct
import subprocess
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))


def path(name):
    return os.path.join(HERE, name)


def magick(*args):
    subprocess.run(["magick", *args], check=True)


def write(name, data):
    with open(path(name), "wb") as f:
        f.write(data)


def read(name):
    with open(path(name), "rb") as f:
        return f.read()


# --- EXIF (a TIFF block with IFD0: Orientation, optionally X/YResolution) ---------------------

def exif_tiff(orientation=None, resolution=None):
    entries = []
    extra = b""
    # Offsets: header 8, count 2, n entries * 12, next IFD 4, then the rationals.
    tags = []
    if orientation is not None:
        tags.append((0x0112, 3, 1, struct.pack("<HH", orientation, 0)))
    if resolution is not None:
        tags.append((0x011A, 5, 1, None))
        tags.append((0x011B, 5, 1, None))
        tags.append((0x0128, 3, 1, struct.pack("<HH", 2, 0)))
    tags.sort(key=lambda t: t[0])
    base = 8 + 2 + 12 * len(tags) + 4
    for tag, typ, count, value in tags:
        if value is None:
            value = struct.pack("<I", base + len(extra))
            extra += struct.pack("<II", resolution, 1)
        entries.append(struct.pack("<HHI", tag, typ, count) + value)
    return b"II*\x00" + struct.pack("<I", 8) + struct.pack("<H", len(tags)) + b"".join(entries) + b"\x00\x00\x00\x00" + extra


def jpeg_with_exif(jpeg, tiff):
    payload = b"Exif\x00\x00" + tiff
    app1 = b"\xff\xe1" + struct.pack(">H", len(payload) + 2) + payload
    return jpeg[:2] + app1 + jpeg[2:]


def png_chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)


def png_insert_before_idat(png, chunk):
    i = png.index(b"IDAT") - 4
    return png[:i] + chunk + png[i:]


def png_raw(width, height, rows, colour_type=2, depth=8, plte=None, trns=None):
    ihdr = struct.pack(">IIBBBBB", width, height, depth, colour_type, 0, 0, 0)
    raw = b"".join(b"\x00" + row for row in rows)
    out = b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", ihdr)
    if plte is not None:
        out += png_chunk(b"PLTE", plte)
    if trns is not None:
        out += png_chunk(b"tRNS", trns)
    return out + png_chunk(b"IDAT", zlib.compress(raw)) + png_chunk(b"IEND", b"")


def webp_extended_with_exif(webp, width, height, tiff):
    # Turns a simple (VP8 or VP8L) WebP into an extended one carrying an EXIF chunk.
    assert webp[:4] == b"RIFF" and webp[8:12] == b"WEBP"
    image = webp[12:]
    flags = 0x08  # EXIF
    vp8x = b"VP8X" + struct.pack("<I", 10) + bytes([flags, 0, 0, 0]) + (width - 1).to_bytes(3, "little") + (height - 1).to_bytes(3, "little")
    exif = tiff
    exif_chunk = b"EXIF" + struct.pack("<I", len(exif)) + exif + (b"\x00" if len(exif) % 2 else b"")
    body = b"WEBP" + vp8x + image + exif_chunk
    return b"RIFF" + struct.pack("<I", len(body)) + body


def marker_source(name, width=40, height=20):
    # A grey picture with a red block in the stored top-left corner.
    magick("-size", f"{width}x{height}", "xc:#808080", "-fill", "#FF0000", "-draw", "rectangle 0,0 7,7", path(name))


def main():
    # Valid PNG.
    magick("-size", "8x6", "gradient:#000000-#FFFFFF", "-define", "png:color-type=2", "-strip", path("png_rgb_8.png"))
    write("png_rgba_8.png", png_raw(3, 1, [bytes([255, 0, 0, 0, 0, 255, 0, 128, 0, 0, 255, 255])], colour_type=6))
    rows16 = [b"".join(struct.pack(">H", (x * 4 + y) * 4369) for x in range(4)) for y in range(4)]
    write("png_gray_16.png", png_raw(4, 4, rows16, colour_type=0, depth=16))
    write("png_indexed_trns.png", png_raw(2, 1, [bytes([0, 1])], colour_type=3, plte=bytes([0, 0, 255, 255, 255, 0]), trns=bytes([0])))
    base = png_raw(30, 20, [bytes([10, 20, 30] * 30)] * 20)
    write("png_phys_300dpi.png", png_insert_before_idat(base, png_chunk(b"pHYs", struct.pack(">IIB", 11811, 11811, 1))))
    write("png_phys_unit_unknown.png", png_insert_before_idat(base, png_chunk(b"pHYs", struct.pack(">IIB", 1, 2, 0))))
    write("png_phys_150x300dpi.png", png_insert_before_idat(base, png_chunk(b"pHYs", struct.pack(">IIB", 5906, 11811, 1))))
    magick(path("png_rgb_8.png"), "-interlace", "PNG", path("png_interlaced.png"))
    icc = b"bogus" + b"\x00" + b"\x00" + zlib.compress(b"\x00" * 200)
    write("png_icc_ignored.png", png_insert_before_idat(read("png_rgb_8.png"), png_chunk(b"iCCP", icc)))
    marker_source("_marker.png")
    write("png_exif_orientation_6.png", png_insert_before_idat(read("_marker.png"), png_chunk(b"eXIf", exif_tiff(orientation=6))))

    # Valid JPEG.
    magick("-size", "60x40", "xc:#336699", "-units", "PixelsPerInch", "-density", "300", "-quality", "95", path("jpeg_baseline_jfif_300dpi.jpg"))
    magick("-size", "60x40", "gradient:#FF0000-#0000FF", "-interlace", "JPEG", "-quality", "95", path("jpeg_progressive.jpg"))
    magick("-size", "16x16", "xc:#777777", "-colorspace", "Gray", "-quality", "95", path("jpeg_gray.jpg"))
    magick("-size", "16x16", "xc:#00FF00", "-colorspace", "CMYK", "-quality", "95", path("jpeg_cmyk_adobe.jpg"))
    magick(path("_marker.png"), "-quality", "95", "-strip", path("_marker.jpg"))
    for orientation in range(1, 9):
        write(f"jpeg_exif_orientation_{orientation}.jpg", jpeg_with_exif(read("_marker.jpg"), exif_tiff(orientation=orientation)))
    magick("-size", "48x48", "xc:#808080", "-units", "Undefined", "-density", "1", "-quality", "90", "-strip", path("_plain.jpg"))
    write("jpeg_exif_dpi_only.jpg", jpeg_with_exif(read("_plain.jpg"), exif_tiff(resolution=300)))
    write("png_renamed.jpg", read("png_rgb_8.png"))

    # Valid WebP.
    magick("-size", "40x30", "xc:#C08040", "-quality", "80", path("webp_lossy.webp"))
    magick(path("png_rgba_8.png"), "-define", "webp:lossless=true", path("webp_lossless_alpha.webp"))
    magick("-size", "20x10", "xc:#FF0000", "-size", "20x10", "xc:#0000FF", "-define", "webp:lossless=true", "-loop", "0", path("webp_animated.webp"))
    magick(path("_marker.png"), "-define", "webp:lossless=true", "-strip", path("_marker.webp"))
    write("webp_exif_orientation_6.webp", webp_extended_with_exif(read("_marker.webp"), 40, 20, exif_tiff(orientation=6)))

    # Damaged and hostile.
    write("empty.png", b"")
    write("text_renamed.png", b"This is a text file, not a picture.\n")
    good = read("png_rgb_8.png")
    write("png_truncated.png", good[: len(good) // 2])
    i = good.index(b"IDAT")
    bad = bytearray(good)
    bad[i + 4] ^= 0xFF  # first byte of the compressed data; the CRC no longer matches
    write("png_bad_crc.png", bytes(bad))
    bomb_ihdr = struct.pack(">IIBBBBB", 60000, 60000, 8, 2, 0, 0, 0)
    write("png_bomb_60000x60000.png", b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", bomb_ihdr) + png_chunk(b"IDAT", zlib.compress(b"\x00" * 1024)) + png_chunk(b"IEND", b""))
    zero_ihdr = struct.pack(">IIBBBBB", 0, 10, 8, 2, 0, 0, 0)
    write("png_zero_width.png", b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", zero_ihdr) + png_chunk(b"IDAT", zlib.compress(b"")) + png_chunk(b"IEND", b""))
    write("png_bad_palette_index.png", png_raw(2, 1, [bytes([0, 7])], colour_type=3, plte=bytes([0, 0, 255, 255, 255, 0])))
    magick("-size", "64x64", "gradient:#FF8000-#0080FF", "-quality", "95", "-strip", path("_long.jpg"))
    long_jpeg = read("_long.jpg")
    scan = long_jpeg.index(b"\xff\xda")
    write("jpeg_truncated.jpg", long_jpeg[: scan + (len(long_jpeg) - scan) // 2])
    jpeg = read("jpeg_baseline_jfif_300dpi.jpg")
    com = b"\xff\xfe" + struct.pack(">H", 4) + b"ab"
    write("jpeg_endless_markers.jpg", b"\xff\xd8" + com * 20000)
    sof = jpeg.index(b"\xff\xc0")
    big = bytearray(jpeg)
    big[sof + 5 : sof + 9] = struct.pack(">HH", 65535, 65535)
    write("jpeg_dimensions_65535x65535.jpg", bytes(big))
    lossy = read("webp_lossy.webp")
    over = bytearray(lossy)
    over[16:20] = struct.pack("<I", 0xFFFFFFF0)  # the VP8 chunk claims almost 4 GiB
    write("webp_chunk_overflow.webp", bytes(over))
    write("webp_truncated.webp", lossy[: len(lossy) // 2])
    write("polyglot_png_zip.png", good + b"PK\x03\x04" + b"\x00" * 26 + b"zip tail")
    write("pdf_renamed.png", b"%PDF-1.7\n1 0 obj << >> endobj\ntrailer << >>\n%%EOF\n")
    write("svg_renamed.png", b'<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>')

    for name in ("_marker.png", "_marker.jpg", "_marker.webp", "_plain.jpg", "_long.jpg"):
        os.remove(path(name))


if __name__ == "__main__":
    main()
