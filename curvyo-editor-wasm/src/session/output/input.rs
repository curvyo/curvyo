//! The job input of the active laser page (`specs/0029` criteria 3, 11, 13, 20, 33; ADR 0064 §1):
//! the running fixture jobs as one block per kind, then the operations in run order, each with
//! the leaves the nearest rule runs in it, in page mm.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{Affine, AssetHash, NodeId, OperationSnapshot, Point, Vec2};
use curvyo_laser_core::GENERIC_LASER_ID;
use curvyo_laser_core::job::{
    BlockInput, BlockSource, CutOutItem, GateFacts, JobInput, LeafGeometry, LeafInput, LeafOutline,
    MachineInput, Ring, read_output_settings,
};
use curvyo_laser_core::operations::{
    KindRead, Leaf, LeafStatus, OperationKind, OperationList, leaf_status,
};
use curvyo_laser_core::settings::{OperationOptions, read_options, read_settings};

use super::super::Session;
use super::super::fixtures::FixtureRun;
use super::super::manufacturing::ActiveCopy;
use super::super::object_cache::DocumentRead;
use super::super::operations::index::{IndexedLeaf, LeafSource};
use super::leaf_geometry::{leaf_geometry, polygon};

/// The job input of one laser page and the facts of its gate.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PageInput {
    /// The laser page.
    pub manufacturing: NodeId,
    /// The input.
    pub input: JobInput,
    /// The gate's facts beside the job.
    pub facts: GateFacts,
}

/// The page-mm geometry of each indexed leaf, for the leaf list it was built from.
#[derive(Debug, Default)]
pub(super) struct GeometryCache(std::cell::RefCell<Option<GeometryEntry>>);

#[derive(Debug)]
struct GeometryEntry {
    leaves: Arc<[IndexedLeaf]>,
    maps: Vec<Affine>,
    geometry: Arc<[Option<LeafGeometry>]>,
}

/// The id a check line selects for a leaf: the first node of its chain (a clone member selects
/// its clone, a placed page's member its source object).
pub(super) fn select_id(leaf: &IndexedLeaf) -> Option<NodeId> {
    leaf.chain.first().and_then(|key| key.first().copied())
}

/// The page-mm maps of a laser page: the page's own (document to page) first, then each
/// placement's in placement order.
fn page_maps(
    session: &Session,
    manufacturing: &curvyo_document_core::ManufacturingSnapshot,
) -> Vec<Affine> {
    let Some(page) = manufacturing.page.as_ref() else {
        return Vec::new();
    };
    let pages = session.document.pages();
    let mut maps = vec![Affine::translation(Vec2::new(
        -page.origin.x,
        -page.origin.y,
    ))];
    for placement in &manufacturing.placements {
        maps.push(
            pages
                .iter()
                .find(|p| p.id == placement.page)
                .map_or(placement.matrix, |source| {
                    curvyo_ui_core::placement_map(source, placement.matrix)
                }),
        );
    }
    maps
}

/// The geometry of `leaf` on its laser page.
fn geometry_of(
    session: &Session,
    read: &DocumentRead,
    manufacturing: &curvyo_document_core::ManufacturingSnapshot,
    maps: &[Affine],
    leaf: &IndexedLeaf,
) -> Option<LeafGeometry> {
    let own = *maps.first()?;
    let placement_map = |placement: NodeId| {
        let at = manufacturing
            .placements
            .iter()
            .position(|p| p.id == placement)?;
        maps.get(at + 1).copied()
    };
    match leaf.source {
        LeafSource::Own(at) => leaf_geometry(read.leaves.get(at)?, own),
        LeafSource::CloneMember { clone, member } => {
            leaf_geometry(read.leaves.get(clone)?.picture()?.get(member)?, own)
        }
        LeafSource::NotDrawn => None,
        LeafSource::PlacedMember { index, placement } => {
            leaf_geometry(read.leaves.get(index)?, placement_map(placement)?)
        }
        LeafSource::PlacedCloneMember {
            clone,
            member,
            placement,
        } => leaf_geometry(
            read.leaves.get(clone)?.picture()?.get(member)?,
            placement_map(placement)?,
        ),
        LeafSource::PlacedFrame(placement) => {
            let snapshot = manufacturing
                .placements
                .iter()
                .find(|p| p.id == placement)?;
            let source = session
                .document
                .pages()
                .into_iter()
                .find(|p| p.id == snapshot.page)?;
            let (w, h) = (source.size.width.as_mm(), source.size.height.as_mm());
            let corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
                .map(|(x, y)| snapshot.matrix.apply_point(Point::new(x, y)));
            Some(LeafGeometry::Outlines(vec![polygon(&corners, true)]))
        }
    }
}

impl Session {
    /// The page-mm geometry of each of `leaves`, from the cache while the leaves and maps hold.
    fn leaf_geometries(
        &self,
        manufacturing: &curvyo_document_core::ManufacturingSnapshot,
        leaves: &Arc<[IndexedLeaf]>,
    ) -> Arc<[Option<LeafGeometry>]> {
        let maps = page_maps(self, manufacturing);
        let cache = &self.output.geometry.0;
        if let Some(entry) = cache.borrow().as_ref()
            && Arc::ptr_eq(&entry.leaves, leaves)
            && entry.maps == maps
        {
            return Arc::clone(&entry.geometry);
        }
        let read = self.document_read();
        let geometry: Arc<[Option<LeafGeometry>]> = leaves
            .iter()
            .map(|leaf| geometry_of(self, &read, manufacturing, &maps, leaf))
            .collect();
        *cache.borrow_mut() = Some(GeometryEntry {
            leaves: Arc::clone(leaves),
            maps,
            geometry: Arc::clone(&geometry),
        });
        geometry
    }

    /// The working image of `asset` the job decoded already, so the canvas draws that copy.
    pub(in crate::session) fn job_decoded_image(
        &self,
        asset: &AssetHash,
    ) -> Option<Arc<curvyo_image_core::WorkingImage>> {
        self.output.pixels.held(asset)
    }

    /// The working image of `asset` for the job (`pixels.rs`): the canvas's when decoded, else
    /// decoded once; `None` when its bytes cannot be read.
    pub(in crate::session) fn job_pixels(
        &self,
        asset: &AssetHash,
    ) -> Option<Arc<curvyo_image_core::WorkingImage>> {
        self.output
            .pixels
            .working(&self.document, asset, || self.decoded_working_image(asset))
    }

    /// The job input of the active laser page, `None` on another page or with its extension off.
    pub(super) fn page_input(&self) -> Option<PageInput> {
        let active = self.active_operations()?;
        let manufacturing = self.active_manufacturing()?;
        let copy = self.active_copy()?;
        let page = manufacturing.page.as_ref()?;
        let settings = read_output_settings(&manufacturing);
        let list = OperationList::new(&active.operations, active.cuts_last);
        let leaves = self.output_leaves(&manufacturing);
        let geometry = self.leaf_geometries(&manufacturing, &leaves);
        let grid_parts: HashSet<NodeId> = self
            .grid_looks(&active.operations)
            .parts
            .into_keys()
            .collect();
        let statuses = leaf_statuses(&list, &leaves);
        let maps = page_maps(self, &manufacturing);
        let mut blocks = fixture_blocks(&self.running_fixture_runs(), maps.first().copied());
        for (row, &at) in list.sequence().iter().enumerate() {
            let op = &active.operations[at];
            if !op.output {
                continue;
            }
            // The ring is the row's digit in the Operations tab (UX P26).
            let ring = Ring::Operation(u32::try_from(row + 1).unwrap_or(u32::MAX));
            let runs_here = |index: usize| {
                let LeafStatus::Assigned { running, .. } = &statuses[index] else {
                    return false;
                };
                running.contains(&at)
                    && (!op.grid
                        || select_id(&leaves[index]).is_some_and(|id| grid_parts.contains(&id)))
            };
            let rows = (0..leaves.len()).filter(|&i| runs_here(i));
            let mut block = self.operation_block(op, rows, &leaves, &geometry, &statuses);
            block.ring = ring;
            blocks.push(block);
        }
        self.retain_job_pixels(&blocks);
        Some(PageInput {
            manufacturing: manufacturing.id,
            input: JobInput {
                name: page.name.clone(),
                machine: machine_input(&copy),
                start: settings.start,
                start_damaged: settings.start_damaged,
                fixture_material: settings.fixture_material,
                blocks,
            },
            facts: super::gate_facts::gate_facts(&active.operations, &list, &leaves, &statuses),
        })
    }

    /// Keeps the working images the leaves of `blocks` name; unreadable ones stay held too, so
    /// they are not hashed and decoded again.
    fn retain_job_pixels(&self, blocks: &[BlockInput]) {
        let assets: HashSet<&AssetHash> = blocks
            .iter()
            .flat_map(|block| &block.leaves)
            .filter_map(|leaf| match &leaf.geometry {
                LeafGeometry::Image { asset, .. } => Some(asset),
                _ => None,
            })
            .collect();
        self.output.pixels.retain(&assets);
    }

    /// Operation `op`'s block with no leaves yet: its kind, settings (an absent register read
    /// from the active copy's pair, `specs/0150` criterion 15), options and grid, ringed with the
    /// digit 0 (the caller sets the row's digit). An unknown kind's placeholder settings are
    /// never read: its block is damaged.
    fn empty_block(&self, op: &OperationSnapshot) -> BlockInput {
        let kind = KindRead::of(op.kind.as_deref());
        let defaults = self.active_defaults();
        let (settings, options) = kind.known().map_or_else(
            || {
                (
                    read_settings(None, OperationKind::Cut, defaults),
                    Ok(OperationOptions::default()),
                )
            },
            |known| {
                (
                    read_settings(op.settings.as_ref(), known, defaults),
                    read_options(op.options.as_ref(), known),
                )
            },
        );
        BlockInput {
            id: op.id,
            ring: Ring::Operation(0),
            name: op.name.clone(),
            kind,
            source: BlockSource::Operation {
                settings,
                options,
                margin: op.margin,
                grid: op
                    .grid_part
                    .as_ref()
                    .and_then(|_| self.document.test_grid_of_derived_row(op.id)),
            },
            leaves: Vec::new(),
            cut_out: Vec::new(),
        }
    }

    /// The block of operation `op` holding the leaves at `rows`, ringed with the digit 0 (the
    /// caller sets the row's digit).
    pub(in crate::session) fn operation_block(
        &self,
        op: &OperationSnapshot,
        rows: impl Iterator<Item = usize>,
        leaves: &[IndexedLeaf],
        geometry: &[Option<LeafGeometry>],
        statuses: &[LeafStatus],
    ) -> BlockInput {
        let mut block = self.empty_block(op);
        let kind = block.kind;
        let cut_out = kind.known() == Some(OperationKind::CutOut);
        // Only an Image operation reads pixels, so a page of pictures in no Image operation
        // decodes nothing for the job (`specs/0123` criterion 37 g); every other kind sees an
        // image as its frame rectangle and a gradient shape as its outlines (ADR 0069 §13).
        let rasters = kind.known() == Some(OperationKind::Image);
        let mut items: HashMap<NodeId, usize> = HashMap::new();
        for at in rows {
            let (Some(node), Some(shape)) = (select_id(&leaves[at]), geometry[at].clone()) else {
                continue;
            };
            let stacking = u32::try_from(at).unwrap_or(u32::MAX);
            if !cut_out {
                let geometry = match shape {
                    LeafGeometry::Image {
                        frame,
                        asset,
                        mut adjust,
                        ..
                    } if rasters => {
                        if let Some((held, strength)) = self.held_adjust_strength()
                            && held == node
                        {
                            adjust.strength = strength;
                        }
                        // ADR 0077 §3, §4: the stored AI results are held for the job; one
                        // that cannot be read skips the picture as unreadable.
                        let cache = self.adjust_pictures();
                        let pixels = self
                            .job_pixels(&asset)
                            .filter(|_| self.hold_stored_results(&cache, &adjust));
                        if let Some(pixels) = &pixels {
                            adjust.stale_results = cache.stale_results(&asset, pixels, &adjust);
                        }
                        LeafGeometry::Image {
                            pixels,
                            frame,
                            asset,
                            adjust,
                        }
                    }
                    other if rasters => other,
                    other => vector_geometry(other),
                };
                block.leaves.push(LeafInput {
                    node,
                    stacking,
                    geometry,
                });
                continue;
            }
            let LeafStatus::Assigned { assignment, .. } = &statuses[at] else {
                continue;
            };
            let item = assignment.target.first().copied().unwrap_or(node);
            add_cut_out(&mut block.cut_out, &mut items, (item, stacking), shape);
        }
        block
    }
}

/// `geometry` as every kind but Image runs it (Frage 271 A): an image as the one closed ring of
/// its frame, a gradient-filled leaf as its outlines.
fn vector_geometry(geometry: LeafGeometry) -> LeafGeometry {
    match geometry {
        LeafGeometry::Image { frame, .. } => LeafGeometry::Outlines(vec![polygon(&frame, true)]),
        LeafGeometry::Gradient { outlines, .. } => LeafGeometry::Outlines(outlines),
        outlines @ LeafGeometry::Outlines(_) => outlines,
    }
}

/// The status of each of `leaves` under `list`.
fn leaf_statuses(list: &OperationList, leaves: &[IndexedLeaf]) -> Vec<LeafStatus> {
    leaves
        .iter()
        .map(|leaf| {
            leaf_status(
                list,
                &Leaf {
                    chain: &leaf.chain,
                    hidden: leaf.hidden,
                    shape: leaf.shape,
                },
            )
        })
        .collect()
}

/// Adds the leaf `shape` to the Cut-out item `item` (its place in `out` kept in `items`); an
/// image counts by its frame.
fn add_cut_out(
    out: &mut Vec<CutOutItem>,
    items: &mut HashMap<NodeId, usize>,
    (item, stacking): (NodeId, u32),
    shape: LeafGeometry,
) {
    let outlines = match shape {
        LeafGeometry::Outlines(outlines) | LeafGeometry::Gradient { outlines, .. } => outlines,
        LeafGeometry::Image { frame, .. } => vec![polygon(&frame, true)],
    };
    if let Some(&i) = items.get(&item) {
        out[i].leaves.push(outlines);
    } else {
        items.insert(item, out.len());
        out.push(CutOutItem {
            item,
            stacking,
            leaves: vec![outlines],
        });
    }
}

/// The machine of a laser page's job from its profile copy.
fn machine_input(copy: &ActiveCopy) -> MachineInput {
    let profile = &copy.read.copy;
    MachineInput {
        profile_id: profile
            .id
            .clone()
            .unwrap_or_else(|| GENERIC_LASER_ID.to_owned()),
        values: profile.values.clone(),
        bed: copy.page,
        damaged: copy.read.output_format_damaged,
    }
}

/// The fixture runs as blocks, each part mapped by `to_page` (document to page mm).
fn fixture_blocks(runs: &[FixtureRun], to_page: Option<Affine>) -> Vec<BlockInput> {
    let Some(map) = to_page else {
        return Vec::new();
    };
    runs.iter()
        .filter_map(|run| {
            let id = *run.jobs.first()?;
            let part_outlines: Vec<(NodeId, Vec<LeafOutline>)> = run
                .parts
                .iter()
                .map(|(fixture, outlines)| {
                    let mapped = outlines
                        .iter()
                        .map(|o| LeafOutline {
                            anchors: o
                                .anchors
                                .iter()
                                .map(|&(p, i, out)| {
                                    (
                                        map.apply_point(p),
                                        map.apply_vector(i),
                                        map.apply_vector(out),
                                    )
                                })
                                .collect(),
                            closed: o.closed,
                        })
                        .collect();
                    (*fixture, mapped)
                })
                .collect();
            let mut block = BlockInput {
                id,
                ring: Ring::Fixture,
                name: format!("Fixture jobs · {}", run.kind.label()),
                kind: KindRead::Known(run.kind),
                source: BlockSource::Fixture {
                    jobs: run.jobs.clone(),
                },
                leaves: Vec::new(),
                cut_out: Vec::new(),
            };
            for (at, (fixture, parts)) in part_outlines.into_iter().enumerate() {
                let stacking = u32::try_from(at).unwrap_or(u32::MAX);
                if run.kind == OperationKind::CutOut {
                    block.cut_out.push(CutOutItem {
                        item: fixture,
                        stacking,
                        leaves: vec![parts],
                    });
                } else {
                    block.leaves.push(LeafInput {
                        node: fixture,
                        stacking,
                        geometry: LeafGeometry::Outlines(parts),
                    });
                }
            }
            Some(block)
        })
        .collect()
}
