//! Ordered spatial page frames stored as root registers, each frame one register.
use crate::document::{KEY_HEIGHT_MM, KEY_WIDTH_MM, ROOT_MAP};
use crate::document_size::{MAX_DOCUMENT_MM, MIN_DOCUMENT_MM, validated_document_side};
use crate::page_frame::{Frame, read_frame, write_frame};
use crate::page_grid::{PageGrid, grid_of_fields, grid_of_root, write_non_default};
use crate::{Document, DocumentSize, DocumentSizeError, NodeId, ObjectEditError, Point, Vec2};
use loro::{LoroMap, LoroValue, ValueOrContainer};
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

/// Every drawing page id with a name (an owned output page is not one), and the grid registers
/// of each page, in one pass over the root.
fn page_keys(
    root: &LoroMap,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::HashMap<String, GridFields>,
) {
    let mut ids = std::collections::BTreeSet::new();
    // A page a manufacturing owns is its output page, never a drawing page (ADR 0032 §5).
    let mut owned = std::collections::HashSet::new();
    // The grid registers are gathered in the same pass, so a page without any (a grid never
    // touched) costs no keyed read per frame (`specs/0090-grid` criterion 9).
    let mut grids: std::collections::HashMap<String, GridFields> = std::collections::HashMap::new();
    root.for_each(|name, value| {
        let Some(rest) = name.strip_prefix("page.") else {
            return;
        };
        if let Some(id) = rest.strip_suffix(".name") {
            ids.insert(id.to_owned());
        } else if let Some(id) = rest.strip_suffix(".owner") {
            owned.insert(id.to_owned());
        } else if let Some((id, field)) = rest.rsplit_once('.')
            && field.starts_with("grid_")
            && let ValueOrContainer::Value(value) = value
        {
            grids
                .entry(id.to_owned())
                .or_default()
                .push((field.to_owned(), value));
        }
    });
    ids.retain(|id| !owned.contains(id));
    (ids, grids)
}

pub(crate) fn pages_of_root(root: &LoroMap) -> Vec<Page> {
    let (ids, grids) = page_keys(root);
    let mut pages: Vec<_> = ids
        .into_iter()
        .filter_map(|id| {
            let frame = frame_of(root, &id)?;
            let page = Page {
                name: string(root, &id, "name")?,
                origin: frame.origin,
                size: frame.size,
                grid: grids.get(&id).map_or(PageGrid::DEFAULT, |fields| {
                    grid_of_fields(|field| {
                        fields
                            .iter()
                            .find(|(name, _)| name == field)
                            .map(|(_, value)| value.clone())
                    })
                }),
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
    }
}

/// The output page `id` a manufacturing owns, `None` when it is missing, deleted or not owned. Its
/// size is the machine's work area; its position in the plane never matters (ADR 0032 §5).
pub(crate) fn owned_page(root: &LoroMap, id: &str) -> Option<Page> {
    crate::manufacturing_codec::page_owner(root, id)?;
    if root
        .get(&key(id, "deleted"))
        .is_some_and(|v| v.get_deep_value() == LoroValue::Bool(true))
    {
        return None;
    }
    let frame = frame_of(root, id)?;
    Some(Page {
        id: id.to_owned(),
        name: string(root, id, "name")?,
        origin: Point::new(0.0, 0.0),
        size: frame.size,
        grid: PageGrid::DEFAULT,
    })
}

impl Document {
    /// Frames in list order; reading never writes document state.
    #[must_use]
    pub fn pages(&self) -> Vec<Page> {
        pages_of_root(&self.loro().get_map(ROOT_MAP))
    }
    /// Adds a frame after the last listed frame in one history step.
    #[allow(clippy::missing_panics_doc)] // pages() guarantees a frame.
    pub fn add_page(&self) -> Page {
        let pages = self.pages();
        // invariant: pages always contains at least one frame.
        #[allow(clippy::unwrap_used)]
        let last = pages.last().unwrap();
        let gap = 20.0_f64.max(0.1 * last.size.width.as_mm().max(last.size.height.as_mm()));
        let row: Vec<_> = pages
            .iter()
            .rev()
            .take_while(|p| (p.origin.y - last.origin.y).abs() <= EPS)
            .collect();
        let left = row.iter().map(|p| p.origin.x).fold(f64::INFINITY, f64::min);
        let mut origin = Point::new(last.origin.x + last.size.width.as_mm() + gap, last.origin.y);
        if row.len() >= 8 || origin.x + last.size.width.as_mm() - left > 4000.0 {
            origin = Point::new(
                left,
                row.iter()
                    .map(|p| p.origin.y + p.size.height.as_mm())
                    .fold(f64::NEG_INFINITY, f64::max)
                    + gap,
            );
        }
        let peer = self.loro().peer_id();
        let counter = self.loro().oplog_vv().get(&peer).copied().unwrap_or(0);
        let mut n = 1;
        while pages.iter().any(|p| p.name == format!("Page {n}")) {
            n += 1;
        }
        let pending = self.loro().get_pending_txn_len();
        let page = Page {
            id: if pending == 0 {
                format!("{peer}-{counter}")
            } else {
                format!("{peer}-{counter}-{pending}")
            },
            name: format!("Page {n}"),
            origin,
            size: last.size,
            grid: last.grid,
        };
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, &last.id, "deleted", false);
        write(&root, &page.id, "name", page.name.clone());
        write(
            &root,
            &page.id,
            "order",
            number(&root, &last.id, "order").unwrap_or(0.0) + 1.0,
        );
        write_frame(
            &root,
            &page.id,
            Frame {
                origin,
                size: page.size,
            },
        );
        write_non_default(&root, &page.id, page.grid);
        self.commit_with_label("add_page");
        page
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
    /// Removes only the frame, preserving all objects.
    /// # Errors
    /// Refuses an unknown page or the last frame.
    pub fn delete_page(&self, id: &str) -> Result<(), PageError> {
        self.find_page(id)?;
        if self.pages().len() == 1 {
            return Err(PageError::LastPage);
        }
        write(&self.loro().get_map(ROOT_MAP), id, "deleted", true);
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
    /// Refuses unknown identities or invalid sizes.
    pub fn resize_page(&self, id: &str, size: DocumentSize) -> Result<bool, PageError> {
        let page = self.find_page(id)?;
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
