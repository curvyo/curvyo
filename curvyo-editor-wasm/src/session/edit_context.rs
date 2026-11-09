//! The session's side of move versus edit (`specs/0106-move-versus-edit`, `adrs.md` decision 4):
//! the cached scope of the selection, the objects the edit tools may touch, the last active
//! drawing page and the double-click or Enter that makes an object's page active.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{DocumentVersion, NodeId, ObjectSnapshot, Page, Point};
use curvyo_ui_core::{
    ActivePage, ContextKind, ContextPages, DoubleClickTarget, Home, NO_DRAWING_PAGE_NOTICE,
    OutsideHomes, PageMembership, Scope, double_click_target, in_context, not_editable_notice,
    object_bounds,
};

use super::{DoubleClickHint, Session, Workbench};

/// What the cached reads were made for.
#[derive(Debug, Clone, PartialEq)]
struct Key {
    version: DocumentVersion,
    active: String,
    workbench: Workbench,
}

/// The homes of the selected objects, read once per key and selection.
#[derive(Debug, Clone)]
pub(super) struct SelectionHomes {
    /// Move-only when an object is out of context (Frage 165 A).
    pub(super) scope: Scope,
    /// The homes of the out-of-context objects, distinct, in selection order.
    pub(super) outside: Vec<Home>,
    /// The words of those homes.
    pub(super) names: OutsideHomes,
}

/// The homes of a selection, kept for the key and the selection they were read for.
type HeldScope = (Key, Vec<NodeId>, Arc<SelectionHomes>);

/// The edit targets, kept for the key and the pickable objects they were cut from.
type HeldTargets = (Key, Arc<[ObjectSnapshot]>, Arc<[ObjectSnapshot]>);

impl SelectionHomes {
    /// The homes of an empty selection: it may be edited.
    fn empty() -> Self {
        Self {
            scope: Scope::Edit,
            outside: Vec::new(),
            names: OutsideHomes {
                mixed: false,
                names: Vec::new(),
            },
        }
    }
}

/// The session's move-versus-edit state: caches and the last active drawing page, never saved.
#[derive(Debug, Default)]
pub(super) struct EditContext {
    scope: RefCell<Option<HeldScope>>,
    targets: RefCell<Option<HeldTargets>>,
    /// The drawing page most recently active in this session (criterion 14).
    last_drawing_page: Option<String>,
    /// A guarded command was refused since the key handler last looked.
    pub(super) refused: bool,
    /// A Select drag of drawing objects runs on a sketch page (criterion 7).
    pub(super) sketch_drawing_drag: bool,
    /// An empty Select press inside a frame: the page it activates on a click's release.
    pub(super) empty_press: Option<Point>,
    /// The notice for the canvas slot, until the host takes it.
    notice: Option<String>,
}

impl Session {
    /// The context kind of a page of `kind`, as its workbench reads it.
    fn context_kind(&self, kind: &curvyo_document_core::PageKind) -> ContextKind {
        match self.workbench_of_kind(kind) {
            Workbench::Vector => ContextKind::Drawing,
            Workbench::Sketch => ContextKind::Sketch,
            Workbench::Manufacturing(_) => ContextKind::Laser,
        }
    }

    /// The pages with their context kinds and the one membership read with the placed pages.
    pub(super) fn context_read(&self) -> (ContextPages, PageMembership) {
        let pages = self.document.pages();
        let membership = PageMembership::of(&self.document, &pages).with_placements(
            self.document
                .placed_pages()
                .into_iter()
                .map(|placed| (placed.id, placed.output.id)),
        );
        let pages = ContextPages::new(pages, |page: &Page| self.context_kind(&page.kind));
        (pages, membership)
    }

    fn context_key(&self) -> Key {
        // The stored active page is read by its own register; only a page that is gone needs
        // the page list's fallback (the `0098` budgets of a frame with hundreds of pages).
        let active = match &self.page_view.active {
            Some(id) if self.document.page_kind(id).is_some() => id.clone(),
            _ => self.active_page().id,
        };
        Key {
            version: self.document.version(),
            active,
            workbench: self.workbench(),
        }
    }

    /// The homes of the selection, cached per document version, active page and selection
    /// (criterion 22 (a)).
    pub(super) fn selection_homes(&self) -> Arc<SelectionHomes> {
        if self.selection.ids().is_empty() {
            return Arc::new(SelectionHomes::empty());
        }
        let key = self.context_key();
        let selected = self.selection.ids();
        if let Some((held, ids, homes)) = self.edit_context.scope.borrow().as_ref()
            && *held == key
            && ids.as_slice() == selected
        {
            return Arc::clone(homes);
        }
        let homes = Arc::new(self.read_selection_homes(&key));
        *self.edit_context.scope.borrow_mut() = Some((key, selected.to_vec(), Arc::clone(&homes)));
        homes
    }

    fn read_selection_homes(&self, key: &Key) -> SelectionHomes {
        let selected: HashSet<NodeId> = self.selection.ids().iter().copied().collect();
        if selected.is_empty() {
            return SelectionHomes::empty();
        }
        let (pages, membership) = self.context_read();
        let active = ActivePage {
            id: &key.active,
            kind: context_active(key),
        };
        let mut outside: Vec<Home> = Vec::new();
        let mut inside = false;
        for object in self.objects().iter().filter(|o| selected.contains(&o.id())) {
            let home = pages.home_of(&membership, object.id(), object_bounds(object));
            if in_context(active, &home) {
                inside = true;
            } else if !outside.contains(&home) {
                outside.push(home);
            }
        }
        let names = outside.iter().map(|home| home_name(&pages, home)).collect();
        SelectionHomes {
            scope: if outside.is_empty() {
                Scope::Edit
            } else {
                Scope::MoveOnly
            },
            outside,
            names: OutsideHomes {
                mixed: inside,
                names,
            },
        }
    }

    /// Whether the selection may be edited or only moved (criteria 1 and 4).
    pub(super) fn edit_scope(&self) -> Scope {
        self.selection_homes().scope
    }

    /// What the edit tools may touch (criterion 10): the pickable objects in context of the active
    /// page. The Select tool keeps the whole set.
    pub(super) fn edit_targets(&self) -> Arc<[ObjectSnapshot]> {
        let pickable = self.pickable_objects();
        let key = self.context_key();
        if let Some((held, from, targets)) = self.edit_context.targets.borrow().as_ref()
            && *held == key
            && Arc::ptr_eq(from, &pickable)
        {
            return Arc::clone(targets);
        }
        let (pages, membership) = self.context_read();
        let active = ActivePage {
            id: &key.active,
            kind: context_active(&key),
        };
        let keep = |object: &ObjectSnapshot| {
            in_context(
                active,
                &pages.home_of(&membership, object.id(), object_bounds(object)),
            )
        };
        let targets: Arc<[ObjectSnapshot]> = if pickable.iter().all(keep) {
            Arc::clone(&pickable)
        } else {
            pickable.iter().filter(|o| keep(o)).cloned().collect()
        };
        *self.edit_context.targets.borrow_mut() = Some((key, pickable, Arc::clone(&targets)));
        targets
    }

    /// Records the active page as the last active drawing page when it is one (criterion 14).
    pub(super) fn note_active_drawing_page(&mut self) {
        // The page's own kind register: no decode of the page list (`0098` page budgets).
        let Some(id) = self.page_view.active.clone() else {
            return;
        };
        if let Some(kind) = self.document.page_kind(&id)
            && self.context_kind(&kind) == ContextKind::Drawing
        {
            self.edit_context.last_drawing_page = Some(id);
        }
    }

    /// The refusal of an edit of the selection while it is move-only (criterion 9), `None` while
    /// it may be edited.
    pub(super) fn not_editable(&self) -> Option<String> {
        let homes = self.selection_homes();
        (homes.scope == Scope::MoveOnly).then(|| not_editable_notice(&homes.names))
    }

    /// The notice of a refused edit chord (criterion 9), set when a guarded command was refused
    /// during the key just handled.
    pub(super) fn note_refused_key(&mut self, repeat: bool) {
        if !std::mem::take(&mut self.edit_context.refused) || repeat {
            return;
        }
        let homes = self.selection_homes();
        if homes.scope == Scope::MoveOnly {
            self.edit_context.notice = Some(not_editable_notice(&homes.names));
        }
    }

    /// The canvas notice of this slice, once: a refused edit chord or a double-click with no
    /// drawing page.
    pub fn take_context_notice(&mut self) -> String {
        self.edit_context.notice.take().unwrap_or_default()
    }

    /// The page a double-click or Enter activates for `home`.
    fn target_of(&self, home: &Home) -> DoubleClickTarget {
        let key = self.context_key();
        let (pages, _) = self.context_read();
        double_click_target(
            ActivePage {
                id: &key.active,
                kind: context_active(&key),
            },
            home,
            self.edit_context.last_drawing_page.as_deref(),
            &pages,
        )
    }

    /// Activates `target`; the selection stays and the view does not move (criteria 13 to 16).
    fn switch_to(&mut self, target: &DoubleClickTarget) -> bool {
        match target {
            DoubleClickTarget::AsBuilt => false,
            DoubleClickTarget::Activate(page) => {
                let selected = self.selection.ids().to_vec();
                self.activate_page_in_place(page);
                self.selection.set(&selected);
                true
            }
            DoubleClickTarget::NoDrawingPage => {
                self.edit_context.notice = Some(NO_DRAWING_PAGE_NOTICE.to_owned());
                true
            }
        }
    }

    /// The first part of every double-click (criteria 13 to 16): on an out-of-context object the
    /// object's page becomes active and nothing else happens; on a placed page of the active
    /// laser page its source page becomes active with the object under the pointer selected.
    /// `None` when the double-click acts as built.
    pub(super) fn double_click_switch(&mut self, point: Point) -> Option<DoubleClickHint> {
        let pickable = self.pickable_objects();
        let hit = curvyo_ui_core::hit_test_objects(&pickable, point, self.object_tolerance());
        // The first press of the pair selected the object meant; prefer it.
        let &top = hit
            .iter()
            .find(|&&id| self.selection.contains(id))
            .or_else(|| hit.first())?;
        let object = pickable.iter().find(|o| o.id() == top)?;
        // A clone of a page's sketch opens its sketch page, as built (`0099` criterion 13).
        if self.is_sketch_clone(object) {
            return None;
        }
        let (pages, membership) = self.context_read();
        let home = pages.home_of(&membership, top, object_bounds(object));
        let target = self.target_of(&home);
        if target == DoubleClickTarget::AsBuilt {
            return self
                .open_placed_page_source(top, point)
                .then_some(DoubleClickHint::None);
        }
        if !self.selection.contains(top) {
            let members = self.units().leaves_with(&top).to_vec();
            self.selection.set(&members);
        }
        self.switch_to(&target).then_some(DoubleClickHint::None)
    }

    /// Whether the Select tool's box, handles and live edit are drawn: in the Select tool, and in
    /// the Node tool for a move-only selection, which it moves as Select does (criterion 10).
    pub(super) fn select_box_tool(&self) -> bool {
        match self.tool {
            super::Tool::Select => true,
            super::Tool::Node => self.selection_move_only(),
            _ => false,
        }
    }

    /// A Node tool press on an out-of-context object selects it as Select does and shows its box
    /// (criterion 10); a press on a move-only selection's box does nothing. The Node tool starts
    /// no drag on either, so nothing is written. `false` when the Node tool keeps the press.
    pub(super) fn node_press_as_select(&mut self, point: Point) -> bool {
        let pickable = self.pickable_objects();
        let hit = curvyo_ui_core::hit_test_objects(&pickable, point, self.object_tolerance());
        if let Some(&top) = hit.first()
            && !self.edit_targets().iter().any(|o| o.id() == top)
        {
            let members = self.units().leaves_with(&top).to_vec();
            self.selection.set(&members);
            return true;
        }
        self.selection_move_only() && self.press_reaches_selection_box(point)
    }

    /// Enter with a move-only selection (UX notes §4): one home is made active; several homes
    /// show the refusal notice. `false` when the selection may be edited.
    pub(super) fn enter_switch(&mut self) -> bool {
        let homes = self.selection_homes();
        if homes.scope == Scope::Edit {
            return false;
        }
        match homes.outside.as_slice() {
            [home] => {
                let target = self.target_of(home);
                self.switch_to(&target);
            }
            _ => self.edit_context.notice = Some(not_editable_notice(&homes.names)),
        }
        true
    }

    /// Whether `object` is a linked clone whose root is a page's sketch.
    fn is_sketch_clone(&self, object: &ObjectSnapshot) -> bool {
        matches!(object, ObjectSnapshot::Clone(clone) if self.document.is_fixed_sketch(clone.chain.root))
    }

    /// A double-click on a placed page of the active laser page (criterion 15): its source page
    /// becomes active and the topmost source object under the pointer is selected, the view
    /// panning only when it is out of view; an empty frame area selects nothing.
    fn open_placed_page_source(&mut self, id: NodeId, point: Point) -> bool {
        let Some(placed) = self.document.placed_page(id) else {
            return false;
        };
        let Some(source) = placed.source.clone() else {
            return false;
        };
        let Some(inverse) = placed
            .global_map()
            .and_then(curvyo_document_core::Affine::inverse)
        else {
            return false;
        };
        let at = inverse.apply_point(point);
        self.selection.clear();
        self.activate_page_in_place(&source.id);
        let pickable = self.pickable_objects();
        let hit = curvyo_ui_core::hit_test_objects(&pickable, at, self.object_tolerance());
        if let Some(&top) = hit.first() {
            let members = self.units().leaves_with(&top).to_vec();
            self.selection.set(&members);
            self.reveal_point(at);
        }
        true
    }

    /// Pans so that `point` is at the centre of the view when it is outside it; never zooms.
    fn reveal_point(&mut self, point: Point) {
        let (w, h) = self.viewport.canvas_size();
        let (x, y) = self.view().document_to_screen(point);
        if !(0.0..=w).contains(&x) || !(0.0..=h).contains(&y) {
            self.viewport.pan_by_screen_delta(x - w / 2.0, y - h / 2.0);
        }
    }
}

/// How the line names `home`: its page's name, "Pasteboard" for the pasteboard.
fn home_name(pages: &ContextPages, home: &Home) -> String {
    let id = match home {
        Home::Pasteboard => return "Pasteboard".to_owned(),
        Home::Drawing(ids) => ids.first().map_or("", String::as_str),
        Home::Laser(id) | Home::Sketch(id) => id.as_str(),
    };
    pages.name_of(id).unwrap_or(id).to_owned()
}

/// The active page of `key` as the context rule reads it.
fn context_active(key: &Key) -> ContextKind {
    match key.workbench {
        Workbench::Vector => ContextKind::Drawing,
        Workbench::Sketch => ContextKind::Sketch,
        Workbench::Manufacturing(_) => ContextKind::Laser,
    }
}
