//! The tabs the path units of one block take (`specs/0139` criteria 3 to 5, 11, adrs L7): the
//! block's cut settings, each outline's tabs, and the damage of an item's own tabs.

use curvyo_document_core::NodeId;

use crate::cut_item_records::LeafTabs;
use crate::cut_tabs::{Measured, overlap, own_spans};
use crate::job_cut_path::UnitCut;
use crate::job_flatten::flat;
use crate::job_input::{BlockInput, BlockSource, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_plan::CutMark;
use crate::operation_cut_options::{CutSettings, cuts_through};

/// What the tabs of one block need and leave (`specs/0139`).
#[derive(Debug, Default)]
pub(crate) struct Cuts {
    /// The block's cut settings; `None` for a block whose kind takes no tabs, a fixture block or
    /// a test grid's derived operation (adrs L7).
    pub(crate) settings: Option<CutSettings>,
    pub(crate) marks: Vec<CutMark>,
    pub(crate) covered: Vec<NodeId>,
    pub(crate) damaged: Vec<NodeId>,
    pub(crate) placed: Vec<usize>,
}

/// The cut settings of `block` when its paths may get tabs.
pub(crate) fn block_cut(block: &BlockInput, numbers: &BlockNumbers) -> Option<CutSettings> {
    let BlockSource::Operation { grid: None, .. } = &block.source else {
        return None;
    };
    cuts_through(numbers.kind).then_some(numbers.options.cut)
}

/// The tabs of outline `index` (flattened to `points`) of a leaf or item with tabs `tabs`;
/// `None` when it runs as today (no own tabs and Count 0).
pub(crate) fn unit_cut(
    settings: Option<CutSettings>,
    tabs: &LeafTabs,
    index: u32,
    points: &[PointUm],
    closed: bool,
) -> Option<Box<UnitCut>> {
    let settings = settings?;
    let own = tabs.own(index);
    if own.is_none_or(<[_]>::is_empty) && (own.is_some() || settings.tab_count.get() == 0) {
        return None;
    }
    let measured = Measured::new(points.to_vec(), closed);
    let own = own.map(|tabs| own_spans(&measured, tabs, settings.tab_width));
    Some(Box::new(UnitCut {
        measured,
        own,
        settings,
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

/// Whether the own tabs of a leaf are damaged: its register, or two of them overlap.
pub(crate) fn leaf_damaged(
    tabs: &LeafTabs,
    outlines: &[LeafOutline],
    settings: Option<CutSettings>,
) -> bool {
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
