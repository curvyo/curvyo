//! The one rule of what the active page edits (`specs/0106-move-versus-edit` criterion 1,
//! `adrs.md` decision 2): an object's home is read from the membership of ADR 0043 §1 only.
//!
//! An object is **in context** when the active page edits it; every other object is only
//! selected and moved. The session maps each page's kind to a [`ContextKind`] through its
//! workbench, so a laser page whose extension is off or a page of an unknown kind reads as a
//! drawing page here as it does for the tools.

use curvyo_document_core::{NodeId, Page, Point};

use crate::{PageMembership, Reach};

/// What a page edits, as its workbench says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextKind {
    /// A drawing page (also a laser page whose extension is off and an unknown kind).
    Drawing,
    /// A laser page whose extension is on.
    Laser,
    /// A sketch page.
    Sketch,
}

/// The pages of the document with their context kinds, in list order.
#[derive(Debug, Clone, Default)]
pub struct ContextPages {
    pages: Vec<(Page, ContextKind)>,
}

impl ContextPages {
    /// `pages` in list order, each with the kind `kind_of` gives it.
    #[must_use]
    pub fn new(pages: Vec<Page>, kind_of: impl Fn(&Page) -> ContextKind) -> Self {
        Self {
            pages: pages
                .into_iter()
                .map(|page| {
                    let kind = kind_of(&page);
                    (page, kind)
                })
                .collect(),
        }
    }

    /// The context kind of the page `id`.
    #[must_use]
    pub fn kind_of(&self, id: &str) -> Option<ContextKind> {
        self.pages.iter().find(|(p, _)| p.id == id).map(|(_, k)| *k)
    }

    /// The name of the page `id`.
    #[must_use]
    pub fn name_of(&self, id: &str) -> Option<&str> {
        self.pages
            .iter()
            .find(|(p, _)| p.id == id)
            .map(|(p, _)| p.name.as_str())
    }

    /// The first drawing page in list order.
    #[must_use]
    pub fn first_drawing(&self) -> Option<&str> {
        self.pages
            .iter()
            .find(|(_, k)| *k == ContextKind::Drawing)
            .map(|(p, _)| p.id.as_str())
    }

    /// The home of the object `id` with the bounds `bounds` (`adrs.md` decision 2): a page's
    /// sketch belongs to its sketch page; an object reaching a laser page belongs to the first
    /// such page (a laser page wins over a drawing page, `0100` criterion 5); else to every
    /// drawing page it reaches; else it lies on the pasteboard.
    #[must_use]
    pub fn home_of(&self, membership: &PageMembership, id: NodeId, bounds: (Point, Point)) -> Home {
        let mut drawings = Vec::new();
        for (page, kind) in &self.pages {
            if !membership.is_member(page, id, bounds, Reach::Touches) {
                continue;
            }
            match kind {
                ContextKind::Sketch => return Home::Sketch(page.id.clone()),
                ContextKind::Laser => return Home::Laser(page.id.clone()),
                ContextKind::Drawing => drawings.push(page.id.clone()),
            }
        }
        if drawings.is_empty() {
            Home::Pasteboard
        } else {
            Home::Drawing(drawings)
        }
    }
}

/// Where an object belongs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Home {
    /// A member of no page (a drawing object over a sketch page included).
    Pasteboard,
    /// A member of these drawing pages, in list order (at least one).
    Drawing(Vec<String>),
    /// An own item, clone or placed page of this laser page.
    Laser(String),
    /// The sketch of this sketch page.
    Sketch(String),
}

/// The active page: its id and context kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePage<'a> {
    /// The page id.
    pub id: &'a str,
    /// What it edits.
    pub kind: ContextKind,
}

/// Whether the active page edits an object whose home is `home` (criterion 1's table). An object
/// reaching drawing pages A and B is in context on either (Proposal).
#[must_use]
pub fn in_context(active: ActivePage<'_>, home: &Home) -> bool {
    match (active.kind, home) {
        (ContextKind::Drawing, Home::Pasteboard) => true,
        (ContextKind::Drawing, Home::Drawing(pages)) => pages.iter().any(|p| p == active.id),
        (ContextKind::Laser, Home::Laser(page)) | (ContextKind::Sketch, Home::Sketch(page)) => {
            page == active.id
        }
        _ => false,
    }
}

/// What a selection may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every object is in context: every command as built.
    Edit,
    /// At least one object is out of context: move commands only (Frage 165 A).
    MoveOnly,
}

/// The scope of a selection whose objects have `homes`; an empty selection is [`Scope::Edit`].
pub fn scope<'h>(active: ActivePage<'_>, homes: impl IntoIterator<Item = &'h Home>) -> Scope {
    if homes.into_iter().all(|home| in_context(active, home)) {
        Scope::Edit
    } else {
        Scope::MoveOnly
    }
}

/// What a double-click on an object does first (criteria 13 to 16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoubleClickTarget {
    /// The object is in context: the double-click acts as built.
    AsBuilt,
    /// This page becomes active and nothing else happens.
    Activate(String),
    /// A pasteboard object with no drawing page in the project: nothing is activated.
    NoDrawingPage,
}

/// The page a double-click on an object at `home` activates: its own page (the first drawing
/// page of several, Proposal), for a pasteboard object the last active drawing page while it
/// exists as a drawing page, else the first drawing page in list order.
#[must_use]
pub fn double_click_target(
    active: ActivePage<'_>,
    home: &Home,
    last_drawing: Option<&str>,
    pages: &ContextPages,
) -> DoubleClickTarget {
    if in_context(active, home) {
        return DoubleClickTarget::AsBuilt;
    }
    match home {
        Home::Drawing(drawings) => drawings.first().map_or(DoubleClickTarget::AsBuilt, |p| {
            DoubleClickTarget::Activate(p.clone())
        }),
        Home::Laser(page) | Home::Sketch(page) => DoubleClickTarget::Activate(page.clone()),
        Home::Pasteboard => last_drawing
            .filter(|id| pages.kind_of(id) == Some(ContextKind::Drawing))
            .or_else(|| pages.first_drawing())
            .map_or(DoubleClickTarget::NoDrawingPage, |p| {
                DoubleClickTarget::Activate(p.to_owned())
            }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use curvyo_document_core::{DocumentSize, PageGrid, PageKind};

    use super::*;

    fn node(counter: i32) -> NodeId {
        serde_json::from_value(serde_json::json!({"peer": "1", "counter": counter})).unwrap()
    }

    fn page(id: &str, x: f64, kind: PageKind) -> Page {
        Page {
            id: id.into(),
            name: id.into(),
            origin: Point::new(x, 0.0),
            size: DocumentSize::from_mm(100.0, 100.0),
            grid: PageGrid::DEFAULT,
            kind,
        }
    }

    fn laser_kind() -> PageKind {
        PageKind::Output {
            manufacturing: node(99),
            process: "curvyo.laser".to_owned(),
        }
    }

    /// Criterion 1's project: drawing pages A and B, laser page L right of B, sketch page S.
    fn project() -> ContextPages {
        ContextPages::new(
            vec![
                page("A", 0.0, PageKind::Drawing),
                page("B", 200.0, PageKind::Drawing),
                page("L", 300.0, laser_kind()),
                page("S", 500.0, PageKind::Sketch),
            ],
            |p| match p.kind {
                PageKind::Sketch => ContextKind::Sketch,
                PageKind::Output { .. } => ContextKind::Laser,
                _ => ContextKind::Drawing,
            },
        )
    }

    fn rect(x: f64) -> (Point, Point) {
        (Point::new(x, 10.0), Point::new(x + 20.0, 30.0))
    }

    /// The seven objects of criterion 1's test and their bounds.
    fn objects() -> Vec<(&'static str, NodeId, (Point, Point))> {
        vec![
            ("a", node(1), rect(10.0)),
            ("b", node(2), rect(210.0)),
            ("l", node(3), rect(330.0)),
            ("sketch", node(4), rect(520.0)),
            (
                "paste",
                node(5),
                (Point::new(10.0, -300.0), Point::new(20.0, -290.0)),
            ),
            ("over_s", node(6), rect(560.0)),
            ("over_bl", node(7), rect(290.0)),
        ]
    }

    fn in_context_names(active: &str) -> Vec<&'static str> {
        let pages = project();
        let membership = PageMembership::from_owners([(node(4), "S".to_owned())]);
        let active = ActivePage {
            id: active,
            kind: pages.kind_of(active).unwrap(),
        };
        objects()
            .into_iter()
            .filter(|(_, id, bounds)| in_context(active, &pages.home_of(&membership, *id, *bounds)))
            .map(|(name, _, _)| name)
            .collect()
    }

    #[test]
    fn criterion_one_table_for_each_active_page() {
        assert_eq!(in_context_names("A"), ["a", "paste", "over_s"]);
        assert_eq!(in_context_names("B"), ["b", "paste", "over_s"]);
        assert_eq!(in_context_names("L"), ["l", "over_bl"]);
        assert_eq!(in_context_names("S"), ["sketch"]);
    }

    #[test]
    fn homes_follow_the_one_membership_read() {
        let pages = project();
        let membership = PageMembership::from_owners([(node(4), "S".to_owned())])
            .with_placements([(node(8), "L".to_owned())]);
        let home = |id: i32, bounds| pages.home_of(&membership, node(id), bounds);
        assert_eq!(home(1, rect(10.0)), Home::Drawing(vec!["A".into()]));
        assert_eq!(home(7, rect(290.0)), Home::Laser("L".into()));
        assert_eq!(home(4, rect(520.0)), Home::Sketch("S".into()));
        assert_eq!(home(6, rect(560.0)), Home::Pasteboard);
        // A placed page is its laser page's by ownership, wherever its frame lies.
        assert_eq!(home(8, rect(10.0)), Home::Laser("L".into()));
        let across = (Point::new(90.0, 10.0), Point::new(210.0, 20.0));
        assert_eq!(home(9, across), Home::Drawing(vec!["A".into(), "B".into()]));
    }

    #[test]
    fn a_selection_with_one_object_out_of_context_is_move_only() {
        let a = ActivePage {
            id: "A",
            kind: ContextKind::Drawing,
        };
        let on_a = Home::Drawing(vec!["A".into()]);
        let on_b = Home::Drawing(vec!["B".into()]);
        assert_eq!(scope(a, [&on_a, &Home::Pasteboard]), Scope::Edit);
        assert_eq!(scope(a, [&on_a, &on_b]), Scope::MoveOnly);
        assert_eq!(scope(a, []), Scope::Edit);
    }

    #[test]
    fn double_click_targets() {
        let pages = project();
        let laser = ActivePage {
            id: "L",
            kind: ContextKind::Laser,
        };
        let a = ActivePage {
            id: "A",
            kind: ContextKind::Drawing,
        };
        let on_b = Home::Drawing(vec!["B".into(), "A".into()]);
        assert_eq!(
            double_click_target(laser, &on_b, None, &pages),
            DoubleClickTarget::Activate("B".into())
        );
        assert_eq!(
            double_click_target(a, &on_b, None, &pages),
            DoubleClickTarget::AsBuilt
        );
        assert_eq!(
            double_click_target(a, &Home::Laser("L".into()), None, &pages),
            DoubleClickTarget::Activate("L".into())
        );
        // A pasteboard object: the last active drawing page, else the first drawing page.
        assert_eq!(
            double_click_target(laser, &Home::Pasteboard, Some("B"), &pages),
            DoubleClickTarget::Activate("B".into())
        );
        assert_eq!(
            double_click_target(laser, &Home::Pasteboard, Some("gone"), &pages),
            DoubleClickTarget::Activate("A".into())
        );
        assert_eq!(
            double_click_target(laser, &Home::Pasteboard, Some("L"), &pages),
            DoubleClickTarget::Activate("A".into())
        );
        assert_eq!(
            double_click_target(a, &Home::Pasteboard, None, &pages),
            DoubleClickTarget::AsBuilt
        );
        let no_drawing =
            ContextPages::new(vec![page("L", 0.0, laser_kind())], |_| ContextKind::Laser);
        assert_eq!(
            double_click_target(laser, &Home::Pasteboard, None, &no_drawing),
            DoubleClickTarget::NoDrawingPage
        );
    }
}
