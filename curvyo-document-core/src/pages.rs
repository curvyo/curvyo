//! Ordered spatial page frames stored as independent root registers.
use crate::document::{KEY_HEIGHT_MM, KEY_WIDTH_MM, ROOT_MAP};
use crate::document_size::{MAX_DOCUMENT_MM, MIN_DOCUMENT_MM, validated_document_side};
use crate::{Document, DocumentSize, DocumentSizeError, NodeId, ObjectEditError, Point, Vec2};
use loro::{LoroMap, LoroValue};
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
pub(crate) fn pages_of_root(root: &LoroMap) -> Vec<Page> {
    let mut ids = std::collections::BTreeSet::new();
    root.for_each(|name, _| {
        if let Some(id) = name
            .strip_prefix("page.")
            .and_then(|s| s.strip_suffix(".name"))
        {
            ids.insert(id.to_owned());
        }
    });
    let mut pages: Vec<_> = ids
        .into_iter()
        .filter_map(|id| {
            // Persisted values are validated strictly: command tolerance must not
            // admit damaged sizes. Keep the frame and fall back on both axes,
            // without repairing or otherwise writing the stored registers.
            let valid = |field| {
                number(root, &id, field)
                    .filter(|mm| (MIN_DOCUMENT_MM..=MAX_DOCUMENT_MM).contains(mm))
            };
            let size = match (valid("width"), valid("height")) {
                (Some(width), Some(height)) => DocumentSize::from_mm(width, height),
                _ => DocumentSize::default(),
            };
            let page = Page {
                name: string(root, &id, "name")?,
                origin: Point::new(number(root, &id, "x")?, number(root, &id, "y")?),
                size,
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
        pages.push(fallback.unwrap_or_else(|| Page {
            id: FIRST_PAGE_ID.to_owned(),
            name: "Page 1".into(),
            origin: Point::new(0.0, 0.0),
            size: Document::size_of_root(root),
        }));
    }
    pages
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
        };
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, &last.id, "deleted", false);
        write(&root, &page.id, "name", page.name.clone());
        for (field, value) in [
            ("x", origin.x),
            ("y", origin.y),
            ("width", page.size.width.as_mm()),
            ("height", page.size.height.as_mm()),
            (
                "order",
                number(&root, &last.id, "order").unwrap_or(0.0) + 1.0,
            ),
        ] {
            write(&root, &page.id, field, value);
        }
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
            write(&root, id, "x", page.origin.x + delta.x);
            write(&root, id, "y", page.origin.y + delta.y);
            Ok(true)
        })
    }
    /// Resizes a frame, centering the supplied exclusively contained leaves.
    /// # Errors
    /// Refuses unknown identities, invalid sizes or invalid object transforms.
    pub fn resize_page(
        &self,
        id: &str,
        size: DocumentSize,
        objects: &[NodeId],
    ) -> Result<bool, PageError> {
        let page = self.find_page(id)?;
        let size = DocumentSize::new(
            validated_document_side(size.width)?,
            validated_document_side(size.height)?,
        );
        let delta = Self::resize_shift(page.size, size);
        if delta.x.abs() <= EPS && delta.y.abs() <= EPS {
            return Ok(false);
        }
        self.validate_page_translation(objects, delta)?;
        self.batch("resize_page", |doc| {
            doc.translate_page_objects(objects, delta)?;
            let root = doc.loro().get_map(ROOT_MAP);
            write(&root, id, "deleted", false);
            write(&root, id, "width", size.width.as_mm());
            write(&root, id, "height", size.height.as_mm());
            Ok(true)
        })
    }
}
