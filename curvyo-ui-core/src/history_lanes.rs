//! The items of the History list and the lanes drawn beside them: rows, branch header rows and the
//! "N more branches" row, with the slice of the gutter each item draws (`specs/0042-history-branches`
//! criteria 8 and 9, UX notes "Branches in a 244 px list"). Pure presentation of branches the
//! document derived.

use std::collections::HashMap;

/// How many branches get a lane and an unfolded header before the rest fold into one row.
pub const DRAWN_BRANCHES: usize = 3;

/// The rightmost lane: the main lane is 0, branch lanes are 1 to 3.
pub const MAX_LANE: u8 = 3;

/// A branch as the layout reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSpan {
    /// Its rows as indices into the list (0 is the newest), newest first; never empty.
    pub rows: Vec<usize>,
    /// The row it forked from; `None` when it forked before the first listed row.
    pub fork_row: Option<usize>,
}

/// One item of the list, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryItem {
    /// A step row: the index into the list of rows.
    Row(usize),
    /// The header row of a branch: the index into the branches.
    Header(usize),
    /// The row that holds the branches beyond the first three.
    More {
        /// How many branches it holds.
        count: usize,
    },
}

/// Where a line of the gutter starts or ends inside its item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// The item's top edge.
    Top,
    /// The height of the item's mark (a row's) or its middle (a header row's).
    Mark,
    /// The item's bottom edge.
    Bottom,
}

/// How a line of the gutter is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    /// The main lane: the line the current state is on.
    Main,
    /// The main lane through a row that was taken back and can be redone.
    MainDotted,
    /// The main lane through a row an object copy took over from its original.
    MainDashed,
    /// A branch lane.
    Branch,
}

/// One part of the gutter of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    /// A vertical line in `lane`.
    Line {
        /// The lane, 0 being the main lane.
        lane: u8,
        /// Where it starts.
        from: Edge,
        /// Where it ends.
        to: Edge,
        /// How it is drawn.
        style: LineStyle,
    },
    /// The filled mark of a step in effect.
    Disc {
        /// The lane.
        lane: u8,
    },
    /// The open mark of a step that was taken back.
    Ring {
        /// The lane.
        lane: u8,
    },
    /// The connector at a fork point: leaves `from_lane` at the mark to the right and runs up
    /// `lane`.
    Fork {
        /// The lane of the line the branch forks from.
        from_lane: u8,
        /// The branch's lane.
        lane: u8,
    },
}

/// How the main-lane mark of a row looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowLook {
    /// The row was taken back and can be redone.
    pub undone: bool,
    /// The row was taken over from the object a copy was made of.
    pub inherited: bool,
}

/// The list as it shows: which items, which branches are unfolded and which lane each drawn
/// branch has.
#[derive(Debug, Clone)]
pub struct ListLayout {
    items: Vec<HistoryItem>,
    item_of_row: HashMap<usize, usize>,
    branch_of_row: HashMap<usize, usize>,
    open: Vec<bool>,
    header_item: Vec<Option<usize>>,
    fork_item: Vec<Option<usize>>,
    lanes: Vec<Option<u8>>,
    parent: Vec<Option<usize>>,
    more_open: bool,
}

impl ListLayout {
    /// Lays out `total` rows with `branches` (newest first). The first three branches start
    /// unfolded and the rest are held by one "N more branches" row; `toggled` says, per branch,
    /// that the maker pressed its header an odd number of times; `more_open` that the "more" row
    /// is unfolded.
    #[must_use]
    pub fn new(total: usize, branches: &[BranchSpan], toggled: &[bool], more_open: bool) -> Self {
        let branch_of_row: HashMap<usize, usize> = branches
            .iter()
            .enumerate()
            .flat_map(|(b, branch)| branch.rows.iter().map(move |row| (*row, b)))
            .collect();
        let parent: Vec<Option<usize>> = branches
            .iter()
            .map(|branch| {
                branch
                    .fork_row
                    .and_then(|row| branch_of_row.get(&row).copied())
            })
            .collect();
        let primary = primary_branches(&parent);
        let open: Vec<bool> = (0..branches.len())
            .map(|b| primary[b] != toggled.get(b).copied().unwrap_or(false))
            .collect();
        let mut header_shown = vec![false; branches.len()];
        // A parent comes after its child in the newest-first order only when the child is
        // newer; resolve in rounds until nothing changes.
        let mut changed = true;
        while changed {
            changed = false;
            for b in 0..branches.len() {
                let shown = match parent[b] {
                    None => primary[b] || more_open,
                    Some(p) if !primary[b] && primary[p] => more_open && header_shown[p] && open[p],
                    Some(p) => header_shown[p] && open[p],
                };
                if shown != header_shown[b] {
                    header_shown[b] = shown;
                    changed = true;
                }
            }
        }
        let header_at: HashMap<usize, usize> = branches
            .iter()
            .enumerate()
            .filter(|(b, _)| header_shown[*b])
            .filter_map(|(b, branch)| Some((*branch.rows.first()?, b)))
            .collect();
        let more_roots: Vec<usize> = (0..branches.len())
            .filter(|b| !primary[*b] && parent[*b].is_none_or(|p| primary[p]))
            .collect();
        let more_count = primary.iter().filter(|p| !**p).count();
        let more_at = more_roots
            .iter()
            .filter_map(|b| branches[*b].rows.first().copied())
            .min();
        let mut items = Vec::with_capacity(total + branches.len() + 1);
        let mut item_of_row = HashMap::new();
        let mut header_item = vec![None; branches.len()];
        for row in 0..total {
            if more_at == Some(row) {
                items.push(HistoryItem::More { count: more_count });
            }
            if let Some(b) = header_at.get(&row) {
                header_item[*b] = Some(items.len());
                items.push(HistoryItem::Header(*b));
            }
            let visible = branch_of_row
                .get(&row)
                .is_none_or(|b| header_shown[*b] && open[*b]);
            if visible {
                item_of_row.insert(row, items.len());
                items.push(HistoryItem::Row(row));
            }
        }
        let fork_item: Vec<Option<usize>> = branches
            .iter()
            .map(|branch| {
                branch
                    .fork_row
                    .and_then(|row| item_of_row.get(&row).copied())
            })
            .collect();
        let lanes = assign_lanes(branches, &parent, &header_shown, total);
        Self {
            items,
            item_of_row,
            branch_of_row,
            open,
            header_item,
            fork_item,
            lanes,
            parent,
            more_open,
        }
    }

    /// The items, top to bottom.
    #[must_use]
    pub fn items(&self) -> &[HistoryItem] {
        &self.items
    }

    /// The item that shows `row`, unless its branch is folded.
    #[must_use]
    pub fn item_of_row(&self, row: usize) -> Option<usize> {
        self.item_of_row.get(&row).copied()
    }

    /// The branch `row` is on.
    #[must_use]
    pub fn branch_of_row(&self, row: usize) -> Option<usize> {
        self.branch_of_row.get(&row).copied()
    }

    /// Whether the branch's rows show.
    #[must_use]
    pub fn is_open(&self, branch: usize) -> bool {
        self.open.get(branch).copied().unwrap_or(false)
    }

    /// Whether the "more" row is unfolded.
    #[must_use]
    pub fn more_open(&self) -> bool {
        self.more_open
    }

    /// The rightmost lane any item draws in: the gutter is 16 px plus 12 px per lane.
    #[must_use]
    pub fn lane_count(&self) -> u8 {
        self.lanes.iter().flatten().copied().max().unwrap_or(0)
    }

    /// The slice of the gutter `item` draws. `look` is the main-lane look of the item's row.
    #[must_use]
    pub fn gutter(&self, item: usize, look: RowLook) -> Vec<Segment> {
        let Some(this) = self.items.get(item) else {
            return Vec::new();
        };
        let last = item + 1 == self.items.len();
        let mut out = Vec::new();
        let main_row =
            matches!(this, HistoryItem::Row(row) if !self.branch_of_row.contains_key(row));
        let style = if main_row && look.undone {
            LineStyle::MainDotted
        } else if main_row && look.inherited {
            LineStyle::MainDashed
        } else {
            LineStyle::Main
        };
        out.push(Segment::Line {
            lane: 0,
            from: Edge::Top,
            to: if last { Edge::Mark } else { Edge::Bottom },
            style,
        });
        if main_row {
            out.push(if look.undone {
                Segment::Ring { lane: 0 }
            } else {
                Segment::Disc { lane: 0 }
            });
        }
        for (b, lane) in self.lanes.iter().enumerate() {
            let (Some(lane), Some(header)) = (*lane, self.header_item[b]) else {
                continue;
            };
            let fork = self.fork_item[b];
            let branch_line = |from, to| Segment::Line {
                lane,
                from,
                to,
                style: LineStyle::Branch,
            };
            if item == header {
                out.push(branch_line(Edge::Mark, Edge::Bottom));
            } else if item > header && fork.is_none_or(|fork| item < fork) {
                out.push(branch_line(Edge::Top, Edge::Bottom));
                if matches!(this, HistoryItem::Row(row) if self.branch_of_row.get(row) == Some(&b))
                {
                    out.push(Segment::Ring { lane });
                }
            } else if fork == Some(item) {
                out.push(branch_line(Edge::Top, Edge::Mark));
                let from_lane = self.parent[b].and_then(|p| self.lanes[p]).unwrap_or(0);
                out.push(Segment::Fork { from_lane, lane });
            }
        }
        out
    }
}

/// The first three branches, newest first, whose parent is drawn too.
fn primary_branches(parent: &[Option<usize>]) -> Vec<bool> {
    let mut primary = vec![false; parent.len()];
    let mut count = 0;
    for b in 0..parent.len() {
        if count < DRAWN_BRANCHES && parent[b].is_none_or(|p| primary[p]) {
            primary[b] = true;
            count += 1;
        }
    }
    primary
}

/// A lane per branch whose header shows, newest first: the leftmost lane right of its parent's that
/// no other drawn branch holds over the same rows; the rightmost lane when all are taken.
fn assign_lanes(
    branches: &[BranchSpan],
    parent: &[Option<usize>],
    shown: &[bool],
    total: usize,
) -> Vec<Option<u8>> {
    let span = |b: usize| {
        let top = branches[b].rows.first().copied().unwrap_or(0);
        (top, branches[b].fork_row.unwrap_or(total))
    };
    let mut lanes: Vec<Option<u8>> = vec![None; branches.len()];
    for b in 0..branches.len() {
        if !shown[b] {
            continue;
        }
        let min = parent[b]
            .and_then(|p| lanes[p])
            .map_or(1, |l| l + 1)
            .min(MAX_LANE);
        let (top, bottom) = span(b);
        let taken = |lane: u8| {
            (0..branches.len()).any(|other| {
                lanes[other] == Some(lane) && {
                    let (t, o) = span(other);
                    t <= bottom && top <= o
                }
            })
        };
        lanes[b] = Some(
            (min..=MAX_LANE)
                .find(|lane| !taken(*lane))
                .unwrap_or(MAX_LANE),
        );
    }
    lanes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(rows: &[usize], fork_row: Option<usize>) -> BranchSpan {
        BranchSpan {
            rows: rows.to_vec(),
            fork_row,
        }
    }

    #[test]
    fn a_header_row_sits_directly_above_the_newest_row_of_its_branch() {
        // s4, [s3, s2] from s1, create.
        let layout = ListLayout::new(5, &[span(&[1, 2], Some(3))], &[], false);
        assert_eq!(
            layout.items(),
            [
                HistoryItem::Row(0),
                HistoryItem::Header(0),
                HistoryItem::Row(1),
                HistoryItem::Row(2),
                HistoryItem::Row(3),
                HistoryItem::Row(4),
            ]
        );
        assert_eq!(layout.lane_count(), 1);
    }

    #[test]
    fn pressing_a_header_folds_the_branch_rows_out_of_the_list() {
        let layout = ListLayout::new(5, &[span(&[1, 2], Some(3))], &[true], false);
        assert_eq!(
            layout.items(),
            [
                HistoryItem::Row(0),
                HistoryItem::Header(0),
                HistoryItem::Row(3),
                HistoryItem::Row(4)
            ]
        );
        assert!(!layout.is_open(0));
        assert_eq!(layout.item_of_row(1), None);
    }

    #[test]
    fn the_fourth_and_later_branches_fold_into_one_row_that_unfolds_into_folded_headers() {
        // Five one-step branches from the main line, each forking from the row below it.
        let branches: Vec<BranchSpan> = (0..5)
            .map(|n| span(&[1 + 2 * n], Some(2 + 2 * n)))
            .collect();
        let closed = ListLayout::new(11, &branches, &[], false);
        let headers = closed
            .items()
            .iter()
            .filter(|item| matches!(item, HistoryItem::Header(_)))
            .count();
        assert_eq!(headers, 3);
        assert!(closed.items().contains(&HistoryItem::More { count: 2 }));
        assert_eq!(
            closed.item_of_row(7),
            None,
            "a held branch's row is not listed"
        );
        let opened = ListLayout::new(11, &branches, &[], true);
        assert!(opened.items().contains(&HistoryItem::Header(3)));
        assert!(opened.items().contains(&HistoryItem::Header(4)));
        assert!(!opened.is_open(3), "revealed headers start folded");
        assert_eq!(opened.item_of_row(7), None);
        let unfolded = ListLayout::new(11, &branches, &[false, false, false, true], true);
        assert!(unfolded.item_of_row(7).is_some());
    }

    #[test]
    fn the_gutter_draws_the_main_lane_the_branch_lane_and_the_fork_connector() {
        let layout = ListLayout::new(5, &[span(&[1, 2], Some(3))], &[], false);
        let main = |to| Segment::Line {
            lane: 0,
            from: Edge::Top,
            to,
            style: LineStyle::Main,
        };
        let branch = |from, to| Segment::Line {
            lane: 1,
            from,
            to,
            style: LineStyle::Branch,
        };
        let look = RowLook::default();
        assert_eq!(
            layout.gutter(0, look),
            [main(Edge::Bottom), Segment::Disc { lane: 0 }]
        );
        assert_eq!(
            layout.gutter(1, look),
            [main(Edge::Bottom), branch(Edge::Mark, Edge::Bottom)]
        );
        assert_eq!(
            layout.gutter(2, look),
            [
                main(Edge::Bottom),
                branch(Edge::Top, Edge::Bottom),
                Segment::Ring { lane: 1 }
            ]
        );
        assert_eq!(
            layout.gutter(4, look),
            [
                main(Edge::Bottom),
                Segment::Disc { lane: 0 },
                branch(Edge::Top, Edge::Mark),
                Segment::Fork {
                    from_lane: 0,
                    lane: 1
                }
            ]
        );
        assert_eq!(
            layout.gutter(5, look),
            [main(Edge::Mark), Segment::Disc { lane: 0 }]
        );
    }

    #[test]
    fn an_undone_main_row_has_a_ring_on_a_dotted_line() {
        let layout = ListLayout::new(2, &[], &[], false);
        let gutter = layout.gutter(
            0,
            RowLook {
                undone: true,
                inherited: false,
            },
        );
        assert_eq!(
            gutter,
            [
                Segment::Line {
                    lane: 0,
                    from: Edge::Top,
                    to: Edge::Bottom,
                    style: LineStyle::MainDotted
                },
                Segment::Ring { lane: 0 }
            ]
        );
    }

    #[test]
    fn a_branch_that_has_a_branch_is_drawn_one_lane_further_right() {
        // s5; [s4, s2] from s1; [s3] from s2; s1.
        let branches = [span(&[1, 3], Some(4)), span(&[2], Some(3))];
        let layout = ListLayout::new(5, &branches, &[], false);
        assert_eq!(layout.lane_count(), 2);
        let fork_item = layout.item_of_row(3).unwrap();
        assert!(
            layout
                .gutter(fork_item, RowLook::default())
                .contains(&Segment::Fork {
                    from_lane: 1,
                    lane: 2
                })
        );
    }

    #[test]
    fn folding_a_branch_hides_the_branches_that_fork_from_it() {
        let branches = [span(&[1, 3], Some(4)), span(&[2], Some(3))];
        let layout = ListLayout::new(5, &branches, &[true], false);
        assert!(!layout.items().contains(&HistoryItem::Header(1)));
        assert_eq!(layout.item_of_row(2), None);
    }

    #[test]
    fn branches_whose_rows_do_not_overlap_share_a_lane() {
        let branches = [span(&[1], Some(2)), span(&[3], Some(4))];
        let layout = ListLayout::new(5, &branches, &[], false);
        assert_eq!(layout.lane_count(), 1);
    }

    #[test]
    fn fifty_branches_over_ten_thousand_rows_lay_out_quickly() {
        let branches: Vec<BranchSpan> = (0..50)
            .map(|n| span(&[n * 200 + 1, n * 200 + 2], Some(n * 200 + 3)))
            .collect();
        let started = std::time::Instant::now();
        let layout = ListLayout::new(10_000, &branches, &[], false);
        let _ = layout.gutter(0, RowLook::default());
        assert!(started.elapsed() < std::time::Duration::from_millis(800));
        assert!(layout.items().len() > 9_000);
    }
}
