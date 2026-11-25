#!/usr/bin/env python3
"""Generates the search keyword file of the library browser (spec 0153, ADR 0087 decision 1).

Reads the pinned upstream files (Lucide `tags.json`, Tabler `icons.json`, MDI `meta.json`) from an
unpacked folder, checks each against the SHA-256 in `pins.toml`, folds and splits every keyword as
the matcher does (the table of `curvyo-library-core/src/fold_table.rs`), derives the related words
and writes format 1 (see README.md):

- `<out>/keywords.bin.gz` (shipped; gzip level 9, mtime 0, no file name),
- `<out>/keywords-LICENSES.txt` (shipped; name, version, licence and LICENSE text per source),
- `<related>` (not shipped; one line per head word, for review and readable diffs).

It never fetches (README.md has the download commands). A file with another checksum, a file that
is not JSON, a missing or empty name or a tag that is neither text nor an integer stops it with the
file and the key named, and nothing is written. Python 3.11+ standard library only:

    python3 -I tools/library-keywords/generate.py <upstream> [--pins F] [--out D] [--related F]
"""

import argparse
import gzip
import hashlib
import json
import pathlib
import re
import sys
import tomllib

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FOLD_TABLE = ROOT / "curvyo-library-core" / "src" / "fold_table.rs"

MAGIC = b"CVYK"
FORMAT = 1
# Related words (spec 0153 criterion 8, Proposal): K is related to N when at least MIN_ICONS
# distinct icons whose names contain N carry K, in at least MIN_PROJECTS projects or in at least
# MANY_ICONS icons; at most MAX_RELATED per head.
MIN_ICONS = 3
MIN_PROJECTS = 2
MANY_ICONS = 6
MAX_RELATED = 8
MIN_LETTERS = 3
STOP = {"icon", "outline", "filled", "solid", "alt", "variant"}
SEPARATORS = re.compile(r"[\s\-_.]+")
# Null tags read as the word "null" in this run (printed, so a version bump shows new ones).
NULL_TAGS = [0]


class Stop(Exception):
    """A damaged input: the message names the file and the key."""


# --- folding (as `curvyo-library-core/src/fold.rs`) ---------------------------------------------


def read_folds(path: pathlib.Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    pairs = re.findall(r"\('\\u\{([0-9a-f]+)\}', \"([^\"]*)\"\)", text)
    if not pairs:
        raise Stop(f"{path}: no folding table")
    return {chr(int(cp, 16)): folded for cp, folded in pairs}


def fold(text: str, folds: dict[str, str]) -> str:
    out = []
    for c in text:
        for lower in c.lower():
            if "\u0300" <= lower <= "\u036f":
                continue
            out.append(lower if lower.isascii() else folds.get(lower, lower))
    return "".join(out)


def words_of(text: str, folds: dict[str, str]) -> list[str]:
    return [w for w in SEPARATORS.split(fold(text, folds)) if w]


# --- reading the upstream files ------------------------------------------------------------------


def checked(upstream: pathlib.Path, rel: str, sha: str) -> bytes:
    path = upstream / rel
    try:
        data = path.read_bytes()
    except OSError as err:
        raise Stop(f"{rel}: cannot read ({err.strerror})") from None
    digest = hashlib.sha256(data).hexdigest()
    if digest != sha:
        raise Stop(f"{rel}: SHA-256 {digest} is not the pinned {sha}")
    return data


def parse(rel: str, data: bytes):
    try:
        return json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as err:
        raise Stop(f"{rel}: not JSON ({err})") from None


def tag_text(rel: str, key: str, value) -> str:
    # A JSON integer is its decimal text (Tabler `ambulance` 911), a JSON null the word "null"
    # (Tabler `number-0`, a YAML null upstream; Proposal); bool is not an integer here.
    if isinstance(value, str):
        return value
    if value is None:
        NULL_TAGS[0] += 1
        return "null"
    if isinstance(value, int) and not isinstance(value, bool):
        return str(value)
    raise Stop(f"{rel}: {key}: a tag is neither text, an integer nor null ({json.dumps(value)})")


def tag_list(rel: str, key: str, value) -> list[str]:
    if value is None:
        return []
    if not isinstance(value, list):
        raise Stop(f"{rel}: {key}: the tags are not a list")
    return [tag_text(rel, key, v) for v in value]


def name_of(rel: str, key: str, value) -> str:
    if not isinstance(value, str) or not value:
        raise Stop(f"{rel}: {key}: no name")
    return value


def read_lucide(rel: str, doc) -> dict:
    if not isinstance(doc, dict):
        raise Stop(f"{rel}: not an object of names")
    return {name_of(rel, repr(k), k): (tag_list(rel, k, v), [], 0) for k, v in doc.items()}


def read_tabler(rel: str, doc) -> dict:
    if not isinstance(doc, dict):
        raise Stop(f"{rel}: not an object of names")
    icons = {}
    for key, entry in doc.items():
        name = name_of(rel, repr(key), key)
        if not isinstance(entry, dict):
            raise Stop(f"{rel}: {key}: not an object")
        category = entry.get("category") or ""
        if not isinstance(category, str):
            raise Stop(f"{rel}: {key}: the category is not text")
        styles = entry.get("styles") or {}
        mask = 1 if isinstance(styles, dict) and "filled" in styles else 0
        icons[name] = (tag_list(rel, key, entry.get("tags")), [category] if category else [], mask)
    return icons


def read_mdi(rel: str, doc) -> dict:
    if not isinstance(doc, list):
        raise Stop(f"{rel}: not a list of icons")
    icons = {}
    for n, entry in enumerate(doc):
        if not isinstance(entry, dict):
            raise Stop(f"{rel}: entry {n}: not an object")
        name = name_of(rel, f"entry {n}", entry.get("name"))
        icons[name] = (tag_list(rel, name, entry.get("aliases")), tag_list(rel, name, entry.get("tags")), 0)
    return icons


READERS = {"lucide": read_lucide, "tabler": read_tabler, "mdi": read_mdi}


# --- the data ------------------------------------------------------------------------------------


def unique(words):
    seen = []
    for w in words:
        if w not in seen:
            seen.append(w)
    return seen


def derive_related(sources, folds):
    """{head: [(word, icons)]}, strongest first (criterion 8).

    A pair is kept when either word has the other among its MAX_RELATED strongest candidates,
    and then both list it (the symmetric union of the top 8), so a word may list more than 8;
    the matcher takes at most 8 per typed word. Heads and related words are letters only.
    """
    pair_icons: dict[tuple[str, str], set] = {}
    pair_projects: dict[tuple[str, str], set] = {}
    usable = lambda w: len(w) >= MIN_LETTERS and w.isalpha() and w not in STOP  # noqa: E731
    for s, source in enumerate(sources):
        for name, (keywords, _categories, _mask) in source["icons"].items():
            names = {w for w in words_of(name, folds) if usable(w)}
            for k in {w for w in keywords if usable(w)}:
                for n in names:
                    if k != n:
                        pair_icons.setdefault((k, n), set()).add((s, name))
                        pair_projects.setdefault((k, n), set()).add(s)
    strength: dict[tuple[str, str], int] = {}
    for pair, icons in pair_icons.items():
        if len(icons) >= MIN_ICONS and (len(pair_projects[pair]) >= MIN_PROJECTS or len(icons) >= MANY_ICONS):
            k, n = pair
            for a, b in ((n, k), (k, n)):
                strength[(a, b)] = max(strength.get((a, b), 0), len(icons))
    candidates: dict[str, list[tuple[str, int]]] = {}
    for (head, word), count in strength.items():
        candidates.setdefault(head, []).append((word, count))
    top = {h: {w for w, _ in sorted(ws, key=lambda wc: (-wc[1], wc[0]))[:MAX_RELATED]} for h, ws in candidates.items()}
    heads: dict[str, list[tuple[str, int]]] = {}
    for head, words in sorted(candidates.items()):
        kept = [(w, c) for w, c in words if w in top[head] or head in top.get(w, ())]
        if kept:
            heads[head] = sorted(kept, key=lambda wc: (-wc[1], wc[0]))
    return heads


def leb(n: int) -> bytes:
    out = bytearray()
    while True:
        byte = n & 0x7F
        n >>= 7
        if n:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def text(s: str) -> bytes:
    raw = s.encode("utf-8")
    return leb(len(raw)) + raw


def payload(sources, related) -> tuple[bytes, list[str]]:
    counts: dict[str, int] = {}
    for source in sources:
        for keywords, categories, _mask in source["icons"].values():
            for w in keywords + categories:
                counts[w] = counts.get(w, 0) + 1
    for head, words in related.items():
        for w in [head] + [w for w, _ in words]:
            counts[w] = counts.get(w, 0) + 1
    words = sorted(counts, key=lambda w: (-counts[w], w))
    index = {w: i for i, w in enumerate(words)}
    out = bytearray(MAGIC + bytes([FORMAT]))
    out += leb(len(sources))
    for s in sources:
        for field in ("prefix", "project", "package", "version", "licence", "url"):
            out += text(s[field])
        out += leb(len(s["variants"]))
        for v in s["variants"]:
            out += text(v)
    out += leb(len(words))
    for w in words:
        out += text(w)
    for s in sources:
        icons = s["icons"]
        out += leb(len(icons))
        before = b""
        for name in sorted(icons, key=lambda n: n.encode("utf-8")):
            raw = name.encode("utf-8")
            shared = 0
            while shared < min(len(raw), len(before)) and raw[shared] == before[shared]:
                shared += 1
            out += leb(shared) + leb(len(raw) - shared) + raw[shared:]
            before = raw
            keywords, categories, mask = icons[name]
            out += leb(mask)
            for group in (keywords, categories):
                ids = sorted(index[w] for w in group)
                out += leb(len(ids))
                for i in ids:
                    out += leb(i)
    out += leb(len(related))
    for head, rel in related.items():
        out += leb(index[head]) + leb(len(rel))
        for w, _count in rel:
            out += leb(index[w])
    return bytes(out), words


def previous_words(path: pathlib.Path) -> set[str] | None:
    """The word list of an earlier data file, for the gained and lost counts; None if none."""
    try:
        data = gzip.decompress(path.read_bytes())
    except (OSError, EOFError, gzip.BadGzipFile):
        return None
    at = 5

    def num():
        nonlocal at
        value, shift = 0, 0
        while True:
            byte = data[at]
            at += 1
            value |= (byte & 0x7F) << shift
            shift += 7
            if byte < 0x80:
                return value

    def string():
        nonlocal at
        length = num()
        at += length
        return data[at - length : at].decode("utf-8")

    try:
        if data[:5] != MAGIC + bytes([FORMAT]):
            return None
        for _ in range(num()):
            for _ in range(6):
                string()
            for _ in range(num()):
                string()
        return {string() for _ in range(num())}
    except (IndexError, UnicodeDecodeError):
        return None


def licences_text(sources, upstream: pathlib.Path) -> str:
    parts = [
        "Search keywords of the Curvyo library browser, generated by tools/library-keywords/generate.py",
        "from the files below (words folded and split, related words derived; spec 0153, ADR 0087).",
        "",
    ]
    for s in sources:
        parts += [
            "=" * 78,
            f"{s['project']} {s['version']} (npm {s['package']}, file {s['file'].split('/')[-1]})",
            f"Licence of the file: {s['licence']}",
            f"Project: {s['url']}",
            "=" * 78,
            "",
            s["licence_text"].rstrip("\n"),
            "",
        ]
    return "\n".join(parts) + "\n"


def related_text(related, pins: pathlib.Path) -> str:
    lines = [
        f"# Related words of the library search, generated by tools/library-keywords/generate.py from {pins.name}.",
        "# One line per head word: its related words, strongest first (icon count). Not shipped.",
    ]
    for head, words in related.items():
        lines.append(f"{head}: {', '.join(f'{w} ({c})' for w, c in words)}")
    return "\n".join(lines) + "\n"


def build(upstream: pathlib.Path, pins_path: pathlib.Path):
    try:
        pins = tomllib.loads(pins_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise Stop(f"{pins_path.name}: {err}") from None
    folds = read_folds(FOLD_TABLE)
    NULL_TAGS[0] = 0
    sources = []
    for pin in pins.get("source", []):
        reader = READERS.get(pin.get("reader", ""))
        if reader is None:
            raise Stop(f"{pins_path.name}: {pin.get('prefix')}: unknown reader {pin.get('reader')!r}")
        rel = pin["file"]
        doc = parse(rel, checked(upstream, rel, pin["sha256"]))
        # Verbatim apart from the line ends (MDI's LICENSE has CRLF; the repository keeps LF).
        licence_text = checked(upstream, pin["licence_file"], pin["licence_sha256"]).decode("utf-8")
        licence_text = licence_text.replace("\r\n", "\n")
        icons = {}
        for name, (keywords, categories, mask) in reader(rel, doc).items():
            kws = unique(w for t in keywords for w in words_of(t, folds))
            cats = unique(w for t in categories for w in words_of(t, folds))
            if kws or cats:
                icons[name] = (kws, cats, mask)
        sources.append({**pin, "icons": icons, "licence_text": licence_text, "variants": pin.get("variants", [])})
    related = derive_related(sources, folds)
    data, words = payload(sources, related)
    return sources, related, data, words


def main() -> int:
    args = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    args.add_argument("upstream", type=pathlib.Path, help="folder with the unpacked packages (README.md)")
    args.add_argument("--pins", type=pathlib.Path, default=HERE / "pins.toml")
    args.add_argument("--out", type=pathlib.Path, default=ROOT / "frontend" / "public" / "library")
    args.add_argument("--related", type=pathlib.Path, default=HERE / "related-words.txt")
    opts = args.parse_args()
    try:
        sources, related, data, words = build(opts.upstream, opts.pins)
    except Stop as err:
        print(f"generate.py: {err}; nothing written", file=sys.stderr)
        return 1
    packed = gzip.compress(data, compresslevel=9, mtime=0)
    target = opts.out / "keywords.bin.gz"
    before = previous_words(target)
    opts.out.mkdir(parents=True, exist_ok=True)
    target.write_bytes(packed)
    (opts.out / "keywords-LICENSES.txt").write_text(licences_text(sources, opts.upstream), encoding="utf-8")
    opts.related.parent.mkdir(parents=True, exist_ok=True)
    opts.related.write_text(related_text(related, opts.pins), encoding="utf-8")
    for s in sources:
        print(f"{s['prefix']}: {len(s['icons'])} icons with keywords ({s['package']} {s['version']})")
    pairs = sum(len(w) for w in related.values())
    print(f"{len(words)} words, {len(related)} related heads, {pairs} related pairs, {NULL_TAGS[0]} null tags read as \"null\"")
    if before is not None:
        now = set(words)
        print(f"words gained {len(now - before)}, lost {len(before - now)}")
    print(f"payload {len(data)} bytes, {target.name} {len(packed)} bytes ({len(packed) / 1024:.1f} KiB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
