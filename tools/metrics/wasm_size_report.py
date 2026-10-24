#!/usr/bin/env python3
"""Records and reports the size of the release editor wasm (CI job `wasm size`).

Standard library only, so CI needs no install. Subcommands:

  summary  Markdown for a pull request: size, gzip and the delta against the
           newest `main` row of the CSV (the CSV may be missing).
  append   Adds one row to the CSV (creates it with a header if missing).
  render   Writes the Markdown table (newest first) and the SVG chart.
  pr       Prints the PR number found in a commit message, or nothing.

CSV columns: date (UTC, YYYY-MM-DD), commit, pr, raw_bytes, gzip_bytes, ref.
`ref` is `main` for rows recorded on main; any other value names the branch a
row was measured on, and such rows never serve as the delta baseline.
"""

from __future__ import annotations

import argparse
import csv
import html
import re
import sys
from dataclasses import dataclass
from pathlib import Path

FIELDS = ["date", "commit", "pr", "raw_bytes", "gzip_bytes", "ref"]
MIB = 1024 * 1024


@dataclass(frozen=True)
class Row:
    """One measurement."""

    date: str
    commit: str
    pr: str
    raw_bytes: int
    gzip_bytes: int
    ref: str

    @property
    def on_main(self) -> bool:
        return self.ref == "main"


def read_rows(path: Path) -> list[Row]:
    """Rows in file order (oldest first); an absent file has none."""
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as handle:
        return [
            Row(
                date=r["date"],
                commit=r["commit"],
                pr=r["pr"],
                raw_bytes=int(r["raw_bytes"]),
                gzip_bytes=int(r["gzip_bytes"]),
                ref=r["ref"],
            )
            for r in csv.DictReader(handle)
        ]


def append_row(path: Path, row: Row) -> None:
    """Appends `row`, writing the header first when the file is new."""
    new = not path.exists() or path.stat().st_size == 0
    with path.open("a", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        if new:
            writer.writerow(FIELDS)
        writer.writerow(
            [row.date, row.commit, row.pr, row.raw_bytes, row.gzip_bytes, row.ref]
        )


def latest_main(rows: list[Row]) -> Row | None:
    """The newest row recorded on main."""
    mains = [r for r in rows if r.on_main]
    return mains[-1] if mains else None


def fmt_bytes(value: int) -> str:
    return f"{value:,} ({value / 1024:,.0f} KiB)"


def fmt_delta(value: int, base: int) -> str:
    sign = "+" if value >= 0 else "-"
    pct = abs(value) / base * 100 if base else 0.0
    return f"{sign}{abs(value):,} bytes ({sign}{abs(value) / 1024:,.1f} KiB, {sign}{pct:.2f} %)"


def summary(rows: list[Row], raw: int, gzip: int, commit: str) -> str:
    """Markdown for the job summary of a pull request."""
    base = latest_main(rows)
    lines = [
        "## Editor wasm size",
        "",
        f"`curvyo_editor_wasm_bg.wasm` at `{commit[:7]}` (release, after wasm-bindgen, no wasm-opt).",
        "",
        "| | Bytes | Delta against main |",
        "|---|---|---|",
    ]
    if base is None:
        lines += [f"| raw | {fmt_bytes(raw)} | no main value recorded |",
                  f"| gzip -9 | {fmt_bytes(gzip)} | no main value recorded |"]
    else:
        lines += [
            f"| raw | {fmt_bytes(raw)} | {fmt_delta(raw - base.raw_bytes, base.raw_bytes)} |",
            f"| gzip -9 | {fmt_bytes(gzip)} | {fmt_delta(gzip - base.gzip_bytes, base.gzip_bytes)} |",
            "",
            f"Baseline: main at `{base.commit[:7]}` ({base.date}).",
        ]
    return "\n".join(lines) + "\n"


def render_md(rows: list[Row]) -> str:
    """The history table, newest first."""
    lines = [
        "# Editor wasm size",
        "",
        "Release `curvyo_editor_wasm_bg.wasm` after wasm-bindgen, no wasm-opt, as built by",
        "`frontend/scripts/build-wasm.sh`. Recorded by the CI job `wasm size` on every push to",
        "`main`; rows whose ref is not `main` were measured on a branch by hand.",
        "",
        "![Editor wasm size over time](wasm-size.svg)",
        "",
        "| Date (UTC) | Commit | PR | Ref | Bytes | gzip -9 | Bytes change |",
        "|---|---|---|---|---|---|---|",
    ]
    previous: dict[int, Row] = {}
    for index, row in enumerate(rows):
        mains_before = [r for r in rows[:index] if r.on_main]
        if mains_before:
            previous[index] = mains_before[-1]
    for index in range(len(rows) - 1, -1, -1):
        row = rows[index]
        base = previous.get(index)
        change = f"{row.raw_bytes - base.raw_bytes:+,}" if base else ""
        pr = f"#{row.pr}" if row.pr else ""
        ref = row.ref if row.on_main else f"{row.ref} (branch)"
        lines.append(
            f"| {row.date} | `{row.commit[:7]}` | {pr} | {ref} | "
            f"{row.raw_bytes:,} | {row.gzip_bytes:,} | {change} |"
        )
    return "\n".join(lines) + "\n"


SVG_STYLE = """
  .surface { fill: #fcfcfb; }
  .grid { stroke: #e4e3dd; stroke-width: 1; }
  .axis { stroke: #a9a89f; stroke-width: 1; }
  .ink { fill: #1f1f1d; font: 13px sans-serif; }
  .muted { fill: #6b6a63; font: 11px sans-serif; }
  .raw { stroke: #2a78d6; fill: #2a78d6; }
  .gz { stroke: #eb6834; fill: #eb6834; }
  .line { fill: none; stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }
  .hollow { fill: #fcfcfb; stroke-width: 2; }
  @media (prefers-color-scheme: dark) {
    .surface { fill: #1a1a19; }
    .grid { stroke: #33332f; }
    .axis { stroke: #5c5b55; }
    .ink { fill: #f0efec; }
    .muted { fill: #a9a89f; }
    .raw { stroke: #3987e5; fill: #3987e5; }
    .gz { stroke: #d95926; fill: #d95926; }
    .hollow { fill: #1a1a19; }
  }
"""


def render_svg(rows: list[Row]) -> str:
    """A line chart of raw and gzip size per row, in MiB, on one axis from zero.

    Branch rows are drawn as hollow markers; the lines join all rows in order.
    """
    width, height = 720, 300
    left, right, top, bottom = 56, 96, 40, 48
    plot_w, plot_h = width - left - right, height - top - bottom
    peak = max((r.raw_bytes for r in rows), default=MIB) / MIB
    step = 2 if peak <= 12 else 4
    y_max = max(step, (int(peak // step) + 1) * step)

    def x_at(i: int) -> float:
        return left + (plot_w / 2 if len(rows) == 1 else i * plot_w / (len(rows) - 1))

    def y_at(value: int) -> float:
        return top + plot_h - (value / MIB) / y_max * plot_h

    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" role="img" aria-labelledby="t">',
        f"<title id=\"t\">Editor wasm size over time, raw and gzip -9, in MiB</title>",
        f"<style>{SVG_STYLE}</style>",
        f'<rect class="surface" width="{width}" height="{height}"/>',
        f'<text class="ink" x="{left}" y="22">Editor wasm size (MiB)</text>',
        f'<rect class="raw" x="{left + 190}" y="13" width="12" height="3" rx="1"/>',
        f'<text class="muted" x="{left + 208}" y="19">raw</text>',
        f'<rect class="gz" x="{left + 246}" y="13" width="12" height="3" rx="1"/>',
        f'<text class="muted" x="{left + 264}" y="19">gzip -9</text>',
        f'<circle class="raw hollow" cx="{left + 330}" cy="14.5" r="4"/>',
        f'<text class="muted" x="{left + 340}" y="19">measured on a branch</text>',
    ]
    for tick in range(0, y_max + 1, step):
        y = top + plot_h - tick / y_max * plot_h
        css = "axis" if tick == 0 else "grid"
        out.append(f'<line class="{css}" x1="{left}" y1="{y:.1f}" x2="{left + plot_w}" y2="{y:.1f}"/>')
        out.append(f'<text class="muted" x="{left - 8}" y="{y + 4:.1f}" text-anchor="end">{tick}</text>')
    label_every = max(1, (len(rows) + 7) // 8)
    for i, row in enumerate(rows):
        if i % label_every == 0 or i == len(rows) - 1:
            out.append(
                f'<text class="muted" x="{x_at(i):.1f}" y="{top + plot_h + 18}" '
                f'text-anchor="middle">{html.escape(row.date[5:])}</text>'
            )
    for css, attr, name in (("raw", "raw_bytes", "raw"), ("gz", "gzip_bytes", "gzip -9")):
        points = " ".join(f"{x_at(i):.1f},{y_at(getattr(r, attr)):.1f}" for i, r in enumerate(rows))
        if len(rows) > 1:
            out.append(f'<polyline class="{css} line" points="{points}"/>')
        for i, row in enumerate(rows):
            value = getattr(row, attr)
            hollow = "" if row.on_main else " hollow"
            tip = html.escape(f"{row.date} {row.commit[:7]} ({row.ref}): {name} {value:,} bytes")
            out.append(
                f'<circle class="{css}{hollow}" cx="{x_at(i):.1f}" cy="{y_at(value):.1f}" r="4">'
                f"<title>{tip}</title></circle>"
            )
        if rows:
            last = rows[-1]
            value = getattr(last, attr)
            out.append(
                f'<text class="ink" x="{x_at(len(rows) - 1) + 10:.1f}" y="{y_at(value) + 4:.1f}">'
                f"{name} {value / MIB:.2f}</text>"
            )
    out.append("</svg>")
    return "\n".join(out) + "\n"


PR_PATTERNS = (
    re.compile(r"Merge pull request #(\d+)"),
    re.compile(r"\(#(\d+)\)\s*$", re.MULTILINE),
)


def pr_from_message(message: str) -> str:
    """The PR number named by a merge or squash commit message, else ''."""
    for pattern in PR_PATTERNS:
        match = pattern.search(message)
        if match:
            return match.group(1)
    return ""


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    p_sum = sub.add_parser("summary")
    p_sum.add_argument("--csv", type=Path, required=True)
    p_sum.add_argument("--raw", type=int, required=True)
    p_sum.add_argument("--gzip", type=int, required=True)
    p_sum.add_argument("--commit", required=True)

    p_app = sub.add_parser("append")
    p_app.add_argument("--csv", type=Path, required=True)
    p_app.add_argument("--date", required=True)
    p_app.add_argument("--commit", required=True)
    p_app.add_argument("--pr", default="")
    p_app.add_argument("--raw", type=int, required=True)
    p_app.add_argument("--gzip", type=int, required=True)
    p_app.add_argument("--ref", default="main")

    p_ren = sub.add_parser("render")
    p_ren.add_argument("--csv", type=Path, required=True)
    p_ren.add_argument("--md", type=Path, required=True)
    p_ren.add_argument("--svg", type=Path, required=True)

    p_pr = sub.add_parser("pr")
    p_pr.add_argument("--message", required=True)

    args = parser.parse_args(argv)
    if args.command == "summary":
        sys.stdout.write(summary(read_rows(args.csv), args.raw, args.gzip, args.commit))
    elif args.command == "append":
        append_row(args.csv, Row(args.date, args.commit, args.pr, args.raw, args.gzip, args.ref))
    elif args.command == "render":
        rows = read_rows(args.csv)
        args.md.write_text(render_md(rows), encoding="utf-8")
        args.svg.write_text(render_svg(rows), encoding="utf-8")
    else:
        sys.stdout.write(pr_from_message(args.message))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
