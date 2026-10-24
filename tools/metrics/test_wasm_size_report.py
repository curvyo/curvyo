"""Tests for wasm_size_report.py: `cd tools/metrics && python3 -m unittest`."""

import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import wasm_size_report as report

MAIN_A = report.Row("2026-10-20", "4ece1b1d", "", 10_000_000, 3_000_000, "main")
BRANCH = report.Row("2026-10-21", "aaaaaaaa", "", 13_000_000, 4_500_000, "story/x")
MAIN_B = report.Row("2026-10-22", "3ff03cf8", "42", 12_000_000, 4_000_000, "main")


class RoundTrip(unittest.TestCase):
    def test_append_then_read_keeps_order_and_values(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "wasm-size.csv"
            for row in (MAIN_A, BRANCH, MAIN_B):
                report.append_row(path, row)
            self.assertEqual(report.read_rows(path), [MAIN_A, BRANCH, MAIN_B])
            self.assertEqual(path.read_text().splitlines()[0], ",".join(report.FIELDS))

    def test_missing_csv_has_no_rows(self):
        self.assertEqual(report.read_rows(Path("/nonexistent/wasm-size.csv")), [])


class Summary(unittest.TestCase):
    def test_delta_is_against_newest_main_row_not_branch_rows(self):
        text = report.summary([MAIN_A, MAIN_B, BRANCH], 12_001_024, 3_999_000, "deadbeef00")
        self.assertIn("+1,024 bytes (+1.0 KiB", text)
        self.assertIn("-1,000 bytes", text)
        self.assertIn("main at `3ff03cf`", text)

    def test_without_main_row_reports_sizes_only(self):
        text = report.summary([BRANCH], 1, 1, "deadbeef")
        self.assertIn("no main value recorded", text)


class Render(unittest.TestCase):
    def test_table_is_newest_first_and_marks_branch_rows(self):
        lines = [l for l in report.render_md([MAIN_A, BRANCH, MAIN_B]).splitlines() if l.startswith("| 2026")]
        self.assertTrue(lines[0].startswith("| 2026-10-22"))
        self.assertIn("#42", lines[0])
        self.assertIn("+2,000,000", lines[0])
        self.assertIn("story/x (branch)", lines[1])
        self.assertIn("+3,000,000", lines[1])
        self.assertTrue(lines[2].endswith("|  |"))

    def test_svg_is_well_formed_with_one_marker_per_value(self):
        for rows in ([], [MAIN_A], [MAIN_A, BRANCH, MAIN_B]):
            root = ET.fromstring(report.render_svg(rows))
            circles = root.findall(".//{http://www.w3.org/2000/svg}circle")
            self.assertEqual(len(circles), 1 + 2 * len(rows))  # +1: legend marker


class PrNumber(unittest.TestCase):
    def test_merge_and_squash_messages(self):
        self.assertEqual(report.pr_from_message("Merge pull request #158 from x/y"), "158")
        self.assertEqual(report.pr_from_message("feat: thing (#77)\n\nbody"), "77")
        self.assertEqual(report.pr_from_message("feat: rebased commit, no number"), "")


if __name__ == "__main__":
    unittest.main()
