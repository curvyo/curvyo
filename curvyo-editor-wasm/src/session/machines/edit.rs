//! The Machines tab's edits (ADR 0098 decisions 4 and 5, Frage 434 C): a field, a duplicate, a
//! rename or a delete of a machine record of any family, and the inline offer to update the
//! open project's pages that used the record as it was.

use curvyo_document_core::{DocumentSize, NodeId, ProfileChange};
use curvyo_laser_core::{LibraryWrite, ProfileCopy, clamp_name, new_record_id};

use super::super::Session;
use super::{Family, FollowView};

/// One page's Update: its copy, size, anchor and the `0150` fills (laser only).
type FollowWrite = (
    NodeId,
    serde_json::Map<String, serde_json::Value>,
    DocumentSize,
    curvyo_document_core::PageAnchor,
    Vec<curvyo_document_core::SettingsWrite>,
);

/// The offer after an edit: the record and the pages that used it as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) struct FollowOffer {
    /// The edited record.
    pub(in crate::session) id: String,
    /// Its name, for the sentence.
    pub(in crate::session) name: String,
    /// The pages offered.
    pub(in crate::session) pages: Vec<NodeId>,
}

impl Session {
    /// The family whose library holds the record `id` (ids are unique across families).
    pub(in crate::session) fn family_of_record(&self, id: &str) -> Option<Family> {
        if self.manufacturing.machine.profiles.find(id).is_some() {
            Some(Family::Laser)
        } else if self.plotter_profile(id).is_some() {
            Some(Family::Plotter)
        } else if self.embroidery_profile(id).is_some() {
            Some(Family::Embroidery)
        } else if self.cnc_profile(id).is_some() {
            Some(Family::Cnc)
        } else {
            None
        }
    }

    /// Sets the field `key` of the maker's machine `id` from the typed `text` (`on`/`off` for a
    /// switch, the corner name for Origin): a library write, then the offer to update the pages
    /// that used the machine as it was (Frage 434 C). `Ok` with the notice of a clamped value;
    /// `Ok(None)` when nothing changed.
    ///
    /// # Errors
    /// The field message; empty for a built-in, an unknown id or an unknown field.
    pub(super) fn edit_machine_record(
        &mut self,
        id: &str,
        key: &str,
        text: &str,
        now_ms: i64,
    ) -> Result<Option<String>, String> {
        let family = self.family_of_record(id).ok_or_else(String::new)?;
        let offered = self.pages_matching(family, id);
        let notice = match family {
            Family::Laser => {
                let p = self
                    .manufacturing
                    .machine
                    .profiles
                    .find(id)
                    .filter(|p| !p.is_builtin())
                    .cloned()
                    .ok_or_else(String::new)?;
                let (next, notice) = super::values::edit_laser(p.clone(), key, text)?;
                if next == p {
                    return Ok(None);
                }
                let write = self
                    .manufacturing
                    .machine
                    .profiles
                    .update(id, next.values, next.work_area, now_ms)
                    .ok_or_else(String::new)?;
                self.manufacturing.machine.push_library_write(write);
                notice
            }
            Family::Plotter => {
                let mut p = self.user_plotter(id)?;
                let values = super::values::edit_plotter(p.values.clone(), key, text)?;
                if values == p.values {
                    return Ok(None);
                }
                p.values = values;
                p.modified = now_ms;
                let write = LibraryWrite::Write {
                    id: p.id.clone(),
                    text: curvyo_cutter_core::write_record(&p),
                };
                self.manufacturing.plotter.put(p);
                self.manufacturing.machine.queue(write);
                None
            }
            Family::Embroidery => {
                let mut p = self.user_embroidery(id)?;
                let speed = super::values::whole(text)
                    .ok()
                    .and_then(curvyo_embroidery_core::StitchesPerMin::new)
                    .filter(|_| key == "speed")
                    .ok_or_else(|| super::super::embroidery::commands::SPEED_MESSAGE.to_owned())?;
                if speed == p.values.speed {
                    return Ok(None);
                }
                p.values.speed = speed;
                p.modified = now_ms;
                let write = LibraryWrite::Write {
                    id: p.id.clone(),
                    text: curvyo_embroidery_core::write_record(&p),
                };
                self.manufacturing.embroidery.put(p);
                self.manufacturing.machine.queue(write);
                None
            }
            Family::Cnc => {
                let mut p = self.user_cnc(id)?;
                let values = super::values::edit_cnc(p.values.clone(), key, text)?;
                if values == p.values {
                    return Ok(None);
                }
                p.values = values;
                p.modified = now_ms;
                let write = LibraryWrite::Write {
                    id: p.id.clone(),
                    text: curvyo_cnc_core::write_record(&p),
                };
                self.manufacturing.cnc.put(p);
                self.manufacturing.machine.queue(write);
                None
            }
        };
        let name = self.record_name(id).unwrap_or_default();
        let pages: Vec<NodeId> = offered
            .into_iter()
            .filter(|page| self.page_differs(family, *page, id))
            .collect();
        self.machines_follow = (!pages.is_empty()).then(|| FollowOffer {
            id: id.to_owned(),
            name,
            pages,
        });
        Ok(notice)
    }

    fn user_plotter(&self, id: &str) -> Result<curvyo_cutter_core::CutterProfile, String> {
        self.manufacturing
            .plotter
            .find(id)
            .cloned()
            .ok_or_else(String::new)
    }

    fn user_embroidery(
        &self,
        id: &str,
    ) -> Result<curvyo_embroidery_core::EmbroideryProfile, String> {
        self.manufacturing
            .embroidery
            .find(id)
            .cloned()
            .ok_or_else(String::new)
    }

    fn user_cnc(&self, id: &str) -> Result<curvyo_cnc_core::CncProfile, String> {
        self.manufacturing
            .cnc
            .find(id)
            .cloned()
            .ok_or_else(String::new)
    }

    /// The name of the record `id` of any family.
    pub(super) fn record_name(&self, id: &str) -> Option<String> {
        Some(match self.family_of_record(id)? {
            Family::Laser => self
                .manufacturing
                .machine
                .profiles
                .find(id)?
                .values
                .name
                .clone(),
            Family::Plotter => self.plotter_profile(id)?.values.name,
            Family::Embroidery => self.embroidery_profile(id)?.values.name,
            Family::Cnc => self.cnc_profile(id)?.values.name,
        })
    }

    /// The output pages of the open project whose copy names the record `id` and equals it now.
    fn pages_matching(&self, family: Family, id: &str) -> Vec<NodeId> {
        self.document
            .manufacturings()
            .into_iter()
            .filter(|m| m.process == family.process())
            .filter(|m| copy_id(family, m).as_deref() == Some(id))
            .filter(|m| !self.page_differs(family, m.id, id))
            .map(|m| m.id)
            .collect()
    }

    /// Whether the page `page`'s copy differs from the record `id` (ADR 0098 R3).
    pub(in crate::session) fn page_differs(&self, family: Family, page: NodeId, id: &str) -> bool {
        let Some(m) = self.document.manufacturing(page) else {
            return false;
        };
        match family {
            Family::Laser => {
                let read = curvyo_laser_core::read_copy(
                    m.profile.as_ref(),
                    m.legacy_machine_name.as_deref(),
                );
                let (Some(record), Some(size)) = (
                    self.manufacturing.machine.profiles.find(id),
                    m.page.map(|p| p.size),
                ) else {
                    return false;
                };
                curvyo_laser_core::differs(&read.copy, size, record)
            }
            Family::Plotter => match (
                super::super::plotter::plotter_page(&m),
                self.plotter_profile(id),
            ) {
                (Some(p), Some(r)) => curvyo_cutter_core::differs(&p.copy, &r),
                _ => false,
            },
            Family::Embroidery => {
                match (
                    super::super::embroidery::embroidery_page(&m),
                    self.embroidery_profile(id),
                ) {
                    (Some(p), Some(r)) => curvyo_embroidery_core::differs(&p.copy, &r),
                    _ => false,
                }
            }
            Family::Cnc => match (super::super::cnc::cnc_page(&m), self.cnc_profile(id)) {
                (Some(p), Some(r)) => curvyo_cnc_core::differs(&p.copy, &r),
                _ => false,
            },
        }
    }

    /// The offer's view; it goes by itself once no offered page differs (an undo).
    pub(super) fn follow_view(&self) -> Option<FollowView> {
        let offer = self.machines_follow.as_ref()?;
        let family = self.family_of_record(&offer.id)?;
        let n = offer
            .pages
            .iter()
            .filter(|p| self.page_differs(family, **p, &offer.id))
            .count();
        (n > 0).then(|| FollowView {
            id: offer.id.clone(),
            sentence: if n == 1 {
                format!("1 page uses {}. Update it too?", offer.name)
            } else {
                format!("{n} pages use {}. Update them too?", offer.name)
            },
            pages: n,
        })
    }

    /// Keep pages (UX unify 4.5): the offer closes; the pages show "Differs from profile".
    pub fn keep_machine_pages(&mut self) {
        self.machines_follow = None;
    }

    /// Update pages (Frage 434 C, ADR 0098 R4): every offered page that still differs takes the
    /// record's copy through its family's writer, in one step "Update `n` pages". A page whose
    /// new size cannot be laid out keeps its copy. Whether a page changed.
    pub fn follow_machine(&mut self) -> bool {
        let Some(offer) = self.machines_follow.take() else {
            return false;
        };
        let Some(family) = self.family_of_record(&offer.id) else {
            return false;
        };
        let pages: Vec<NodeId> = offer
            .pages
            .iter()
            .copied()
            .filter(|p| self.page_differs(family, *p, &offer.id))
            .collect();
        if pages.is_empty() {
            return false;
        }
        let contents = self.page_contents().unwrap_or_default();
        let writes: Vec<FollowWrite> = pages
            .iter()
            .filter_map(|page| self.follow_write(family, *page, &offer.id))
            .collect();
        let label = format!(
            "Update {} {}",
            writes.len(),
            if writes.len() == 1 { "page" } else { "pages" }
        );
        self.recording_named_counting(label, None, |this| {
            this.document.batch("update_pages", |d| {
                let mut any = false;
                for (page, map, size, anchor, fills) in &writes {
                    any |= d
                        .set_manufacturing_profile_filled(
                            *page,
                            map,
                            *size,
                            *anchor,
                            ProfileChange::Reset,
                            None,
                            fills,
                            &contents,
                        )
                        .unwrap_or(false);
                }
                any
            })
        })
    }

    /// The copy, page size and anchor the page `page` takes from the record `id`.
    fn follow_write(&self, family: Family, page: NodeId, id: &str) -> Option<FollowWrite> {
        let m = self.document.manufacturing(page)?;
        let size = m.page.as_ref()?.size;
        Some(match family {
            Family::Laser => {
                let record = self.manufacturing.machine.profiles.find(id)?;
                let copy = ProfileCopy::of(record);
                let origin = copy.values.origin;
                let anchor = curvyo_document_core::PageCorner {
                    right: origin.is_right(),
                    bottom: origin.is_bottom(),
                };
                // The `0150` fills: operations without their own numbers keep what they read
                // (as `write_active_copy`).
                let before = curvyo_laser_core::read_copy(
                    m.profile.as_ref(),
                    m.legacy_machine_name.as_deref(),
                );
                let fills = if before.copy.values.defaults == copy.values.defaults {
                    Vec::new()
                } else {
                    self.derived_fills(Some(page))
                };
                (
                    page,
                    curvyo_laser_core::write_copy(&copy),
                    record.work_area,
                    anchor.into(),
                    fills,
                )
            }
            Family::Plotter => {
                let record = self.plotter_profile(id)?;
                let copy = curvyo_cutter_core::CutterCopy::of(&record);
                let size = if copy.values.mats.is_empty() {
                    copy.values.page_size()
                } else {
                    size
                };
                let anchor = curvyo_cutter_core::resize_anchor(&copy);
                (
                    page,
                    curvyo_cutter_core::write_copy(&copy),
                    size,
                    anchor,
                    Vec::new(),
                )
            }
            Family::Embroidery => {
                let record = self.embroidery_profile(id)?;
                let copy = curvyo_embroidery_core::EmbroideryCopy::of(&record);
                let anchor = curvyo_embroidery_core::resize_anchor(&copy);
                (
                    page,
                    curvyo_embroidery_core::write_copy(&copy),
                    size,
                    anchor,
                    Vec::new(),
                )
            }
            Family::Cnc => {
                let record = self.cnc_profile(id)?;
                let copy = curvyo_cnc_core::CncCopy::of(&record);
                let anchor = curvyo_cnc_core::resize_anchor(&copy);
                let size = copy.values.work_area;
                (
                    page,
                    curvyo_cnc_core::write_copy(&copy),
                    size,
                    anchor,
                    Vec::new(),
                )
            }
        })
    }

    /// Duplicate (UX unify 4.2, "+ Machine" too): a copy of the machine `id` of any family,
    /// "`name` copy", added to its library. The new id.
    pub(super) fn duplicate_machine_record(
        &mut self,
        id: &str,
        random: [u8; 16],
        now_ms: i64,
    ) -> Option<String> {
        let new_id = new_record_id(random);
        match self.family_of_record(id)? {
            Family::Laser => return self.duplicate_machine_profile(id, random, now_ms),
            Family::Plotter => {
                let mut p = self.plotter_profile(id)?;
                p.id.clone_from(&new_id);
                p.values.name = format!("{} copy", p.values.name);
                p.modified = now_ms;
                let text = curvyo_cutter_core::write_record(&p);
                self.manufacturing.plotter.put(p);
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: new_id.clone(),
                    text,
                });
            }
            Family::Embroidery => {
                let mut p = self.embroidery_profile(id)?;
                p.id.clone_from(&new_id);
                p.values.name = format!("{} copy", p.values.name);
                p.modified = now_ms;
                let text = curvyo_embroidery_core::write_record(&p);
                self.manufacturing.embroidery.put(p);
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: new_id.clone(),
                    text,
                });
            }
            Family::Cnc => {
                let mut p = self.cnc_profile(id)?;
                p.id.clone_from(&new_id);
                p.values.name = format!("{} copy", p.values.name);
                p.modified = now_ms;
                let text = curvyo_cnc_core::write_record(&p);
                self.manufacturing.cnc.put(p);
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: new_id.clone(),
                    text,
                });
            }
        }
        Some(new_id)
    }

    /// Renames the maker's machine `id` of any family (1 to 60 characters); an empty name changes
    /// nothing. Whether it changed.
    pub(super) fn rename_machine_record(&mut self, id: &str, typed: &str, now_ms: i64) -> bool {
        let Some(limited) = clamp_name(typed) else {
            return false;
        };
        let name = limited.value;
        match self.family_of_record(id) {
            Some(Family::Laser) => {
                let user = self.manufacturing.machine.profiles.is_user(id);
                if user {
                    let _ = self.rename_machine_profile(id, &name, now_ms);
                }
                user
            }
            Some(Family::Plotter) => self.user_plotter(id).is_ok_and(|mut p| {
                p.values.name = name;
                p.modified = now_ms;
                let text = curvyo_cutter_core::write_record(&p);
                self.manufacturing.plotter.put(p);
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: id.to_owned(),
                    text,
                });
                true
            }),
            Some(Family::Embroidery) => self.user_embroidery(id).is_ok_and(|mut p| {
                p.values.name = name;
                p.modified = now_ms;
                let text = curvyo_embroidery_core::write_record(&p);
                self.manufacturing.embroidery.put(p);
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: id.to_owned(),
                    text,
                });
                true
            }),
            Some(Family::Cnc) => self.user_cnc(id).is_ok_and(|mut p| {
                p.values.name = name;
                p.modified = now_ms;
                let text = curvyo_cnc_core::write_record(&p);
                self.manufacturing.cnc.put(p);
                self.manufacturing.machine.queue(LibraryWrite::Write {
                    id: id.to_owned(),
                    text,
                });
                true
            }),
            None => false,
        }
    }

    /// Deletes the maker's machine `id` of any family; pages keep their copy and show "Not in
    /// your machines" (ADR 0098 R2). Whether it was deleted.
    pub(super) fn delete_machine_record(&mut self, id: &str) -> bool {
        let removed = match self.family_of_record(id) {
            Some(Family::Laser) => return self.delete_machine_profile(id),
            Some(Family::Plotter) => {
                let before = self.manufacturing.plotter.profiles.len();
                self.manufacturing.plotter.profiles.retain(|p| p.id != id);
                before != self.manufacturing.plotter.profiles.len()
            }
            Some(Family::Embroidery) => {
                let before = self.manufacturing.embroidery.profiles.len();
                self.manufacturing
                    .embroidery
                    .profiles
                    .retain(|p| p.id != id);
                before != self.manufacturing.embroidery.profiles.len()
            }
            Some(Family::Cnc) => {
                let before = self.manufacturing.cnc.profiles.len();
                self.manufacturing.cnc.profiles.retain(|p| p.id != id);
                before != self.manufacturing.cnc.profiles.len()
            }
            None => false,
        };
        if removed {
            self.manufacturing
                .machine
                .queue(LibraryWrite::Remove { id: id.to_owned() });
        }
        removed
    }
}

/// The record id the copy of the page `m` names.
pub(super) fn copy_id(
    family: Family,
    m: &curvyo_document_core::ManufacturingSnapshot,
) -> Option<String> {
    match family {
        Family::Laser => {
            curvyo_laser_core::read_copy(m.profile.as_ref(), m.legacy_machine_name.as_deref())
                .copy
                .id
        }
        Family::Plotter => super::super::plotter::plotter_page(m)?.copy.id,
        Family::Embroidery => super::super::embroidery::embroidery_page(m)?.copy.id,
        Family::Cnc => super::super::cnc::cnc_page(m)?.copy.id,
    }
}

/// The machine name the copy of the page `m` holds.
pub(super) fn copy_name(
    family: Family,
    m: &curvyo_document_core::ManufacturingSnapshot,
) -> Option<String> {
    Some(match family {
        Family::Laser => {
            curvyo_laser_core::read_copy(m.profile.as_ref(), m.legacy_machine_name.as_deref())
                .copy
                .values
                .name
        }
        Family::Plotter => super::super::plotter::plotter_page(m)?.copy.values.name,
        Family::Embroidery => {
            super::super::embroidery::embroidery_page(m)?
                .copy
                .values
                .name
        }
        Family::Cnc => super::super::cnc::cnc_page(m)?.copy.values.name,
    })
}
