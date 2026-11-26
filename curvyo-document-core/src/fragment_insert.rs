//! The insert of a [`Fragment`] into this document as plain copies, in one commit (ADR 0084 §6,
//! §7, `specs/0137` criteria 4 to 8): elements into the active layer of a page, or a whole page
//! after the last page; every copied root and an imported page carry the Came from record.

use std::collections::{HashMap, HashSet};

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};

use crate::assets::MAX_PROJECT_ASSET_BYTES;
use crate::came_from::CameFrom;
use crate::copy_roots::{RootCopy, RootPlace, copy_roots};
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::fragment::Fragment;
use crate::page_kind::PageKind;
use crate::pages::Page;
use crate::path_model::{AnchorId, NodeId};
use crate::units::Vec2;

/// The label of the step that imports a page.
pub const IMPORT_PAGE_COMMIT_LABEL: &str = "import_page";
/// The label of the step that imports elements.
pub const IMPORT_ELEMENTS_COMMIT_LABEL: &str = "import_elements";
/// The label of the step that loads every page of an online template (`specs/0138` criterion 3).
pub const LOAD_ONLINE_TEMPLATE_COMMIT_LABEL: &str = "load_online_template";
/// The label of the step that imports an online icon (`specs/0138` criterion 3).
pub const IMPORT_ICON_COMMIT_LABEL: &str = "import_icon";

/// Where a fragment goes.
#[derive(Debug, Clone, Copy)]
pub enum FragmentTarget<'a> {
    /// Its objects onto the page `page`, into the layer new objects go into (the draw parent),
    /// moved by `offset`.
    Elements {
        /// The page the elements land on.
        page: &'a str,
        /// The move from the source position to the drop position.
        offset: Vec2,
    },
    /// Its objects onto the page `page`, moved by `offset`, wrapped in one new group named `name`
    /// at the end of the draw parent; the group carries the Came from record with the source page
    /// as the item, each copy its own (`specs/0154` criterion 2, ADR 0088 decision 3).
    Group {
        /// The page the group lands on.
        page: &'a str,
        /// The move from the source position to the drop position.
        offset: Vec2,
        /// The group's name (the source page's).
        name: &'a str,
    },
    /// Its page as a new page after the last page.
    Page,
}

/// Why an imported page came in as a drawing page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageFallback {
    /// The page kind is one this build does not know (its stored text).
    UnknownKind(String),
    /// The output page's machine data is missing in the source.
    NoMachineData,
}

/// What an import made.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentReport {
    /// The copies of the fragment's roots, in paint order.
    pub roots: Vec<NodeId>,
    /// The imported page.
    pub page: Option<Page>,
    /// How many linked clones became paths because their originals were not taken.
    pub clones_became_paths: usize,
    /// How many texts on a path were released from it because their carrier was not taken.
    pub texts_released: usize,
    /// Why the page came in as a drawing page, when it did.
    pub fallback: Option<PageFallback>,
}

/// Why an import wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    /// Elements cannot go onto a sketch page (`specs/0099`).
    #[error("Elements cannot go onto a sketch page.")]
    SketchPage,
    /// The target page does not exist, or a page import has no page.
    #[error("the page does not exist")]
    NoSuchPage,
    /// The anchor ids are not one per anchor of the fragment, all different.
    #[error("the anchor ids do not fit the fragment")]
    AnchorIds,
    /// The wrapping group would nest deeper than the group depth limit (`specs/0154` criterion 2).
    #[error("The pieces are nested too deep to be wrapped in one more group.")]
    TooDeep,
    /// The pictures and fonts would take the project over its limit (ADR 0047 §3).
    #[error("the project would hold more pictures and fonts than allowed")]
    ProjectFull,
}

/// The page registers an insert writes itself rather than copying from the source page.
const OWN_PAGE_FIELDS: &[&str] = &[
    "name",
    "order",
    "x",
    "y",
    "width",
    "height",
    "deleted",
    crate::came_from::KEY_CAME_FROM,
    crate::page_kind::KIND_FIELD,
    crate::manufacturing_codec::PAGE_OWNER_FIELD,
    crate::sketch_page::OWNER_FIELD,
];

impl Document {
    /// Inserts `fragment` as plain copies at `target`, in one commit labelled
    /// [`IMPORT_PAGE_COMMIT_LABEL`] or [`IMPORT_ELEMENTS_COMMIT_LABEL`]. Every copied root carries
    /// `came_from` with its own name as the item, and an imported page carries `came_from`.
    /// `anchor_ids` are [`Fragment::anchor_count`] fresh ids. A second insert of the same fragment
    /// makes a second copy.
    ///
    /// # Errors
    /// [`ImportError`]; nothing is written then.
    ///
    /// # Panics
    /// Does not panic in practice: the checks come first and the wrapping group's meta map is
    /// read right after it is made.
    pub fn insert_fragment(
        &self,
        fragment: &Fragment,
        target: FragmentTarget<'_>,
        came_from: &CameFrom,
        anchor_ids: &[AnchorId],
    ) -> Result<FragmentReport, ImportError> {
        self.check_insert(fragment, target, anchor_ids)?;
        Ok(match target {
            FragmentTarget::Elements { offset, .. } => {
                self.batch(IMPORT_ELEMENTS_COMMIT_LABEL, |doc| {
                    let tree = doc.loro().get_tree(OBJECTS_TREE);
                    let places = vec![doc.drawing_parent(&tree); fragment.roots.len()];
                    let roots = doc.copy_fragment(fragment, &places, offset, came_from, anchor_ids);
                    elements_report(fragment, roots)
                })
            }
            FragmentTarget::Group { offset, name, .. } => self
                .batch(IMPORT_ELEMENTS_COMMIT_LABEL, |doc| {
                    doc.insert_group(fragment, offset, name, came_from, anchor_ids)
                }),
            FragmentTarget::Page => {
                // invariant: `check_insert` refused a page import without a page.
                #[allow(clippy::unwrap_used)]
                let source = fragment.page.as_ref().unwrap();
                self.batch(IMPORT_PAGE_COMMIT_LABEL, |doc| {
                    doc.insert_page(fragment, source, came_from, anchor_ids)
                })
            }
        })
    }

    /// Every check [`Document::insert_fragment`] makes before it writes: the anchor ids, the
    /// project's asset budget, the target page and, for a wrap, the group depth. A caller that
    /// inserts several fragments in one step checks them all first, so a refusal writes nothing.
    ///
    /// # Errors
    /// The [`ImportError`] the insert would return.
    pub fn check_insert(
        &self,
        fragment: &Fragment,
        target: FragmentTarget<'_>,
        anchor_ids: &[AnchorId],
    ) -> Result<(), ImportError> {
        let distinct: HashSet<_> = anchor_ids.iter().collect();
        if anchor_ids.len() != fragment.anchor_count() || distinct.len() != anchor_ids.len() {
            return Err(ImportError::AnchorIds);
        }
        self.check_budget(fragment)?;
        let page = match target {
            FragmentTarget::Elements { page, .. } | FragmentTarget::Group { page, .. } => page,
            FragmentTarget::Page => {
                return fragment
                    .page
                    .as_ref()
                    .map(|_| ())
                    .ok_or(ImportError::NoSuchPage);
            }
        };
        match self.page_kind(page) {
            None => return Err(ImportError::NoSuchPage),
            Some(PageKind::Sketch) => return Err(ImportError::SketchPage),
            Some(_) => {}
        }
        if matches!(target, FragmentTarget::Group { .. }) {
            self.check_wrap_depth(fragment)?;
        }
        Ok(())
    }

    /// The wrapped insert inside its batch: the group named `name` at the end of the draw
    /// parent, the copies in it, Came from on the group with the source page as the item.
    fn insert_group(
        &self,
        fragment: &Fragment,
        offset: Vec2,
        name: &str,
        came_from: &CameFrom,
        anchor_ids: &[AnchorId],
    ) -> FragmentReport {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let parent = self.drawing_parent(&tree);
        let group = wrap_group(&tree, parent, name);
        let places = vec![TreeParentId::Node(group); fragment.roots.len()];
        self.copy_fragment(fragment, &places, offset, came_from, anchor_ids);
        // invariant: the group was just created by this insert.
        #[allow(clippy::unwrap_used)]
        crate::came_from::write(
            &tree.get_meta(group).unwrap(),
            &came_from.with_item(Some(name.to_owned())),
        );
        elements_report(
            fragment,
            vec![NodeId::from_parts(group.peer, group.counter)],
        )
    }

    /// Inserts the page of every fragment of `pages` after the last page, in order, as plain
    /// copies with `came_from`, in **one** step labelled [`LOAD_ONLINE_TEMPLATE_COMMIT_LABEL`]. Each
    /// fragment is a page import with its own anchor ids. Everything is checked before the
    /// first write.
    ///
    /// # Errors
    /// [`ImportError`]; nothing is written then.
    pub fn insert_fragment_pages(
        &self,
        pages: &[(&Fragment, &[AnchorId])],
        came_from: &CameFrom,
    ) -> Result<Vec<FragmentReport>, ImportError> {
        for (fragment, anchor_ids) in pages {
            let distinct: HashSet<_> = anchor_ids.iter().collect();
            if anchor_ids.len() != fragment.anchor_count() || distinct.len() != anchor_ids.len() {
                return Err(ImportError::AnchorIds);
            }
            if fragment.page.is_none() {
                return Err(ImportError::NoSuchPage);
            }
            self.check_budget(fragment)?;
        }
        Ok(self.batch(LOAD_ONLINE_TEMPLATE_COMMIT_LABEL, |doc| {
            pages
                .iter()
                .filter_map(|(fragment, anchor_ids)| {
                    let source = fragment.page.as_ref()?;
                    Some(doc.insert_page(fragment, source, came_from, anchor_ids))
                })
                .collect()
        }))
    }

    /// Refuses a wrap whose group would pass [`MAX_GROUP_DEPTH`](crate::group_tree::MAX_GROUP_DEPTH)
    /// under the draw parent.
    fn check_wrap_depth(&self, fragment: &Fragment) -> Result<(), ImportError> {
        use crate::group_tree::{MAX_GROUP_DEPTH, groups_above, groups_in, is_group_node};
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let from = fragment.source.loro().get_tree(OBJECTS_TREE);
        let above = match self.creation_parent(&tree) {
            TreeParentId::Node(p) if is_group_node(&tree, p) => groups_above(&tree, p) + 1,
            TreeParentId::Node(p) => groups_above(&tree, p),
            _ => 0,
        };
        let inside = fragment
            .roots
            .iter()
            .map(|&root| groups_in(&from, root))
            .max()
            .unwrap_or(0);
        if above + 1 + inside > MAX_GROUP_DEPTH {
            Err(ImportError::TooDeep)
        } else {
            Ok(())
        }
    }

    /// Refuses a fragment whose new pictures and fonts would pass the project limit.
    fn check_budget(&self, fragment: &Fragment) -> Result<(), ImportError> {
        let store = self.assets.borrow();
        let files: Vec<_> = fragment
            .assets
            .iter()
            .filter(|(_, _, derived)| !derived)
            .map(|(hash, bytes, _)| (hash.clone(), &**bytes))
            .collect();
        budget(
            store.total_bytes(),
            store.added_bytes(&files),
            MAX_PROJECT_ASSET_BYTES,
        )
    }

    /// The page import inside its batch: the new page, its registers, its members moved by the
    /// change of the frame origin into layers of the same names.
    fn insert_page(
        &self,
        fragment: &Fragment,
        source: &Page,
        came_from: &CameFrom,
        anchor_ids: &[AnchorId],
    ) -> FragmentReport {
        let pages = self.pages();
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let from = fragment.source.loro().get_tree(OBJECTS_TREE);
        let mut fallback = match &source.kind {
            PageKind::Unknown(text) => Some(PageFallback::UnknownKind(text.clone())),
            _ => None,
        };
        let manufacturing = match &source.kind {
            PageKind::Output { manufacturing, .. } => {
                let id = crate::paths::tree_id_of(*manufacturing);
                if fragment.roots.contains(&id) {
                    Some(id)
                } else {
                    fallback = Some(PageFallback::NoMachineData);
                    None
                }
            }
            _ => None,
        };
        let origin = crate::pages::add_page_origin(&pages, source.size);
        let offset = Vec2::new(origin.x - source.origin.x, origin.y - source.origin.y);
        let places = page_places(&from, &tree, &fragment.roots, manufacturing);
        let copies = self.copy_fragment_nodes(fragment, &places, offset, came_from, anchor_ids);
        let copy_of = |id: TreeID| copies.iter().find(|(s, _)| *s == id).map(|(_, c)| *c);
        let page = self.imported_page(source, origin, manufacturing.and_then(copy_of));
        self.write_imported_page(fragment, source, &pages, &page, &copies, came_from);
        let roots = fragment
            .roots
            .iter()
            .filter(|&&r| Some(r) != manufacturing)
            .filter_map(|&r| copy_of(r))
            .map(|c| NodeId::from_parts(c.peer, c.counter))
            .collect();
        FragmentReport {
            roots,
            page: Some(page),
            clones_became_paths: fragment.clones_became_paths,
            texts_released: fragment.texts_released,
            fallback,
        }
    }

    /// The imported page: the source's name, size, grid and kind at `origin`; an output page is
    /// owned by the copy of its manufacturing, `manufacturing`, and without one is a drawing page.
    fn imported_page(
        &self,
        source: &Page,
        origin: crate::Point,
        manufacturing: Option<TreeID>,
    ) -> Page {
        let id = match manufacturing {
            Some(node) => format!("mfg-{}-{}", node.peer, node.counter),
            None => self.new_page_id(),
        };
        let kind = match (&source.kind, manufacturing) {
            (PageKind::Output { process, .. }, Some(node)) => PageKind::Output {
                manufacturing: NodeId::from_parts(node.peer, node.counter),
                process: process.clone(),
            },
            (PageKind::Sketch, _) => PageKind::Sketch,
            _ => PageKind::Drawing,
        };
        Page {
            id,
            name: source.name.clone(),
            origin,
            size: source.size,
            grid: source.grid,
            kind,
        }
    }

    /// Writes the registers of the imported `page` (after `pages`), its owner and sketch claim
    /// from `copies`, and its Came from, uncommitted.
    fn write_imported_page(
        &self,
        fragment: &Fragment,
        source: &Page,
        pages: &[Page],
        page: &Page,
        copies: &[(TreeID, TreeID)],
        came_from: &CameFrom,
    ) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let manufacturing_copy = page.kind.manufacturing().map(crate::paths::tree_id_of);
        let root = self.loro().get_map(ROOT_MAP);
        Self::write_new_page(&root, pages, page);
        copy_page_registers(
            &fragment.source.loro().get_map(ROOT_MAP),
            &root,
            &source.id,
            &page.id,
        );
        crate::pages::write(&root, &page.id, "deleted", false);
        if let Some(stored) = page.kind.stored() {
            crate::pages::write(&root, &page.id, crate::page_kind::KIND_FIELD, stored);
        }
        if let Some(node) = manufacturing_copy {
            // invariant: the node was just created by this insert, so its meta map exists.
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(node).unwrap();
            // invariant: an attached meta map takes a plain value under any key.
            #[allow(clippy::unwrap_used)]
            meta.insert(crate::manufacturing_codec::KEY_PAGE, page.id.as_str())
                .unwrap();
            crate::pages::write(
                &root,
                &page.id,
                crate::manufacturing_codec::PAGE_OWNER_FIELD,
                crate::manufacturing_codec::owner_text(node),
            );
        }
        if page.kind == PageKind::Sketch
            && let Some(sketch) = fragment.source.page_sketch(&source.id)
        {
            let claims = HashMap::from([(sketch, page.id.clone())]);
            self.assign_sketch_copies(copies, &claims);
        }
        crate::came_from::write_page(&root, &page.id, came_from);
    }

    /// Copies the fragment's roots under `places` (one per root) and returns the root copies.
    fn copy_fragment(
        &self,
        fragment: &Fragment,
        places: &[TreeParentId],
        offset: Vec2,
        came_from: &CameFrom,
        anchor_ids: &[AnchorId],
    ) -> Vec<NodeId> {
        let made = self.copy_fragment_nodes(fragment, places, offset, came_from, anchor_ids);
        fragment
            .roots
            .iter()
            .filter_map(|root| made.iter().find(|(s, _)| s == root))
            .map(|(_, c)| NodeId::from_parts(c.peer, c.counter))
            .collect()
    }

    /// The copy itself, uncommitted: the nodes, their references, their order, the record on
    /// each root and the assets. Returns `(source, copy)` of every copied node.
    fn copy_fragment_nodes(
        &self,
        fragment: &Fragment,
        places: &[TreeParentId],
        offset: Vec2,
        came_from: &CameFrom,
        anchor_ids: &[AnchorId],
    ) -> Vec<(TreeID, TreeID)> {
        let from = fragment.source.loro().get_tree(OBJECTS_TREE);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut rest = anchor_ids;
        let roots: Vec<RootCopy<'_>> = fragment
            .roots
            .iter()
            .zip(places)
            .map(|(&source, &parent)| {
                let count = crate::group_tree::anchors_under(&from, source);
                let (own, others) = rest.split_at(count.min(rest.len()));
                rest = others;
                RootCopy {
                    source,
                    anchor_ids: own,
                    place: RootPlace::End(parent),
                }
            })
            .collect();
        let copied = copy_roots(
            (&from, &tree),
            &roots,
            (offset, None),
            &fragment.source.clone_links(),
        );
        order_roots(&tree, places, &copied.roots);
        self.relink_copies(
            &fragment.source,
            &tree,
            &copied.made,
            offset,
            &HashMap::new(),
        );
        for (source, copy) in fragment.roots.iter().zip(&copied.roots) {
            let Some(copy) = copy else { continue };
            let item = from
                .get_meta(*source)
                .ok()
                .and_then(|meta| crate::node_registers::read_registers(&meta).name);
            // invariant: the copy was just created by this insert.
            #[allow(clippy::unwrap_used)]
            crate::came_from::write(&tree.get_meta(*copy).unwrap(), &came_from.with_item(item));
        }
        let mut store = self.assets.borrow_mut();
        for (hash, bytes, derived) in &fragment.assets {
            if *derived {
                store.insert_derived(hash.clone(), bytes.clone());
            } else {
                store.insert(hash.clone(), bytes.clone());
            }
        }
        copied.made
    }
}

/// The report of an elements or a group insert with the root copies `roots`.
const fn elements_report(fragment: &Fragment, roots: Vec<NodeId>) -> FragmentReport {
    FragmentReport {
        roots,
        page: None,
        clones_became_paths: fragment.clones_became_paths,
        texts_released: fragment.texts_released,
        fallback: None,
    }
}

/// Creates the wrapping group named `name` at the end of `parent`. Uncommitted.
fn wrap_group(tree: &LoroTree, parent: TreeParentId, name: &str) -> TreeID {
    // invariant: the draw parent is live; the meta map of a node just created takes plain values.
    #[allow(clippy::unwrap_used)]
    let group = tree.create(parent).unwrap();
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(group).unwrap();
    #[allow(clippy::unwrap_used)]
    meta.insert(crate::group_tree::KEY_GROUP, true).unwrap();
    #[allow(clippy::unwrap_used)]
    meta.insert(crate::node_registers::KEY_NAME, name).unwrap();
    group
}

/// The parent of each root's copy in a page import: the top level for the manufacturing, else
/// the layer of the same name as the root's in the source (made when missing), else the top
/// level.
fn page_places(
    from: &LoroTree,
    tree: &LoroTree,
    roots: &[TreeID],
    manufacturing: Option<TreeID>,
) -> Vec<TreeParentId> {
    roots
        .iter()
        .map(|&root| {
            if Some(root) == manufacturing {
                return TreeParentId::Root;
            }
            layer_name(from, root).map_or(TreeParentId::Root, |name| {
                TreeParentId::Node(layer_named(tree, &name))
            })
        })
        .collect()
}

/// Whether `added` new asset bytes fit beside `held` under `limit` (ADR 0047 §3): an import over
/// it is refused whole.
fn budget(held: u64, added: u64, limit: u64) -> Result<(), ImportError> {
    if held.saturating_add(added) > limit {
        Err(ImportError::ProjectFull)
    } else {
        Ok(())
    }
}

/// Moves the root copies so they keep the fragment's paint order at the end of each parent.
fn order_roots(tree: &LoroTree, places: &[TreeParentId], copies: &[Option<TreeID>]) {
    let mut by_parent: Vec<(TreeParentId, Vec<TreeID>)> = Vec::new();
    for (parent, copy) in places.iter().zip(copies) {
        let Some(copy) = copy else { continue };
        match by_parent.iter_mut().find(|(p, _)| p == parent) {
            Some((_, list)) => list.push(*copy),
            None => by_parent.push((*parent, vec![*copy])),
        }
    }
    for (parent, list) in by_parent {
        let count = tree.children(parent).map_or(0, |c| c.len());
        let start = count.saturating_sub(list.len());
        for (offset, node) in list.into_iter().enumerate() {
            // invariant: every node is a child of `parent` just created, the index in range.
            #[allow(clippy::unwrap_used)]
            tree.mov_to(node, parent, start + offset).unwrap();
        }
    }
}

/// The name of the layer `node` lies in, when it lies in one.
fn layer_name(tree: &LoroTree, node: TreeID) -> Option<String> {
    let mut current = node;
    while let Some(parent) = crate::manufacturing_codec::parent_node(tree, current) {
        current = parent;
    }
    if current == node || !crate::group_tree::is_layer_node(tree, current) {
        return None;
    }
    let meta = tree.get_meta(current).ok()?;
    crate::node_registers::read_registers(&meta).name
}

/// The top-level layer named `name`, made at the front when there is none. Uncommitted.
fn layer_named(tree: &LoroTree, name: &str) -> TreeID {
    let existing = crate::group_tree::children_of(tree, None)
        .into_iter()
        .find(|&id| {
            crate::group_tree::is_layer_node(tree, id)
                && tree.get_meta(id).is_ok_and(|meta| {
                    crate::node_registers::read_registers(&meta).name.as_deref() == Some(name)
                })
        });
    existing.unwrap_or_else(|| crate::layers::create_layer_node(tree, name))
}

/// Copies the registers of page `from_id` in `from` that the insert does not write itself to page
/// `to_id` in `into` (grid, background and any register a later build adds).
fn copy_page_registers(from: &LoroMap, into: &LoroMap, from_id: &str, to_id: &str) {
    let prefix = format!("page.{from_id}.");
    let mut copies: Vec<(String, LoroValue)> = Vec::new();
    from.for_each(|key, value| {
        let Some(field) = key.strip_prefix(&prefix) else {
            return;
        };
        if field.contains('.') || OWN_PAGE_FIELDS.contains(&field) {
            return;
        }
        if let ValueOrContainer::Value(value) = value {
            copies.push((field.to_owned(), value));
        }
    });
    for (field, value) in copies {
        crate::pages::write(into, to_id, &field, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0084 decision 6, ADR 0047 §3: an import whose new pictures and fonts would pass the
    /// project limit is refused whole; one that reaches it exactly is taken.
    #[test]
    fn an_import_over_the_asset_budget_is_refused() {
        assert_eq!(budget(90, 11, 100), Err(ImportError::ProjectFull));
        assert_eq!(budget(90, 10, 100), Ok(()));
        assert_eq!(budget(u64::MAX, 1, 100), Err(ImportError::ProjectFull));
    }
}
