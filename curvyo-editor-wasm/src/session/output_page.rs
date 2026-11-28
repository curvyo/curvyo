//! Which machine family the active output page belongs to when it is not the laser (ADR 0098
//! decision 3): the one `match` the shared entry points (Operations tab, output group, play bar,
//! kind press, the unfolded row's fields) dispatch through.

use curvyo_document_core::NodeId;
use serde::Serialize;
use serde_json::Value;

use super::Session;
use super::cnc::CncPage;
use super::embroidery::EmbroideryPage;
use super::plotter::PlotterPage;

/// The active output page of a family with its own view, as read.
pub(in crate::session) enum FamilyPage {
    /// A plotter page.
    Plotter(PlotterPage),
    /// An embroidery page.
    Embroidery(EmbroideryPage),
    /// A CNC page.
    Cnc(CncPage),
}

impl FamilyPage {
    /// The page's manufacturing node.
    pub(in crate::session) const fn manufacturing(&self) -> NodeId {
        match self {
            Self::Plotter(p) => p.id,
            Self::Embroidery(p) => p.id,
            Self::Cnc(p) => p.id,
        }
    }
}

/// The unfolded body of a family row (ADR 0098 decision 3): the row's fields as the family view
/// computes them; data, not a form schema.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum FamilyBodyView {
    /// A plotter Cut or Draw: `pressure`, `speed`, `passes`.
    Plotter {
        /// The stored kind (`cut`, `draw`).
        kind: String,
        /// The numbers.
        fields: Value,
    },
    /// An embroidery row: the stitch numbers of its kind and its thread.
    Embroidery {
        /// The stored kind (`running`, `tatami`, `satin`).
        kind: String,
        /// The numbers.
        fields: Value,
    },
    /// A CNC Profile: side, direction, depth, tool, feed and plunge.
    Cnc {
        /// The numbers.
        fields: Value,
        /// The page's tools the Tool pick lists: `[{id, name, line2}]`.
        tools: Value,
    },
}

/// The stored kind a Select-bar button id names on a family page, `None` for another family's
/// button (ADR 0098 decision 3: the ids are the `KindView` names).
fn family_kind<'a>(family: &FamilyPage, button: &'a str) -> Option<&'a str> {
    match (family, button) {
        (FamilyPage::Plotter(_), "cut" | "draw") => Some(button),
        (FamilyPage::Embroidery(_), "running_stitch") => Some("running"),
        (FamilyPage::Embroidery(_), "tatami_fill") => Some("tatami"),
        (FamilyPage::Embroidery(_), "satin") => Some("satin"),
        (FamilyPage::Cnc(_), "profile_outside") => Some("outside"),
        (FamilyPage::Cnc(_), "profile_inside") => Some("inside"),
        _ => None,
    }
}

impl Session {
    /// The active page when it is a plotter, embroidery or CNC page with its extension on.
    pub(in crate::session) fn active_family(&self) -> Option<FamilyPage> {
        if let Some(page) = self.active_plotter() {
            return Some(FamilyPage::Plotter(page));
        }
        if let Some(page) = self.active_embroidery() {
            return Some(FamilyPage::Embroidery(page));
        }
        self.active_cnc().map(FamilyPage::Cnc)
    }

    /// The family view of `family` as the MVP builds it (rows, checks, total, play).
    pub(in crate::session) fn family_view(&self, family: &FamilyPage) -> Option<Value> {
        match family {
            FamilyPage::Plotter(_) => self.plotter_view(),
            FamilyPage::Embroidery(_) => self.embroidery_view(),
            FamilyPage::Cnc(_) => self.cnc_view(),
        }
    }

    /// A press on the Select-bar button `button` of a family page, `shift` held or not: the
    /// family's own press (ADR 0098 decision 3). `None` on a laser page, for another family's
    /// button, or when nothing changes.
    #[allow(clippy::option_option)] // outer: a family page or not; inner: the operation written
    pub(in crate::session) fn press_family_kind(
        &mut self,
        button: &str,
        shift: bool,
    ) -> Option<Option<NodeId>> {
        let family = self.active_family()?;
        let Some(kind) = family_kind(&family, button) else {
            return Some(None);
        };
        Some(match family {
            FamilyPage::Plotter(_) => self.press_plotter_kind(kind, shift),
            FamilyPage::Embroidery(_) => self.press_embroidery_kind(kind, shift),
            FamilyPage::Cnc(_) => self.press_cnc_profile(kind, shift),
        })
    }

    /// Types `text` into the field `field` of family operation `id`'s unfolded row (the
    /// family's field names; `thread` on embroidery takes "#RRGGBB"). One step of the family's
    /// edit. `Ok(false)` on a laser page or when nothing changes.
    ///
    /// # Errors
    /// The field message of a value that does not read.
    pub fn edit_operation_field(
        &mut self,
        id: NodeId,
        field: &str,
        text: &str,
    ) -> Result<bool, String> {
        match self.active_family() {
            Some(FamilyPage::Plotter(_)) => self.edit_plotter_setting(id, field, text),
            Some(FamilyPage::Embroidery(_)) if field == "thread" => {
                Ok(self.set_embroidery_thread(id, text))
            }
            Some(FamilyPage::Embroidery(_)) => self.edit_embroidery_setting(id, field, text),
            Some(FamilyPage::Cnc(_)) => self.edit_cnc_setting(id, field, text),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Point, RectBounds};

    use super::super::operations::KindView;
    use super::*;

    fn tab_names(s: &Session) -> Vec<&'static str> {
        s.panel_view().tabs.iter().map(|t| t.name).collect()
    }

    fn square(s: &mut Session) -> NodeId {
        let page = s.active_page();
        let (x, y) = (
            page.origin.x + page.size.width.as_mm() / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0,
        );
        s.document.create_rect(RectBounds::from_corners(
            Point::new(x - 10.0, y - 10.0),
            Point::new(x + 10.0, y + 10.0),
        ))
    }

    #[test]
    fn the_button_ids_map_to_each_familys_stored_kind() {
        let mut s = Session::new(98_003);
        s.add_page_of_kind("curvyo.embroidery", None).unwrap();
        let family = s.active_family().unwrap();
        assert_eq!(family_kind(&family, "tatami_fill"), Some("tatami"));
        assert_eq!(family_kind(&family, "cut"), None);
        s.add_page_of_kind("curvyo.cnc", None).unwrap();
        let family = s.active_family().unwrap();
        assert_eq!(family_kind(&family, "profile_inside"), Some("inside"));
    }

    /// ADR 0098 decision 2: every family page has Operations, Fixtures and Machines after
    /// Document; Tools only on CNC; Materials only on the laser.
    #[test]
    fn every_family_page_has_the_output_tabs_of_its_process() {
        let mut s = Session::new(98_004);
        for (process, tools, materials) in [
            ("curvyo.cutter", false, false),
            ("curvyo.embroidery", false, false),
            ("curvyo.cnc", true, false),
            ("curvyo.laser", false, true),
        ] {
            s.add_page_of_kind(process, None).unwrap();
            let tabs = tab_names(&s);
            let at = tabs.iter().position(|t| *t == "document").unwrap();
            assert_eq!(tabs[at + 1], "operations", "{process}");
            assert!(
                tabs.contains(&"fixtures") && tabs.contains(&"machines"),
                "{process}"
            );
            assert_eq!(tabs.contains(&"tools"), tools, "{process}");
            assert_eq!(tabs.contains(&"materials"), materials, "{process}");
            assert!(s.press_panel_tab("machines"), "{process}");
            assert_eq!(s.press_panel_tab("tools"), tools, "{process}");
        }
    }

    /// ADR 0098 decision 3: a family page answers the laser's Operations tab, Select bar and
    /// output group, and a Select-bar press assigns with the family's kind.
    #[test]
    fn an_embroidery_page_answers_the_shared_views() {
        let mut s = Session::new(98_005);
        s.add_page_of_kind("curvyo.embroidery", None).unwrap();
        let a = square(&mut s);
        s.selection.set(&[a]);
        let view = s.operations_view().unwrap();
        let kinds: Vec<KindView> = view
            .select_bar_kinds
            .unwrap()
            .iter()
            .map(|k| k.kind)
            .collect();
        assert_eq!(
            kinds,
            [
                KindView::RunningStitch,
                KindView::TatamiFill,
                KindView::Satin
            ]
        );
        assert_eq!(view.cuts_last, None);
        let op = s.press_operation_kind("tatami_fill", false).unwrap();
        let view = s.operations_view().unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].kind, KindView::TatamiFill);
        assert!(view.rows[0].thread.is_some() && view.rows[0].body.is_some());
        assert!(s.rename_operation(op, "Leaves"));
        assert!(s.unfold_operation_row(op));
        assert!(s.operations_view().unwrap().rows[0].unfolded);
        let group = s.output_group_view().unwrap();
        assert_eq!(group.machine, "W6 EU7 · DST");
        assert_eq!(group.export.extension, "dst");
        assert!(group.export.enabled, "{:?}", group.checks);
        assert_eq!((group.start, group.frame), (None, None));
        assert_eq!(
            s.edit_operation_field(op, "row_spacing_mm", "0.6"),
            Ok(true)
        );
        assert!(s.delete_operation(op));
        assert_eq!(s.operations_view().unwrap().rows.len(), 0);
    }

    /// ADR 0098 decision 3: the canvas play bar plays a CNC page on the planner's time.
    #[test]
    fn a_cnc_page_plays_on_the_shared_play_bar() {
        let mut s = Session::new(98_006);
        s.add_page_of_kind("curvyo.cnc", None).unwrap();
        let a = square(&mut s);
        s.selection.set(&[a]);
        s.press_operation_kind("profile_outside", false).unwrap();
        let page = s.active_page().id;
        assert!(s.toggle_toolpath_view(&page));
        let play = s.play_view().unwrap();
        assert!(play.enabled && play.readout.contains(" of "));
        s.set_play_position(0.5);
        let half = s.play_view().unwrap();
        assert!((half.fraction - 0.5).abs() < 0.01, "{}", half.fraction);
    }
}
