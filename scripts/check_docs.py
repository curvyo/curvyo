#!/usr/bin/env python3
"""Checks the documentation tree: unique spec and ADR numbers, every ADR file in the ADR index,
and relative Markdown links that resolve. Run from the repository root; exits 1 on any finding."""

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(".")
# Numbers that were already used twice before this check existed (ADR 0013 was written twice).
KNOWN_DUPLICATES = {("docs/adr", "0013")}
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
FENCE = re.compile(r"^(```|~~~)")


def duplicate_numbers(directory: Path, pattern: str) -> list[str]:
    seen = defaultdict(list)
    for entry in sorted(directory.iterdir()):
        match = re.match(pattern, entry.name)
        if match:
            seen[match.group(1)].append(entry.name)
    return [
        f"{directory}: number {n} is used by {', '.join(names)}"
        for n, names in seen.items()
        if len(names) > 1 and (str(directory), n) not in KNOWN_DUPLICATES
    ]


def adrs_missing_from_index() -> list[str]:
    index = (ROOT / "docs/adr/index.md").read_text(encoding="utf-8")
    problems = []
    for adr in sorted((ROOT / "docs/adr").glob("[0-9][0-9][0-9][0-9]-*.md")):
        if f"]({adr.name})" not in index:
            problems.append(f"docs/adr/index.md has no row linking {adr.name}")
    return problems


def broken_links() -> list[str]:
    problems = []
    for base in ("docs", "specs"):
        for md in sorted((ROOT / base).rglob("*.md")):
            in_fence = False
            for number, line in enumerate(md.read_text(encoding="utf-8").splitlines(), 1):
                if FENCE.match(line.strip()):
                    in_fence = not in_fence
                if in_fence:
                    continue
                for target in LINK.findall(line):
                    if re.match(r"^[a-z][a-z0-9+.-]*:", target) or target.startswith("#"):
                        continue
                    path = target.split("#", 1)[0]
                    if path and not (md.parent / path).exists():
                        problems.append(f"{md}:{number}: link target {target} does not exist")
    return problems


def main() -> int:
    findings = (
        duplicate_numbers(ROOT / "specs", r"^(\d{4})-")
        + duplicate_numbers(ROOT / "docs/adr", r"^(\d{4})-")
        + adrs_missing_from_index()
        + broken_links()
    )
    for finding in findings:
        print(finding)
    print(f"{len(findings)} finding(s)")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
