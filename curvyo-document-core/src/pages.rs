//! Ordered spatial page frames stored as root registers, each frame one register.
use crate::document::{KEY_HEIGHT_MM, KEY_WIDTH_MM, ROOT_MAP};
use crate::document_size::{MAX_DOCUMENT_MM, MIN_DOCUMENT_MM, validated_document_side};
use crate::page_frame::{Frame, read_frame, write_frame};
use crate::page_grid::{PageGrid, grid_of_fields, grid_of_root, write_non_default};
use crate::page_kind::PageKind;
use crate::{Document, DocumentSize, DocumentSizeError, NodeId, ObjectEditError, Point, Vec2};
use loro::{LoroMap, LoroTree, LoroValue, ValueOrContainer};
use serde::Serialize;

/// Stable identity of the initial page.
pub const FIRST_PAGE_ID: &str = "first";
const EPS: f64 = 1e-9;

/// A named frame in the shared global drawing plane.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    /// Stable identity, independent of list position.
    pub id: String,
    /// User-visible name.
    pub name: String,
    /// Global top-left corner in millimetres.
    pub origin: Point,
    /// Frame dimensions.
    pub size: DocumentSize,
    /// The page's grid settings (`specs/0090-grid` criterion 1); left out of
    /// `document.json` at its default, like a node's registers.
    #[serde(skip_serializing_if = "PageGrid::is_default")]
    pub grid: PageGrid,
    /// What the page is (ADR 0042); fixed from its creation. Left out of `document.json` for a
    /// drawing page, so a file without page kinds exports as before.
    #[serde(skip_serializing_if = "PageKind::is_drawing")]
    pub kind: PageKind,
}
impl Page {
    /// Whether a global point lies within the frame, including its edge.
    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x - EPS
            && point.y >= self.origin.y - EPS
            && point.x <= self.origin.x + self.size.width.as_mm() + EPS
            && point.y <= self.origin.y + self.size.height.as_mm() + EPS
    }
    /// Whether an axis-aligned box touches or overlaps this frame.
    #[must_use]
    pub fn overlaps(&self, min: Point, max: Point) -> bool {
        max.x >= self.origin.x - EPS
            && max.y >= self.origin.y - EPS
            && min.x <= self.origin.x + self.size.width.as_mm() + EPS
            && min.y <= self.origin.y + self.size.height.as_mm() + EPS
    }
}
/// A refused page command; it writes nothing.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    /// The page identity or target index is invalid.
    #[error("page not found")]
    NotFound,
    /// At least one page must remain.
    #[error("the last page cannot be deleted")]
    LastPage,
    /// Page names must contain non-whitespace text.
    #[error("the page name is empty")]
    EmptyName,
    /// Invalid geometry.
    #[error(transparent)]
    Size(#[from] DocumentSizeError),
    /// Invalid content movement.
    #[error(transparent)]
    Object(#[from] ObjectEditError),
    /// The page's size comes from its machine or its kind is unknown (ADR 0042 §3).
    #[error("this page's size cannot be changed here")]
    FixedSize,
}
pub(crate) fn key(id: &str, field: &str) -> String {
    match (id, field) {
        (FIRST_PAGE_ID, "width") => KEY_WIDTH_MM.to_owned(),
        (FIRST_PAGE_ID, "height") => KEY_HEIGHT_MM.to_owned(),
        _ => format!("page.{id}.{field}"),
    }
}
fn number(root: &LoroMap, id: &str, field: &str) -> Option<f64> {
    match root.get(&key(id, field))?.get_deep_value() {
        LoroValue::Double(n) if n.is_finite() => Some(n),
        _ => None,
    }
}
fn string(root: &LoroMap, id: &str, field: &str) -> Option<String> {
    match root.get(&key(id, field))?.get_deep_value() {
        LoroValue::String(s) => Some(s.to_string()),
        _ => None,
    }
}
pub(crate) fn write(root: &LoroMap, id: &str, field: &str, value: impl Into<LoroValue>) {
    // invariant: attached root map, primitive register values only.
    #[allow(clippy::unwrap_used)]
    root.insert(&key(id, field), value).unwrap();
}
pub(crate) fn initialize(root: &LoroMap) {
    write(root, FIRST_PAGE_ID, "name", "Page 1");
    for field in ["x", "y", "order"] {
        write(root, FIRST_PAGE_ID, field, 0.0);
    }
}
/// Writes the pages of a drawing that becomes a new document, without a commit: the first page
/// takes the first frame's name and corner, every further frame becomes a page after it
/// (`specs/0024-svg-import-export` criterion 129).
pub(crate) fn write_drawing_pages(
    document: &Document,
    frames: &[crate::drawing_insert::DrawingPageFrame],
) {
    let root = document.loro().get_map(ROOT_MAP);
    let peer = document.loro().peer_id();
    for (index, frame) in frames.iter().enumerate() {
        let id = if index == 0 {
            FIRST_PAGE_ID.to_owned()
        } else {
            format!("{peer}-svg{index}")
        };
        write(&root, &id, "name", frame.name.clone());
        #[allow(clippy::cast_precision_loss)] // page counts are far below 2^52
        let order = index as f64;
        write(&root, &id, "order", order);
        write_frame(
            &root,
            &id,
            Frame {
                origin: frame.origin,
                size: frame.size,
            },
        );
    }
}

/// The frame of page `id`: its register, else (a version 33 page, or a damaged register the open
/// check let through) the four old registers with the size falling back to A4 on both axes.
fn frame_of(root: &LoroMap, id: &str) -> Option<Frame> {
    if let Some(frame) = read_frame(root, id) {
        return Some(frame);
    }
    // Persisted values are validated strictly: command tolerance must not admit damaged sizes.
    // Keep the frame and fall back on both axes, without repairing or otherwise writing the
    // stored registers.
    let valid = |field| {
        number(root, id, field).filter(|mm| (MIN_DOCUMENT_MM..=MAX_DOCUMENT_MM).contains(mm))
    };
    let size = match (valid("width"), valid("height")) {
        (Some(width), Some(height)) => DocumentSize::from_mm(width, height),
        _ => DocumentSize::default(),
    };
    Some(Frame {
        origin: Point::new(number(root, id, "x")?, number(root, id, "y")?),
        size,
    })
}

/// The registered grid fields of one page, by field name.
type GridFields = Vec<(String, LoroValue)>;

/// The registers of one page record that `pages()` reads in its one pass over the root.
#[derive(Default)]
struct PageRecord {
    named: bool,
    grid: GridFields,
    kind: Option<LoroValue>,
    owner: Option<LoroValue>,
}

/// Every page record, in one pass over the root: the grid registers are gathered in the same
/// pass, so a page without any (a grid never touched) costs no keyed read per frame
/// (`specs/0090-grid` criterion 9), and so are the kind and owner of ADR 0042 §2.
fn page_records(root: &LoroMap) -> std::collections::BTreeMap<String, PageRecord> {
    let mut records: std::collections::BTreeMap<String, PageRecord> =
        std::collections::BTreeMap::new();
    root.for_each(|name, value| {
        let Some(rest) = name.strip_prefix("page.") else {
            return;
        };
        let Some((id, field)) = rest.rsplit_once('.') else {
            return;
        };
        let ValueOrContainer::Value(value) = value else {
            if field == "name" {
                records.entry(id.to_owned()).or_default().named = true;
            }
            return;
        };
        let record = records.entry(id.to_owned()).or_default();
        match field {
            "name" => record.named = true,
            crate::page_kind::KIND_FIELD => record.kind = Some(value),
            crate::manufacturing_codec::PAGE_OWNER_FIELD => record.owner = Some(value),
            grid if grid.starts_with("grid_") => record.grid.push((grid.to_owned(), value)),
            _ => {}
        }
    });
    records.retain(|_, record| record.named);
    records
}

/// Every live page of every kind in list order (ADR 0042 §4); reading never writes.
pub(crate) fn pages_of_root(root: &LoroMap, tree: &LoroTree) -> Vec<Page> {
    let records = page_records(root);
    let mut pages: Vec<_> = records
        .into_iter()
        .filter_map(|(id, record)| {
            let frame = frame_of(root, &id)?;
            let page = Page {
                name: string(root, &id, "name")?,
                origin: frame.origin,
                size: frame.size,
                grid: if record.grid.is_empty() {
                    PageGrid::DEFAULT
                } else {
                    grid_of_fields(|field| {
                        record
                            .grid
                            .iter()
                            .find(|(name, _)| name == field)
                            .map(|(_, value)| value.clone())
                    })
                },
                kind: crate::page_kind::read_kind(
                    record.kind.as_ref(),
                    record.owner.as_ref(),
                    tree,
                ),
                id,
            };
            Some(page)
        })
        .collect();
    pages.sort_by(|a, b| {
        number(root, &a.id, "order")
            .unwrap_or(0.0)
            .total_cmp(&number(root, &b.id, "order").unwrap_or(0.0))
            .then(a.id.cmp(&b.id))
    });
    let fallback = pages.first().cloned();
    pages.retain(|p| {
        root.get(&key(&p.id, "deleted"))
            .is_none_or(|v| v.get_deep_value() != LoroValue::Bool(true))
    });
    if pages.is_empty() {
        // Concurrent deletes may tombstone every frame; deterministically expose one.
        pages.push(fallback.unwrap_or_else(|| first_page_of_old_file(root)));
    }
    pages
}

/// The one page of a file older than pages (format 26), which names none; a resize since wrote
/// a frame.
fn first_page_of_old_file(root: &LoroMap) -> Page {
    let frame = read_frame(root, FIRST_PAGE_ID).unwrap_or(Frame {
        origin: Point::new(0.0, 0.0),
        size: Document::size_of_root(root),
    });
    Page {
        id: FIRST_PAGE_ID.to_owned(),
        name: "Page 1".into(),
        origin: frame.origin,
        size: frame.size,
        grid: grid_of_root(root, FIRST_PAGE_ID),
        kind: PageKind::Drawing,
    }
}

/// The output page `id` that `manufacturing` owns, `None` when it is missing, deleted or owned by
/// another node.
pub(crate) fn owned_page(root: &LoroMap, tree: &LoroTree, id: &str) -> Option<Page> {
    if root
        .get(&key(id, "deleted"))
        .is_some_and(|v| v.get_deep_value() == LoroValue::Bool(true))
    {
        return None;
    }
    let frame = frame_of(root, id)?;
    let owner = root.get(&key(id, crate::manufacturing_codec::PAGE_OWNER_FIELD));
    let owner = match owner? {
        ValueOrContainer::Value(value) => value,
        ValueOrContainer::Container(_) => return None,
    };
    Some(Page {
        id: id.to_owned(),
        name: string(root, id, "name")?,
        origin: frame.origin,
        size: frame.size,
        grid: grid_of_root(root, id),
        kind: crate::page_kind::read_kind(None, Some(&owner), tree),
    })
}

/// The label of Add page with Kind Sketch (`specs/0098-page-kinds` criterion 3).
pub const ADD_SKETCH_PAGE_COMMIT_LABEL: &str = "add_sketch_page";

/// "`prefix` n" with the smallest n from 1 that no page of `pages` is named.
pub(crate) fn next_free_name(pages: &[Page], prefix: &str) -> String {
    let mut n = 1;
    while pages.iter().any(|p| p.name == format!("{prefix} {n}")) {
        n += 1;
    }
    format!("{prefix} {n}")
}

/// Where Add page puts a new page of `size` (`specs/0059-multiple-pages` criterion 3): right of
/// the last page with the gap, or the start of a new row below once the row holds 8 pages or
/// passes 4 m.
pub(crate) fn next_page_origin(pages: &[Page], size: DocumentSize) -> Point {
    let Some(last) = pages.last() else {
        return Point::new(0.0, 0.0);
    };
    let gap = 20.0_f64.max(0.1 * last.size.width.as_mm().max(last.size.height.as_mm()));
    let row: Vec<_> = pages
        .iter()
        .rev()
        .take_while(|p| (p.origin.y - last.origin.y).abs() <= EPS)
        .collect();
    let left = row.iter().map(|p| p.origin.x).fold(f64::INFINITY, f64::min);
    let origin = Point::new(last.origin.x + last.size.width.as_mm() + gap, last.origin.y);
    if row.len() >= 8 || origin.x + size.width.as_mm() - left > 4000.0 {
        return Point::new(
            left,
            row.iter()
                .map(|p| p.origin.y + p.size.height.as_mm())
                .fold(f64::NEG_INFINITY, f64::max)
                + gap,
        );
    }
    origin
}

impl Document {
    /// The kind of the live page `id` from its own registers only (no walk of the page list),
    /// `None` when it is missing or deleted: the cheap read of an activation (`specs/0098`
    /// criterion 21).
    #[must_use]
    pub fn page_kind(&self, id: &str) -> Option<PageKind> {
        let root = self.loro().get_map(ROOT_MAP);
        string(&root, id, "name")?;
        let deleted = root
            .get(&key(id, "deleted"))
            .is_some_and(|v| v.get_deep_value() == LoroValue::Bool(true));
        if deleted {
            return None;
        }
        let value = |field: &str| match root.get(&key(id, field)) {
            Some(ValueOrContainer::Value(value)) => Some(value),
            _ => None,
        };
        Some(crate::page_kind::read_kind(
            value(crate::page_kind::KIND_FIELD).as_ref(),
            value(crate::manufacturing_codec::PAGE_OWNER_FIELD).as_ref(),
            &self.loro().get_tree(crate::document::OBJECTS_TREE),
        ))
    }

    /// Frames in list order; reading never writes document state.
    #[must_use]
    pub fn pages(&self) -> Vec<Page> {
        pages_of_root(
            &self.loro().get_map(ROOT_MAP),
            &self.loro().get_tree(crate::document::OBJECTS_TREE),
        )
    }
    /// Adds a drawing page after the last listed page in one history step "Add page"
    /// (`specs/0098-page-kinds` criterion 2).
    #[allow(clippy::missing_panics_doc)] // a drawing page is always addable.
    pub fn add_page(&self) -> Page {
        // invariant: Drawing is a kind a command creates.
        #[allow(clippy::unwrap_used)]
        self.add_page_of(&PageKind::Drawing).unwrap()
    }

    /// Adds a page of `kind` (Drawing or Sketch) in one history step (criteria 2, 3): named
    /// "Page N" or "Sketch N", the size and grid of the last page of the same kind (a sketch page
    /// falls back to the last drawing page), else the default size, placed by the Add page rule.
    ///
    /// # Errors
    /// [`PageError::FixedSize`] for a kind no command creates this way (output pages come from
    /// [`Document::add_output_page`]).
    pub fn add_page_of(&self, kind: &PageKind) -> Result<Page, PageError> {
        let stored = kind.stored().ok_or(PageError::FixedSize)?;
        let pages = self.pages();
        let last_of = |wanted: &PageKind| pages.iter().rev().find(|p| p.kind == *wanted);
        let source = last_of(kind).or_else(|| last_of(&PageKind::Drawing));
        let (size, grid) = source.map_or((DocumentSize::default(), PageGrid::DEFAULT), |p| {
            (p.size, p.grid)
        });
        let prefix = if *kind == PageKind::Sketch {
            "Sketch"
        } else {
            "Page"
        };
        let page = Page {
            id: self.new_page_id(),
            name: next_free_name(&pages, prefix),
            origin: next_page_origin(&pages, size),
            size,
            grid,
            kind: kind.clone(),
        };
        let root = self.loro().get_map(ROOT_MAP);
        Self::write_new_page(&root, &pages, &page);
        write(&root, &page.id, crate::page_kind::KIND_FIELD, stored);
        write_non_default(&root, &page.id, page.grid);
        self.commit_with_label(if *kind == PageKind::Sketch {
            ADD_SKETCH_PAGE_COMMIT_LABEL
        } else {
            "add_page"
        });
        Ok(page)
    }

    /// A fresh page id from this peer's next operation counter.
    pub(crate) fn new_page_id(&self) -> String {
        let peer = self.loro().peer_id();
        let counter = self.loro().oplog_vv().get(&peer).copied().unwrap_or(0);
        let pending = self.loro().get_pending_txn_len();
        if pending == 0 {
            format!("{peer}-{counter}")
        } else {
            format!("{peer}-{counter}-{pending}")
        }
    }

    /// Writes the name, the order after the last of `pages` and the frame of `page`, uncommitted.
    pub(crate) fn write_new_page(root: &LoroMap, pages: &[Page], page: &Page) {
        if let Some(last) = pages.last() {
            write(root, &last.id, "deleted", false);
        }
        let order = pages.last().map_or(0.0, |last| {
            number(root, &last.id, "order").unwrap_or(0.0) + 1.0
        });
        write(root, &page.id, "name", page.name.clone());
        write(root, &page.id, "order", order);
        write_frame(
            root,
            &page.id,
            Frame {
                origin: page.origin,
                size: page.size,
            },
        );
    }
    fn find_page(&self, id: &str) -> Result<Page, PageError> {
        self.pages()
            .into_iter()
            .find(|p| p.id == id)
            .ok_or(PageError::NotFound)
    }
    /// Renames a page. Empty names or unknown identities write nothing.
    /// # Errors
    /// Returns a refusal for an empty name or unknown page.
    pub fn rename_page(&self, id: &str, name: &str) -> Result<bool, PageError> {
        let page = self.find_page(id)?;
        if name.trim().is_empty() {
            return Err(PageError::EmptyName);
        }
        if page.name == name {
            return Ok(false);
        }
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, id, "deleted", false);
        write(&root, id, "name", name);
        self.commit_with_label("rename_page");
        Ok(true)
    }
    /// Removes only the frame, preserving all objects; a sketch page goes with its sketch and an
    /// output page with its machine data.
    /// # Errors
    /// Refuses an unknown page or the last frame.
    pub fn delete_page(&self, id: &str) -> Result<(), PageError> {
        let page = self.find_page(id)?;
        if self.pages().len() == 1 {
            return Err(PageError::LastPage);
        }
        // A sketch page goes with its sketch; its clones are repaired (ADR 0043 §5).
        if let Some(owner) = self.sketch_to_delete_with(id, &page.kind) {
            return self.delete_sketch_page(id, owner);
        }
        write(&self.loro().get_map(ROOT_MAP), id, "deleted", true);
        // The page owns its machine data: the manufacturing goes in the same step, in one tree
        // delete, so one undo restores the page with its copy, fixtures and placements
        // (ADR 0042 §5, `specs/0098-page-kinds` criterion 16).
        if let Some(manufacturing) = page.kind.manufacturing() {
            self.remove_manufacturing_node(manufacturing)
                .map_err(|_| PageError::NotFound)?;
        }
        self.commit_with_label("delete_page");
        Ok(())
    }
    /// Moves one frame within the list without changing its geometry.
    /// # Errors
    /// Refuses an unknown page or an out-of-bounds index.
    pub fn reorder_page(&self, id: &str, index: usize) -> Result<bool, PageError> {
        let mut pages = self.pages();
        let old = pages
            .iter()
            .position(|p| p.id == id)
            .ok_or(PageError::NotFound)?;
        if index >= pages.len() {
            return Err(PageError::NotFound);
        }
        if old == index {
            return Ok(false);
        }
        let page = pages.remove(old);
        pages.insert(index, page);
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, id, "deleted", false);
        let before = index
            .checked_sub(1)
            .map(|i| number(&root, &pages[i].id, "order").unwrap_or(0.0));
        let after = pages
            .get(index + 1)
            .map(|p| number(&root, &p.id, "order").unwrap_or(0.0));
        let order = match (before, after) {
            (Some(a), Some(b)) => f64::midpoint(a, b),
            (Some(a), None) => a + 1.0,
            (None, Some(b)) => b - 1.0,
            _ => 0.0,
        };
        if !order.is_finite()
            || before.is_some_and(|a| order <= a)
            || after.is_some_and(|b| order >= b)
        {
            for (i, page) in pages.iter().enumerate() {
                #[allow(clippy::cast_precision_loss)]
                write(&root, &page.id, "order", i as f64);
            }
        } else {
            write(&root, id, "order", order);
        }
        self.commit_with_label("reorder_pages");
        Ok(true)
    }
    fn validate_page_translation(&self, objects: &[NodeId], delta: Vec2) -> Result<(), PageError> {
        let moved = objects
            .iter()
            .map(|id| {
                self.object(*id)
                    .map(|o| o.translated(delta))
                    .ok_or(ObjectEditError::NoSuchObject)
            })
            .collect::<Result<Vec<_>, _>>()?;
        crate::transform_objects::check_numbers(&moved, false)?;
        Ok(())
    }
    /// Translates a frame plus the caller's resolved contained leaves atomically.
    /// # Errors
    /// Refuses unknown identities, non-finite movement or invalid object transforms.
    pub fn move_page(&self, id: &str, delta: Vec2, objects: &[NodeId]) -> Result<bool, PageError> {
        let page = self.find_page(id)?;
        if ![
            delta.x,
            delta.y,
            page.origin.x + delta.x,
            page.origin.y + delta.y,
        ]
        .iter()
        .all(|n| n.is_finite())
        {
            return Err(DocumentSizeError::NotFinite.into());
        }
        if delta.x.abs() <= EPS && delta.y.abs() <= EPS {
            return Ok(false);
        }
        self.validate_page_translation(objects, delta)?;
        self.batch("move_page", |doc| {
            doc.translate_page_objects(objects, delta)?;
            let root = doc.loro().get_map(ROOT_MAP);
            write(&root, id, "deleted", false);
            write_frame(
                &root,
                id,
                Frame {
                    origin: Point::new(page.origin.x + delta.x, page.origin.y + delta.y),
                    size: page.size,
                },
            );
            Ok(true)
        })
    }
    /// Resizes a frame around its centre in one commit; no object moves (ADR 0031 decision 9a).
    /// # Errors
    /// Refuses unknown identities or invalid sizes;
    /// [`PageError::FixedSize`] for an output page or a page of an unknown kind (ADR 0042 §3).
    pub fn resize_page(&self, id: &str, size: DocumentSize) -> Result<bool, PageError> {
        let page = self.find_page(id)?;
        if page.kind.fixed_size() {
            return Err(PageError::FixedSize);
        }
        let size = DocumentSize::new(
            validated_document_side(size.width)?,
            validated_document_side(size.height)?,
        );
        Ok(self.write_page_frame(&page, size, "resize_page"))
    }

    /// Writes `page` resized to `size` around its centre and commits it under `label`; `false`
    /// and nothing written when the size stays within 1e-9 mm.
    pub(crate) fn write_page_frame(&self, page: &Page, size: DocumentSize, label: &str) -> bool {
        let same = (size.width.as_mm() - page.size.width.as_mm()).abs() <= EPS
            && (size.height.as_mm() - page.size.height.as_mm()).abs() <= EPS;
        if same {
            return false;
        }
        let frame = Frame {
            origin: page.origin,
            size: page.size,
        }
        .resized(size);
        self.write_frame_commit(&page.id, frame, label);
        true
    }

    /// Writes `frame` as page `id`'s frame and commits it under `label`.
    pub(crate) fn write_frame_commit(&self, id: &str, frame: Frame, label: &str) {
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, id, "deleted", false);
        write_frame(&root, id, frame);
        self.commit_with_label(label);
    }
}
