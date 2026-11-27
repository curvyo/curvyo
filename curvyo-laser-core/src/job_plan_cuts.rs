//! The cut path step the path units of one block take (`specs/0139` criteria 3 to 5, 11, 19 to
//! 22, adrs L7): the block's cut settings, each outline's tabs, path options and entry, the
//! damage of an item's records, and the marks they leave.

use curvyo_document_core::NodeId;

use crate::cut_entry_records::{LeafPath, OutlineEntry};
use crate::cut_item_records::LeafTabs;
use crate::cut_tabs::{Measured, overlap, own_spans};
use crate::job_cut_path::{TabGap, UnitCut};
use crate::job_flatten::flat;
use crate::job_input::{BlockInput, BlockSource, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_order::{Unit, UnitShape};
use crate::job_plan::{Units, cut_out_outlines, index_u32};
use crate::operation_cut_options::{CutSettings, cuts_through};
use crate::operation_path_options::PathOptions;

/// The marks of one cut outline for the Operations view (`specs/0139` criteria 7, 14, 15, 19),
/// from the same pieces the moves are made of (adrs L12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutMark {
    /// The leaf or Cut-out item.
    pub node: NodeId,
    /// The outline's index in its leaf (the contour index of a Cut-out item).
    pub outline: u32,
    /// Its tab gaps.
    pub gaps: Vec<TabGap>,
    /// Its lead-in, from its start to the first burnt point; empty without one.
    pub lead_in: Vec<PointUm>,
    /// Its overcut along the path; empty without one.
    pub overcut: Vec<PointUm>,
    /// Its lead-out, from the last burnt point; empty without one.
    pub lead_out: Vec<PointUm>,
    /// A set start or fixed direction: where the burn begins and the unit direction there times
    /// 1,000,000 (the chevron); `None` for Automatic.
    pub start: Option<(PointUm, (i32, i32))>,
}

/// Where the job really enters one closed cut outline (`specs/0139` criterion 19, C7): the
/// Automatic start marker rests there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutStart {
    /// The leaf or Cut-out item.
    pub node: NodeId,
    /// The outline's index in its leaf (the contour index of a Cut-out item).
    pub outline: u32,
    /// The first burnt point on the outline.
    pub at: PointUm,
}

/// What the cut path step of one block needs and leaves (`specs/0139`).
#[derive(Debug, Default)]
pub(crate) struct Cuts {
    /// The block's cut settings; `None` for a block whose kind takes no tabs, a fixture block or
    /// a test grid's derived operation (adrs L7).
    pub(crate) settings: Option<CutSettings>,
    /// The block's path options (the default when `settings` is `None`).
    pub(crate) path: PathOptions,
    pub(crate) marks: Vec<CutMark>,
    pub(crate) covered: Vec<NodeId>,
    pub(crate) damaged: Vec<NodeId>,
    pub(crate) placed: Vec<usize>,
    pub(crate) starts: Vec<CutStart>,
    pub(crate) leads_touch: Vec<NodeId>,
}

impl Cuts {
    /// The cut path step of `block` with `numbers`.
    pub(crate) fn of(block: &BlockInput, numbers: &BlockNumbers) -> Self {
        let settings = block_cut(block, numbers);
        Self {
            settings,
            path: if settings.is_some() {
                numbers.options.path
            } else {
                PathOptions::DEFAULT
            },
            ..Self::default()
        }
    }
}

/// The cut settings of `block` when its paths may get tabs.
pub(crate) fn block_cut(block: &BlockInput, numbers: &BlockNumbers) -> Option<CutSettings> {
    let BlockSource::Operation { grid: None, .. } = &block.source else {
        return None;
    };
    cuts_through(numbers.kind).then_some(numbers.options.cut)
}

/// The cut path step of outline `index` (flattened to `points`) of a leaf or item with tabs
/// `tabs` and records `path` in a block of `cuts`; `None` when it runs as today (no own tabs,
/// Count 0, no lead or overcut on a closed outline and Automatic, criterion 24).
pub(crate) fn unit_cut(
    cuts: &Cuts,
    tabs: &LeafTabs,
    path: &LeafPath,
    index: u32,
    points: &[PointUm],
    closed: bool,
) -> Option<Box<UnitCut>> {
    let settings = cuts.settings?;
    let own = tabs.own(index);
    let no_tabs =
        own.is_none_or(<[_]>::is_empty) && (own.is_some() || settings.tab_count.get() == 0);
    let options = if closed {
        path.options(cuts.path)
    } else {
        PathOptions::DEFAULT
    };
    let entry = path
        .entry(index)
        .filter(|entry| matches!(entry, OutlineEntry::Start(_)) == closed);
    if no_tabs && options.is_inert() && entry.is_none() {
        return None;
    }
    let measured = Measured::new(points.to_vec(), closed);
    let own = own.map(|tabs| own_spans(&measured, tabs, settings.tab_width));
    Some(Box::new(UnitCut {
        measured,
        own,
        settings,
        path: options,
        entry,
    }))
}

/// Whether the own tabs `tabs` of a leaf with `outlines` (flattened) overlap on any of them
/// (damage, criterion 11).
pub(crate) fn own_tabs_overlap(
    settings: Option<CutSettings>,
    tabs: &LeafTabs,
    outlines: &[(Vec<PointUm>, bool)],
) -> bool {
    let (Some(settings), LeafTabs::Own(list)) = (settings, tabs) else {
        return false;
    };
    list.iter().any(|(index, own)| {
        let Some((points, closed)) = usize::try_from(*index).ok().and_then(|i| outlines.get(i))
        else {
            return false;
        };
        let measured = Measured::new(points.clone(), *closed);
        overlap(&measured, &own_spans(&measured, own, settings.tab_width))
    })
}

/// Whether the records of a leaf are damaged: its own tabs (their register, or two of them
/// overlap) or its own path options and entries (criteria 11, 22).
pub(crate) fn leaf_damaged(
    tabs: &LeafTabs,
    path: &LeafPath,
    outlines: &[LeafOutline],
    settings: Option<CutSettings>,
) -> bool {
    if path.damaged {
        return true;
    }
    match tabs {
        LeafTabs::Damaged => true,
        LeafTabs::Operation => false,
        LeafTabs::Own(_) => {
            let flat: Vec<(Vec<PointUm>, bool)> = outlines
                .iter()
                .map(|o| {
                    let points = flat(o).unwrap_or_default();
                    let closed = o.closed && points.len() >= 3;
                    (points, closed)
                })
                .collect();
            own_tabs_overlap(settings, tabs, &flat)
        }
    }
}

/// The contour units of the Cut-out items; an item with no contour is listed in `no_contour`,
/// an item whose records are damaged in `cuts.damaged`.
pub(crate) fn cut_out_units(
    block: &BlockInput,
    main: &mut Units,
    no_contour: &mut Vec<NodeId>,
    cuts: &mut Cuts,
) {
    let margin = match &block.source {
        BlockSource::Operation { margin, .. } => *margin,
        BlockSource::Fixture { .. } => curvyo_document_core::Length::from_mm(0.0),
    };
    for item in &block.cut_out {
        let rings: Vec<(Vec<PointUm>, bool)> = cut_out_outlines(&item.leaves, margin)
            .into_iter()
            .map(|points| (points.unwrap_or_default(), true))
            .collect();
        if matches!(item.cut, LeafTabs::Damaged)
            || (cuts.settings.is_some() && item.path.damaged)
            || own_tabs_overlap(cuts.settings, &item.cut, &rings)
        {
            cuts.damaged.push(item.item);
            continue;
        }
        let mut any = false;
        for (index, (points, _)) in rings.into_iter().enumerate() {
            if points.len() < 3 {
                continue;
            }
            any = true;
            let outline = index_u32(index);
            let cut = unit_cut(cuts, &item.cut, &item.path, outline, &points, true);
            main.push((
                item.item,
                Unit {
                    shape: UnitShape::Path {
                        points,
                        closed: true,
                    },
                    stacking: item.stacking,
                    outline,
                    cut,
                },
            ));
        }
        if !any {
            no_contour.push(item.item);
        }
    }
}
