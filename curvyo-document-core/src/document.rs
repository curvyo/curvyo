//! The document model's root: a Loro-backed replica carrying exactly the
//! fields this slice needs — `format_version` and `size` — as per-field
//! last-writer-wins registers (ADR 0009 §3; ADR 0002 §1, §9; ADR 0004 §2,
//! §3; `specs/0001-project-file-foundation/adrs.md`, "a minimal document root
//! record, and no more").

use loro::{ExportMode, LoroDoc, LoroMap, LoroValue};
use serde::Serialize;

use crate::display_unit::DisplayUnit;
use crate::document_size::{MAX_DOCUMENT_MM, MIN_DOCUMENT_MM};
use crate::error::{OpenError, SaveError};
use crate::units::{DocumentSize, Length};

/// An opaque marker of a document's state, see [`Document::version`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentVersion(pub(crate) loro::Frontiers);

impl DocumentVersion {
    /// A version no document ever has (every document has its setup change): a project that
    /// exists in no saved file, such as one opened from an SVG, is dirty against it.
    #[must_use]
    pub fn never() -> Self {
        Self(loro::Frontiers::default())
    }

    /// The version as text, `peer@counter` pairs sorted and joined by commas (ADR 0014 §7); the
    /// empty text for [`DocumentVersion::never`]. A shared file stores its synced version so
    /// (ADR 0031 decision 9).
    #[must_use]
    pub fn to_text(&self) -> String {
        crate::history::encode_frontiers(&self.0)
    }

    /// The inverse of [`DocumentVersion::to_text`]; `None` for a malformed text.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        if text.is_empty() {
            return Some(Self::never());
        }
        crate::history::decode_frontiers(text)
            .filter(|frontiers| frontiers.iter().all(|id| id.counter >= 0))
            .map(Self)
    }
}

/// The container `format_version` this build writes and the newest it
/// accepts on read (ADR 0004 §9).
///
/// Bumped to 3 in `primitive-shapes`: a version-2 reader would refuse a
/// primitive node as `OpenError::Damaged` (it has no `anchors`), which is
/// the wrong message for a file this build can actually read — ADR 0004
/// §9 wants "newer version" instead (`specs/0003-primitive-shapes/adrs.md`,
/// "`format_version` goes to 3"). Migration from version 2 is empty by
/// construction: absent `shape` means path, so every version-2 path node
/// opens unchanged.
///
/// Bumped to 4 in `path-merge-split-and-node-types` (`specs/0006-path-
/// merge-split-and-node-types/adrs.md`, architect's 2026-10-05
/// resolution): this slice merges before `object-transform` has a
/// branch at all, so it takes the next free version outright —
/// `object-transform` is sequenced after it and takes 5,
/// `stroke-and-fill-styling` takes 6. The standing rule going forward:
/// a version number named in an `adrs.md` is provisional until its PR
/// actually merges; whichever one merges first takes `main`'s
/// `CURRENT_FORMAT_VERSION + 1`, and a later PR rebases and renumbers
/// itself, its fixtures and its own notes. A version-3 reader maps any
/// unknown `kind` string to `Corner` (slice 2's lenient read), so it
/// would open a version-4 file with `Asymmetric` anchors and silently
/// turn them into corners — the silent partial read ADR 0004 §9
/// forbids. Migration from version 3 is the `"smooth"` → `Symmetric`
/// read alias in `path_codec::read_kind` and nothing else: no anchor's
/// stored shape changes, only the kind tag's legacy spelling is now
/// also understood.
///
/// Bumped to 5 in `object-transform`, exactly as this comment's own
/// previous paragraph already anticipated: it is the first slice to
/// merge after this one, so it takes `CURRENT_FORMAT_VERSION + 1`
/// outright, no renumbering needed. `document.json` and the Loro root
/// gain a `rotation` key on each object (`specs/0005-object-transform/
/// adrs.md`). A version-4 reader has no `rotation` key at all, so it
/// would open a rotated object and draw it unrotated — the silent
/// partial read ADR 0004 §9 forbids. Migration from version 4 is empty
/// by construction: absent `rotation` reads as `0`
/// (`path_codec::read_rotation`), so every version-4 object opens with
/// zero rotation, unchanged.
///
/// Bumped to 6 in `rectangle-corner-radii` (`specs/0013-rectangle-corner-radii/
/// adrs.md`, decisions 2 and 3): a rectangle stores four corner radius
/// registers (`corner_radius_tl`, `_tr`, `_br`, `_bl`) and the single
/// `corner_radius` key is no longer written. A version-5 reader would refuse
/// every rectangle this build writes as `OpenError::Damaged` (no
/// `corner_radius`), the wrong message; with differing radii it must not open
/// the file at all. The bump makes it say "saved by a newer version", for
/// every file this build saves. Migration from version 5 is empty: each corner
/// reads its own key, else the legacy `corner_radius`, else the file is
/// damaged, and an old file is not rewritten on open. The number is
/// provisional by the rule above: whichever PR merges takes `main`'s current
/// number plus one.
///
/// Bumped to 7 in `stroke-and-fill-styling` (`specs/0007-stroke-and-fill-
/// styling/adrs.md`, "`format_version`" and the 2026-10-07 readiness check):
/// every object node may carry the style keys of `crate::style_codec`
/// (`stroke_enabled`, `stroke_opacity`, `stroke_dash`, `stroke_join`,
/// `stroke_cap`, `fill_enabled`, `fill`, `fill_opacity`; a build of that
/// version may also have written the two keys `crate::legacy_fill` reads past),
/// and `document.json` writes one `style` object per object. A version-6
/// reader tolerates unknown keys, so it would open a version-7 file and draw
/// every dash and fill as a thin black outline, the silent partial read ADR 0004 §9 forbids; with the bump it
/// says "saved by a newer version". Migration from version 6 is empty by
/// construction: every new key is absent in an older file and an absent key
/// reads as the frozen default, which is what older builds drew (0.25 mm
/// solid black stroke, no fill), and an old file is not rewritten on open.
/// The stroke `stroke_width` and `stroke` keys keep their meaning. This is
/// the complete format of the slice: later parts of the story add no key
/// and need no further bump. The number is provisional by the rule above.
///
/// The fill keys `fill_kind` and `fill_stops` of a version-7 file are not part
/// of the format any more (`specs/0017-style-panel-rework/adrs.md`, decision 1):
/// `crate::legacy_fill` reads past them and drops them on the next fill write,
/// which needs no bump because nothing is misread in either direction.
///
/// Bumped to 8 in `boolean-operations` (`specs/0016-boolean-operations/
/// adrs.md`, "compound path"): a path may hold several outlines, the first in
/// the ordinary `closed`/`anchors` keys and the others in one new optional
/// movable list, `extra_subpaths` (`crate::subpath_codec`). A build that stops
/// at version 7 would read such a path as its first outline alone and drop
/// the rest without a word, the silent partial read ADR 0004 §9 forbids, so
/// it must refuse the file as too new. Migration from version 7 is none: the
/// key is simply absent, and a file from an earlier build opens unchanged and
/// is not rewritten. Golden: `compound_v8.curvyo`.
///
/// Bumped to 9 in `style-panel-rework` (`specs/0017-style-panel-rework/
/// adrs.md`, decision 2): a stroke dash list may have an odd count (`1 2 4`),
/// stored as typed. A build that stops at version 8 refuses an odd list as a
/// damaged file, the wrong message, so it must refuse the file as too new.
/// Migration from version 8 is none: every stored even list stays valid, and
/// a file from an earlier build opens unchanged and is not rewritten. Golden:
/// `dash_v9.curvyo`.
///
/// Version 9 also carries the five stroke marker keys of `stroke-markers`
/// (`specs/0018-stroke-markers/adrs.md`, decision 1): the two features share
/// one bump because they merge in one pull request. A build that stops at
/// version 8 would ignore the keys and draw no markers, a silent partial read,
/// so it must refuse the file as too new. Migration from version 8 is none:
/// absent keys read as None, Spaced and 1. Golden: `markers_v9.curvyo`.
///
/// Bumped to 10 in `document-background` (`specs/0040-document-background/
/// adrs.md`, decisions 2 and 3): the document root may carry the two registers
/// `background_paint` (`"none"` or `"solid"`) and `background_color` (one list
/// `[r, g, b, a]`). A build that stops at version 9 would ignore them and draw the
/// default grey for a red document, a silent partial read, so it must refuse the
/// file as too new. Migration from version 9 is none: absent registers read as
/// solid `#E8E8EB`, and a file from an earlier build opens unchanged and is not
/// rewritten. A present register that is malformed is refused as damaged
/// (`crate::document_background`). The number is provisional by the rule above:
/// whichever PR merges first takes `main`'s number plus one. Golden:
/// `background_v10.curvyo`.
///
/// Bumped to 11 in `groups` (`specs/0023-groups/adrs.md`, "the stored shape of a group"): an
/// object-tree node may have children, and a node whose meta map holds `group: true` is a group
/// (no style, no anchors, no shape tag; its children are its members in z-order, at most 32
/// groups on one chain). A build that stops at version 10 would read the group node as a path
/// without anchors and the members as objects nested under it that it never draws, a silent
/// partial read, so it must refuse the file as too new. Migration from version 10 is none: a file
/// from an earlier build is a tree of depth 0 and opens unchanged and is not rewritten. A group
/// nested deeper than 32, a leaf with children, and a group that carries anchors or a shape tag
/// are refused as damaged (`crate::groups`). Golden: `groups_v11.curvyo`.
///
/// Bumped to 12 in `layers-and-objects-panel` (`specs/0050-layers-and-objects-panel/adrs.md`
/// decisions 1 and 15): every object-tree node may carry the registers `name` (string), `visible`
/// and `locked` (bool) and `opacity` (number in 0..=1), and a top-level group may carry
/// `layer: true`. A build that stops at version 11 would draw a hidden node, ignore a lock and an
/// opacity, and lose them on its next save, a silent partial read, so it must refuse every file of
/// this build as too new. Migration from version 11 is none: absent registers read as no name,
/// visible, unlocked and opacity 1, and a file from an earlier build opens unchanged and is not
/// rewritten. A mistyped register, an opacity outside 0..=1 and `layer` on a leaf are refused as
/// damaged (`crate::node_registers`); a `layer` marker below the top level reads as a plain group.
/// Golden: `layers_v12.curvyo`.
/// Bumped to 13 in `linked-clone` (`specs/0049-linked-clone/adrs.md` decisions 1 and 11): a leaf
/// may be a clone, `shape = "clone"` with one `clone_link` register `{origin, matrix}` and no
/// outline of its own. A build that stops at version 12 would read the clone as a primitive of an
/// unknown shape and refuse it as damaged, or draw nothing where the maker sees an object, so it
/// must refuse the file as too new. Migration from version 12 is none: a file from an earlier
/// build has no clone and opens unchanged and is not rewritten. A clone without a readable link,
/// or with a matrix entry that is not finite or lies beyond ±1e7, is refused as damaged; a clone
/// whose original is missing or of the wrong kind is an orphan, not damage. Golden: `clone_v13.curvyo`.
///
/// Bumped to 14 in `clone-overrides` (`specs/0057-clone-overrides`, ADR 0016 §1 and §6): a clone
/// may carry `own_placement: true` and `own_style: true`, and with `own_style` the ordinary style
/// keys and `opacity`. A build that stops at version 13 would draw a clone with own style in its
/// original's colours (wrong thread colours in an embroidery job) and would move a clone with own
/// placement along with its original, so it must refuse every file of this build as too new.
/// Migration from version 13 is none: absent flags read as false, and a file from an earlier build
/// opens unchanged and is not rewritten. A flag on a clone that is not a boolean, and a damaged
/// style on a clone with own style, are refused as damaged (`crate::clone_codec`); flags on other
/// nodes, and style keys or `opacity` on a clone without own style, are residue.
/// Golden: `clone_overrides_v14.curvyo`.
///
/// Bumped to 15 in `document-shapes` (`specs/0046-document-shapes`, ADR 0019 §2 and §4): a path,
/// compound path or shape may carry the register `guide: true`, which draws it in the marking
/// style and keeps it out of every output. A build that stops at version 14 would draw the guide
/// as an ordinary path and a laser job would cut the template, so it must refuse every file of
/// this build as too new. Migration from version 14 is none: an absent register reads as not a
/// guide, a file from an earlier build opens unchanged and is not rewritten, and unmarking removes
/// the key. A `guide` value that is not a boolean is refused as damaged
/// (`crate::node_registers`); the key on a group or a linked clone is residue (a clone reads its
/// root's flag). The number is provisional by the rule above. Golden: `guide_v15.curvyo`.
///
/// Bumped to 16 in `ellipse-arcs-and-shaping` (`specs/0021-ellipse-arcs-and-shaping/adrs.md`
/// decisions 1 to 3): an ellipse may carry the list register `ellipse_arc` = `[start°, end°,
/// kind]`, and an ellipse, polygon or star the register `curve` (a fraction from -4 to 4). A
/// build that stops at version 15 would cut an arc as a whole ellipse and a curved polygon with
/// straight sides, so it must refuse every file of this build as too new. Migration from version
/// 15 is none: absent registers read as the whole ellipse and the shape's default curve, and a
/// file from an earlier build opens unchanged and is not rewritten. A non-finite or out-of-range
/// curve, an arc that is not two finite angles and a known kind, and an arc together with a curve
/// other than 100 % are refused as damaged (`crate::shape_arc_codec`). Golden: `arc_pie_v16`,
/// `arc_arc_v16`, `arc_chord_v16`, `curve_ellipse_v16` and `curve_polygon_star_v16.curvyo`.
///
/// Bumped to 17 in `text-objects` (`specs/0074-text-objects`, ADR 0020 §1): a new leaf kind,
/// `shape = "text"`, with the registers `text_string` (a `LoroText`), `text_font` (one map value:
/// family, style, face hash, face index), `text_size` (mm, the capital H) and `text_placement`
/// (six numbers). A build that stops at version 16 does not know the kind, so it must refuse
/// every file of this build as too new. Migration from version 16 is none: a file without texts
/// opens unchanged and is not rewritten. A text with a missing or damaged register, a string over
/// 10,000 characters, a size outside 0.1 to 2,000 mm or a placement that is not finite and
/// invertible is refused as damaged (`crate::text_codec`). Golden: `text_v17.curvyo`.
///
/// Bumped to 18 in `single-line-fonts` (`specs/0076-single-line-fonts/adrs.md` decision 1, Frage
/// 21 SL-KIND A): `text_font` may carry the key `kind = "single_line"`. A build that stops at
/// version 17 ignores the key and could match an outline face of the same name, drawing the
/// letters filled without the "Font missing" mark, so it must refuse every file of this build as
/// too new. Migration from version 17 is none: no key reads as an outline face, and a file from an
/// earlier build opens unchanged. Any other value of `kind` reads as outline. Golden:
/// `text_single_line_v18.curvyo`.
///
/// Bumped to 19 in `text-layout-controls` (`specs/0079-text-layout-controls/adrs.md` decisions 1
/// and 2): a text may carry the optional keys `text_align` ("centre", "right", "justify"),
/// `text_letter_space`, `text_word_space`, `text_line_space` (factors of `text_size`) and
/// `text_rotation` (degrees). A build that stops at version 18 ignores them and would draw a
/// centred, spaced or turned text left-aligned, tight and upright without a word, so it must
/// refuse every file of this build as too new. Migration from version 18 is none: absent keys are
/// the defaults, and a value at its default is never written. An unknown alignment reads as
/// left, a non-finite number as its default, a factor out of range is clamped and a rotation is
/// wrapped; nothing of it is refused (`crate::text_codec`). The number is provisional by the rule
/// above (`0080` text on path takes the next one). Golden: `text_layout_v19.curvyo`.
///
/// Bumped to 20 in `text-vertical` (`specs/0079-text-layout-controls/adrs.md` decision 13): a
/// text may carry the optional keys `text_writing_mode` (`"vertical_rl"`, `"vertical_lr"`) and
/// `text_glyph_orientation` (`"upright"`, `"sideways"`). A build that stops at version 19 ignores them
/// and would draw a vertical text as one horizontal line without a word, so it must refuse every
/// file of this build as too new. Migration from version 19 is none: absent keys are horizontal
/// and auto, a value at its default is never written, an unknown code reads as the default. The
/// number is provisional by the rule above. Golden: `text_layout_v20.curvyo`.
///
/// Bumped to 21 in `text-kerning` (ADR 0021 §7): a text's string may carry the per-character marks
/// `kern_x` and `kern_y` (factors of the size, one mark per character). A build that stops at
/// version 20 reads the string without its marks and would draw a kerned text unkerned without a
/// word, so it must refuse every file of this build as too new. Migration from version 20 is none:
/// a string without marks reads as before. The number is provisional by the rule above. Golden:
/// `text_layout_v21.curvyo`.
///
/// Bumped to 22 in `text-on-path` (ADR 0022 §1 and §2): a text may carry the optional keys
/// `text_carrier`, `text_path_position`, `text_path_side`, `text_path_line`, `text_path_distance`,
/// `text_path_mirror` and `text_carrier_kept` (`crate::text_path_codec`), and a linked clone may
/// carry its own `guide` flag. A build that stops at version 21 ignores them and would draw every
/// text on path as a straight text and a guide clone as artwork (a cut) without a word, so it must
/// refuse every file of this build as too new. Migration from version 21 is none: absent keys are
/// a straight text. The number is provisional by the rule above. Golden: `text_on_path_v22.curvyo`.
///
/// Bumped to 23 in `text-on-several-paths` (ADR 0022 §4): a text may carry `text_source` (a text
/// clone, `crate::text_clone_codec`) and then no string, font, size or layout of its own, and with
/// `own_style` its own style keys. A build that stops at version 22 refuses such a node at open
/// (no string) or would drop it, so it must refuse every file of this build as too new.
/// Migration from version 22 is none: no text has a source. The number is provisional by the rule
/// above. Golden: `text_clones_v23.curvyo`.
///
/// Bumped to 24 in `text-taper` (`specs/0079-text-layout-controls/adrs.md` decision 7): a text may
/// carry the optional key `text_taper` (a fraction, −0.5 to 0.5). A build that stops at version 23
/// ignores it and would draw a tapered text untapered without a word, so it must refuse every file
/// of this build as too new. Migration from version 23 is none: an absent key is no taper, a taper
/// of 0 is never written, a non-finite one reads as 0 and one out of range is clamped. The number
/// is provisional by the rule above. Golden: `text_layout_v24.curvyo`.
///
/// Bumped to 25 in `bitmap-import` (ADR 0017 §1 to §3, `specs/0058-bitmap-import`): a new leaf
/// kind, `shape = "image"`, with the registers `image_asset` (the SHA-256 of the picture's bytes)
/// and `image_placement` (six numbers), and the container members `assets/<hash>.<ext>` that hold
/// the bytes. A build that stops at version 24 does not know the kind and would refuse the node
/// as an unknown shape or draw nothing where the picture is, so it must refuse every file of this
/// build as too new. Migration from version 24 is none: a file without images opens unchanged
/// and is not rewritten. An image whose hash or placement cannot be read is refused as damaged
/// (`crate::image_codec`); a missing, oversized or mismatching asset is not damage: the image
/// keeps its frame and shows as unreadable. The number is provisional by the rule above. Golden:
/// `image_v25.curvyo`.
///
/// Bumped to 26 in `multiple-pages` (ADR 0024): ordered spatial page frames
/// use independent root registers. Older readers must not hide extra frames.
/// Golden: `pages_v26.curvyo`; no old-file migration is required for this SID build.
///
/// Bumped to 27 in `snapping`: document-owned target, axis and zone registers preserve
/// enabled state, target kinds and capture/release zones.
/// Golden: `snapping_v27.curvyo`; no old-file migration is required for this SID build.
///
/// Bumped to 28 in `constraint-sketcher`: parametric objects keep atomic intent records
/// and independently validated solution checkpoints (ADR 0026).
/// Goldens: `sketch_*_v28.curvyo`; older readers refuse this format.
///
/// Bumped to 29 in `constraint-sketcher` customer revision 4 (`specs/0066-constraint-sketcher/
/// adrs.md`, 2026-10-31): sketch relations gain `HorizontalDistance` and `VerticalDistance`.
/// A version-28 reader does not know them and would refuse the sketch record as damaged, so it
/// must refuse every file of this build as too new. Migration from version 28 is none: no
/// version-28 record changes shape, and the version-28 goldens still open. Goldens:
/// `sketch_*_v29.curvyo`.
///
/// Bumped to 30 in `grid` (`specs/0090-grid`, `adrs.md` decisions 1 and 2): each page keeps
/// its grid as independent `page.<id>.grid_*` registers, and snapping gains the
/// `snapping_grid` target switch and the `snapping_grid_point` choice. Absent registers mean
/// the defaults and opening writes nothing; a malformed one is refused as damaged.
/// Golden: `grid_v30.curvyo`. Older files open with the defaults; old-file compatibility
/// waived (customer, 2026-10-30).
///
/// Bumped to 31 in `measure-and-dimension` (ADR 0027, `specs/0070-measure-and-dimension`): a new
/// leaf kind, `shape = "dimension"`, with the registers `dimension_kind`, `dimension_ref_a` to
/// `dimension_ref_c`, `dimension_place` and `dimension_kept` (`crate::dimension_codec`). A build
/// that stops at version 30 reads the tag as an unknown primitive and could output a guide as
/// artwork, so it must refuse every file of this build as too new. Migration from version 30 is
/// none: a file without dimensions opens unchanged and is not rewritten. Damage: an unknown kind,
/// a slot count that does not fit the kind, a malformed slot, a value that is not finite. The
/// number is provisional by the rule above. Goldens: `dimension_v31.curvyo` and
/// `dimension_*_v31.curvyo`.
///
/// Bumped to 32 in `pattern-fill` (ADR 0029, `specs/0071-pattern-fill`): the style keys
/// `fill_paint` and `fill_pattern` (`crate::pattern_codec`). A version-31 reader would paint a
/// patterned shape as a solid colour, so it must refuse every file of this build. Migration from
/// version 31 is none. Damage: an unknown paint or generator, a parameter of the wrong type,
/// non-finite or out of its range, a seed that is not an integer in u32. Golden:
/// `pattern_v32.curvyo`.
///
/// Bumped to 33 in `tiling` (ADR 0030, `specs/0069-tiling`): a tiling is a group with the
/// registers `tiling` (the recipe, one map value) and `tiling_sources`; its instances carry
/// `tiling_cell` and the members of a multi-source instance `tiling_member`
/// (`crate::tiling_codec`). A build that stops at version 32 would edit a tiling as a plain group
/// and lose the link between recipe and instances silently, so it must refuse every file of this
/// build as too new. Migration from version 32 is none: a file without tilings opens unchanged.
/// Damage: `tiling` on a node that is not a group, an unknown layout or result, a count of 0 or
/// over 100,000, a number that is not finite, a malformed source list, cell or member key. The
/// number is provisional by the rule above. Goldens: `tiling_*_v33.curvyo`.
///
/// Bumped to 34 in `cooperative-editing` (ADR 0031 decisions 9 and 9a, `specs/0092-cooperative-
/// editing`): a page's origin and size are one register `page.<id>.frame` (the map `{x, y,
/// width, height}`, written whole), so concurrent resizes converge to one whole frame; resize and
/// fit move the frame, not the objects (`crate::page_frame`). A shared file adds the container
/// members `share.json` and `keyring.log` (`crate::container_share`). A build that stops at 33
/// would read every page at its version 33 registers (or at A4 in the origin) and save a shared
/// file without its keyring, so it must refuse every file of this build as too new. Migration
/// from 33: a page without `frame` is read from the four old registers; nothing is rewritten until
/// the page is next edited. Damage: a frame that is not a map of exactly four finite numbers or
/// has a side outside the document size range; one shared member without the other, or a
/// malformed `share.json`. The number is provisional by the rule above. Goldens:
/// `page_frame_v34.curvyo`, `shared_v34.curvyo`, `page_frame_*_v34.curvyo`.
///
/// Bumped to 35 in `laser-workbench-shell` (ADR 0032, ADR 0033 §7,
/// `specs/0093-laser-workbench-shell`): a top-level node may be a manufacturing (`manufacturing: true`, `mfg_process`, `mfg_page`,
/// `mfg_machine_name`) whose children are placements (`shape = "placement"`, `placement_link`) and
/// fixtures (`shape = "fixture"`, `fixture_kind`, `fixture_params`), and a page record may carry
/// `page.<id>.owner`, which makes it that manufacturing's output page (its size in the version 34
/// `page.<id>.frame`). A build that stops at
/// version 34 would read a placement or fixture as an unknown primitive and could draw or output
/// it, and would list the output page as a drawing page, so it must refuse every file of this
/// build as too new. Migration from version 34 is none: a file without manufacturings opens
/// unchanged and is not rewritten. Damage (`crate::manufacturing_codec::validate`): a
/// `placement_link` that is not `{page string, six finite numbers within ±1e7}`, a fixture
/// without a kind or with `fixture_params` that is not a map of plain values, another leaf under
/// a manufacturing, a manufacturing without process or page, a `page.<id>.owner` of a live page
/// naming a live node that is not a manufacturing. An unknown process is not damage: it opens
/// disabled and saves back unchanged (ADR 0033 §4). Goldens: `laser_workbench_v35.curvyo` and
/// `manufacturing_unknown_process_v35.curvyo`.
///
/// Bumped to 36 in `machine-profile` (ADR 0039, `specs/0026-machine-profile`): a manufacturing
/// carries its machine profile copy as one atomic map register `mfg_profile` (the library id and
/// the copied values, never a connection; opaque here, typed by the process's extension) instead
/// of the placeholder `mfg_machine_name`. A build that stops at 35 would show the manufacturing
/// without its machine and Add manufacturing would write the old shape into the same document,
/// so it must refuse every file of this build as too new. Migration from 35: a manufacturing
/// without `mfg_profile` reads as the built-in Generic laser named by `mfg_machine_name`; nothing
/// is rewritten on open, and the first profile write stores `mfg_profile` and removes the old key
/// in the same step. Damage: `mfg_profile` present and not a map of plain values (a non-finite
/// number inside it reads as `null`, which the extension reads as that field's default). The
/// number is provisional by the rule above. Goldens: `machine_profile_v36.curvyo`,
/// `machine_profile_damaged_v36.curvyo`, `mfg_profile_not_a_map_v36.curvyo` and
/// `laser_workbench_v36.curvyo`; the version 35 goldens stay and open by the migration.
///
/// Bumped to 37 in `page-kinds` (ADR 0042, `specs/0098-page-kinds`): a page record carries
/// `page.<id>.kind` (`"drawing"` or `"sketch"`, written once at creation); a page with
/// `page.<id>.owner` naming a live manufacturing is an output page, listed and placed in the plane
/// like any other, and only a sketch page owns a sketch. A build that stops at 36 would hide output
/// pages again and let a drawing page own a sketch, recreating what the migration removed, so it
/// must refuse every file of this build as too new. Migration from 36
/// (`crate::page_kind_migration`, one commit `migrate_format` at open, no undo step): output pages
/// get a list place and a frame by the Add page rule; the sketch of a drawing page moves to a new
/// sketch page `<page id>-sketch` and leaves a linked clone with own placement. Damage: a `kind`
/// that is not a text reads as a drawing page and is not refused; an unknown text is kept. The
/// number is provisional by the rule above. Goldens: `page_kinds_v37.curvyo`,
/// `page_kind_damaged_v37.curvyo`, `page_kind_unknown_v37.curvyo`,
/// `page_kinds_migration_v36.curvyo`.
///
/// Bumped to 38 in `bring-content-in` (ADR 0045, `specs/0100-bring-content-in`): no register is
/// added; the set of legal `clone_link` origins widens to primitives, texts, images and groups
/// (a clone of a group draws the group's drawn subtree), and a placement may carry any matrix of
/// its laser page. A build that stops at 37 would read every clone of a group, text, image or
/// primitive as a wrong-kind orphan, and its repair after Open would delete them and save the
/// deletion: silent loss. So it must refuse every file of this build as too new. Migration from 37
/// is none: a format-37 file cannot hold a live clone of those kinds (its repair deleted any), so
/// it opens unchanged and is not rewritten. Damage: none new; a clone inside what it draws is an
/// orphan (ADR 0045 §4), repaired as before. The number is provisional by the rule above.
/// Golden: `bring_content_in_v38.curvyo`; the version 37 goldens stay.
///
/// Bumped to 39 in `laser-operations` (ADR 0046, `specs/0101-laser-operations`): a manufacturing
/// may hold operation nodes (`shape = "operation"`, `name`, `op_kind`, `op_show`,
/// `op_output`, `op_margin`, and one `member.<node id>` = `true` per assigned item), in list
/// order, and carry `x.<process id>.cuts_last` (`crate::operation_codec`). A build that stops at
/// 38 refuses any other child of a manufacturing as damage, so it must refuse every file of this
/// build as too new. Migration from 38 is none: a format-38 file has no operation node, opens
/// unchanged and is not rewritten. Damage: none new; an operation whose registers do not read
/// (or whose kind is not known) is kept and saved back unchanged, and reads with no kind. The
/// number is provisional by the rule above. Goldens: `operations_v39.curvyo`,
/// `operations_unknown_kind_v39.curvyo`, `operations_unreadable_v39.curvyo`; the version 38
/// golden stays.
///
/// Bumped to 40 in `laser-fixtures` (ADR 0049, `specs/0102-laser-fixtures`): a fixture node
/// (`shape = "fixture"`) may carry `active` (absent reads as on) and `fixture_matrix` (six
/// numbers, absent is identity), its kind is `curvyo.laser/<kind>` for the grid, holder, clamp
/// and stop, and it may hold slot child nodes (`shape = "fixture_slot"`, `name`, the atomic map
/// `slot_params`). A build that stops at 39 refuses any child of a fixture as damage, so it must
/// refuse every file of this build as too new. Migration from 39 is none: a format-39 file has
/// no slot, opens unchanged and is not rewritten. Damage is read per field
/// (`crate::manufacturing_fixtures`): a fixture value, a matrix or an `active` that does not read
/// is counted and reads as its default, a slot whose `slot_params` is not a map is skipped, a
/// slot outside a fixture is kept and never read; only a fixture without a kind or with a child
/// that is not a slot is refused. The number is provisional by the rule above. Goldens:
/// `fixtures_v40.curvyo`, `fixtures_unknown_kind_v40.curvyo`, `fixtures_damaged_v40.curvyo`,
/// `fixtures_slot_orphan_v40.curvyo`, `fixtures_grid_v39.curvyo`; the older goldens stay.
///
/// Bumped to 41 in `fixture-jobs` (ADR 0052, `specs/0103-fixture-jobs`): a fixture may hold job
/// child nodes (`shape = "fixture_job"`, `name`, `op_kind`, `op_show` absent on, `op_output`
/// absent off, and one `target.<node id>[.lines|.tapes|.head|.thread]` = `true` per ticked part)
/// beside its slots. A build that stops at 40 refuses a fixture with a non-slot child as damage,
/// so it must refuse every file of this build as too new. Migration from 40 is none: a format-40
/// file has no job, opens unchanged and is not rewritten. Damage is read per field
/// (`crate::fixture_job_codec`): a switch that is not a boolean, a kind that is missing or not a
/// text, a tick whose value is not `true`; a key of a node that is not a part of the job's fixture
/// is kept and is not damage; a job of an unknown kind is kept and saved back unchanged. The
/// number is provisional by the rule above. Goldens: `fixture_jobs_v41.curvyo`,
/// `fixture_jobs_unknown_kind_v41.curvyo`, `fixture_jobs_damaged_v41.curvyo`; the older goldens
/// stay.
///
/// Bumped to 42 in `pen-modes-variable-stroke` (ADR 0053, `specs/0110-pen-modes-variable-stroke`):
/// editable curve kinds (Spiro, B-spline with their control points and the Spiro placement
/// register) and independent width points cannot be interpreted safely by older readers. Absent
/// keys keep ordinary Bézier and uniform-stroke behaviour; malformed extended data is refused.
/// Migration from 41 is none. The number is provisional by the rule above. Golden:
/// `pen_modes_v42.curvyo`; the older goldens stay.
///
/// Bumped to 43 in `material-test-library` (ADR 0054, `specs/0028-material-test-library`): an
/// operation may carry `op_settings` (one atomic map of plain values: `main`, `outline`, the
/// material link `material` with its `modified`, the typed `overrides`) and `op_grid` (a test
/// grid row, whose one membership is its top group); a group may carry the test grid keys
/// `x.curvyo.laser.test_grid` and `test_grid_worked`, its items `grid_cell` and `grid_label`; the
/// root may carry `x.curvyo.laser.material_edit`; the label `refresh_material_numbers` is
/// reserved (no step, no history row). A build that stops at 42 would read a grid row as an
/// ordinary operation over the whole group: its copy would share the original's row and its
/// delete would leave a row without a grid, silently, so it must refuse every file of this build
/// as too new. Migration from 42 is none: a format-42 file opens with no settings on every
/// operation and is not rewritten. Damage: an `op_settings` that does not read is Damaged settings
/// (the extension decides; the kind stays readable); an `op_grid` that is not a boolean makes the
/// operation Unknown. The number is provisional by the rule above. Goldens:
/// `laser_settings_v43.curvyo`, `laser_settings_damaged_v43.curvyo`,
/// `test_grid_hand_edited_v43.curvyo`, `test_grid_orphan_row_v43.curvyo`; the older goldens stay.
///
/// Bumped to 44 in `laser-operation-tools` (ADR 0056, `specs/0111-laser-operation-tools`): the
/// `main` set (and `overrides.main`) of an Engrave kind's `op_settings` may hold `interval_mm`; an
/// operation may carry `op_options` (one atomic map of plain values: `crosshatch`,
/// `bidirectional`, `scan_angle_deg`, `overscan_pct`, `z_offset_mm`); the profile copy
/// `mfg_profile` may hold `motorised_z`. A build that stops at 43 would read every Engrave with an
/// interval as Damaged settings and would drop `motorised_z` silently on its next field edit of the
/// copy, so it must refuse every file of this build as too new. Clone-member keys
/// (`member.<clone>/<member>`) are legacy from here on: kept and saved back, read by no rule.
/// Migration from 43 is none: a format-43 file opens with the defaults of the new values and is
/// not rewritten. Damage: an `op_options` that does not read is Damaged settings (the extension
/// decides; the kind stays readable). The number is provisional by the rule above. Goldens:
/// `operation_tools_v44.curvyo`, `operation_options_damaged_v44.curvyo`; the older goldens stay.
///
/// Bumped to 45 in `image-trace-mask` (ADR 0061, `specs/0114-image-trace-mask`): an image node may
/// carry its trace mask as keyed registers `trace_part.<part>` (`{order, outside}`),
/// `trace_settings.<part>` (one atomic map of plain values) and `trace_region.<part>.<region>`
/// (`{order, kind, subtract, points}` in the image's unit square). A build that stops at 44 would
/// open the file, show no mask and trace the whole picture where the maker expects the stored
/// mask, and its tiling update would not drop a mask on an asset change, so it must refuse every
/// file of this build as too new. Migration from 44 is none: masks were never stored, and mask
/// paths drawn under `0094` stay ordinary paths; a format-44 file opens unchanged and is not
/// rewritten. Damage is read as ADR 0061 §7 says and never repaired on read. The number was the
/// next free one at merge. Goldens: `trace_mask_v45.curvyo`, `trace_mask_damaged_v45.curvyo`;
/// the older goldens stay.
///
/// Bumped to 46 in `test-grid-and-materials` (ADR 0057, `specs/0112-test-grid-and-materials`):
/// a test grid is a group whose `x.curvyo.laser.test_grid` is the recipe v2 (one atomic map with
/// the test `id`, `kind`, the `columns` and `rows` axes, `fixed`, `cell_mm`, `crosshatch` and the
/// picked `material`), with `x.curvyo.laser.test_grid_lasered` (Unix milliseconds) once Lasered;
/// its cells are tagged `grid_cell` = `{ column, row }` and its labels `grid_label` = `id`,
/// `column_title`, `row_title`, `column_value` or `row_value`. It resolves into ordinary derived
/// operations that carry `op_grid_part` (one atomic map `{ grid, part: "labels" }` or `{ grid,
/// part: "cell", column, row }`); they are rewritten in place by every grid change and deleted
/// with the grid (delete, cut, ungroup, a move off the laser page; an orphan of a merge at the
/// next local step). A build that stops at 45 would read the new recipe as damaged and keep
/// rows that its copy shares and its delete leaves behind, silently, so it must refuse every
/// file of this build as too new. Migration from 43, 44 and 45 is the session's (ADR 0057 §7,
/// plan Part F); until then such a file opens unchanged, its `op_grid` row inert and written back.
/// Damage (§6): a recipe that does not read makes the group plain (the extension decides), a
/// Lasered register that is not a non-negative integer reads Not lasered, an `op_grid_part` that
/// does not read leaves the row ordinary; nothing is repaired on read. The number is provisional
/// by the rule above. Goldens: `test_grid_v46.curvyo`, `test_grid_damaged_v46.curvyo`,
/// `test_grid_orphan_rows_v46.curvyo`; the older goldens stay.
///
/// Bumped to 47 in `fixture-sets-and-pins` (ADR 0059, `specs/0113-fixture-sets-and-pins`): a
/// manufacturing may hold set nodes (`shape = "fixture_set"`, `name`, `set_on_page`) whose
/// children are fixtures; a fixture directly under the manufacturing is a job fixture and may
/// carry `fixture_locked`; a new kind `curvyo.laser/pin`; pin and screw grid parameters
/// (`columns`, `rows`, `column_spacing_mm`, `row_spacing_mm`, `mark_mm`); a fixture job may carry
/// `job_role` (`centre_mark`, `diameters`) and the part suffixes `.circle` and `.marks`. A build
/// that stops at 46 refuses a `fixture_set` child of a manufacturing as damage, so this build's
/// files must be refused by version (ADR 0033 §7). No migration (2026-11-15, lead, Proposal): a
/// file of 46 or lower opens unchanged, its fixtures are job fixtures, nothing is written.
/// Damage (ADR 0059 §4, §5): a set without a name reads "Set n", a `set_on_page` or
/// `fixture_locked` that is not a boolean reads its default, a `job_role` that does not read, is
/// on another kind or repeats on one fixture reads as an ordinary job; each counts one value and
/// nothing is repaired on read. Amended 2026-11-16 (sets in the profile, ADR 0059 §10, ADR 0060
/// §5a): the set nodes are the page's mirror of its profile record and carry the record's ids
/// (`set_uid` on a set, `fixture_uid` on a set fixture, `job_uid` on its jobs) and `set_locked`;
/// a set fixture's lock is `fixture_locked`; the manufacturing carries
/// `mfg_fixture_sets_modified` (the record's `modified` the mirror was last aligned to); the root
/// may carry the switch `snapping_centres`; the label `align_fixture_sets` is reserved (no step,
/// no history row). A uid that is no text reads absent (the next set edit fills it) and a
/// `set_locked` that is no boolean reads unlocked; each counts one value. Goldens: `fixture_sets_v47.curvyo`,
/// `fixture_sets_damaged_v47.curvyo`, `fixture_sets_align_v47.curvyo`,
/// `snapping_centres_type_v47.curvyo`, `fixtures_pre_sets_v46.curvyo`; the older goldens stay.
///
/// Bumped to 48 in `laser-job-preview-and-output` (ADR 0065, `specs/0029`): the manufacturing's
/// profile copy `mfg_profile` may hold `air_assist` (a boolean) and `output_format` (`"grbl"`);
/// a laser page may carry the process registers `x.curvyo.laser.fixture_material` (a material
/// id) and `.start` (`"machine_origin"`, `"head_position"` or `"home_then_origin"`; one register
/// since 2026-11-17, the first build's `.start_from` and `.home_first` are never read or written);
/// the labels `export_job`, `set_fixture_material`, `set_start_machine_origin`,
/// `set_start_head_position`, `set_start_home_then_origin` and `air_assist_on` / `_off` are
/// steps, `mark_test_grid_lasered` is reserved (no step, no history row, so an undo of the export
/// keeps Lasered). A build that stops at 47 rewrites the whole `mfg_profile` map from its own
/// type on a field edit and would drop `air_assist` and `output_format` silently, so this build's
/// files must be refused by version (ADR 0033 §7); the registers ride on the bump. No migration:
/// a file of 47 or lower opens unchanged with the defaults (no pick, Machine origin, air assist
/// off, GRBL), nothing is written. Damage (ADR 0065 §1, §3): a register or `air_assist` of the
/// wrong type or spelling reads its default and counts one value (a damaged `start` also blocks
/// the export, never guessed into a file); an
/// `output_format` this build does not know reads damaged and blocks the export (never written
/// as GRBL); nothing is repaired on read. The laser extension reads every value. Goldens:
/// `laser_output_v48.curvyo`, `laser_output_damaged_v48.curvyo`; the older goldens stay.
///
/// Bumped to 49 in `gradient-fill` (ADR 0066, `specs/0124`): an object's style may carry
/// `fill_paint = "gradient"` and two registers, each one compact text (ADR 0066 §1, amended
/// 2026-11-18): `fill_gradient` (`linear X1 Y1 X2 Y2` or `radial CX CY RX RY [RATIO]`, points as
/// box fractions) and `fill_gradient_stops` (`#RRGGBB[AA] OFFSET;...`); the labels
/// `fill_gradient`, `gradient_linear`, `gradient_radial`, `edit_gradient`, `add_gradient_stop`,
/// `remove_gradient_stop` and `reverse_gradient` are steps. A build that stops at 48 refuses
/// `"gradient"` as a damaged `fill_paint`, the wrong message, and a build that knew the paint but
/// not the registers would draw no fill or the Colour where the gradient is, so this build's files
/// are refused by version (ADR 0033 §7). No migration: a file of 48 or lower has no gradient and
/// opens unchanged; the legacy `fill_kind` / `fill_stops` of format 7 stay ignored. Damage (ADR
/// 0066 §7): an unknown paint or kind, a register that is not a string, a wrong number of fields
/// or a wrong separator, a fraction that is not a finite number of the grammar, a ratio on a
/// linear or not above 0, a colour that is not `#` plus 6 or 8 hex digits, fewer than 2 stops,
/// offsets out of [0, 1], unordered or not from 0 to 1, and a gradient paint without both
/// registers refuse the file. Goldens: `gradient_fill_v49.curvyo` and
/// `gradient_fill_damaged_{colour,fraction,one_stop,unordered,paint_without_gradient}_v49.curvyo`;
/// the older goldens stay.
///
/// Bumped to 50 in `raster-engraving` (ADR 0069 §7, §8, §10, §13, `specs/0123`): the operation
/// `kind` register may hold `"image"` (the Image kind, opaque here as every kind, ADR 0046 §1); an
/// Image operation's `op_options` may hold the Image settings `dither` (`"threshold"`,
/// `"ordered"`, `"floyd_steinberg"`, `"jarvis"`, `"atkinson"`, `"stucki"` or `"grey"`),
/// `threshold_pct`, `brightness_pct`, `contrast_pct`, `gamma`, `invert` and `lowest_power_pct`, and
/// a test grid recipe `x.curvyo.laser.test_grid` may be of kind `"image"` with `dither` (a binary
/// mode); an Image grid's cells have a Gradient fill of format 49. No new label: the Image
/// settings commit as `edit_operation_settings` ("Edit settings") and Dither as `edit_test_grid`
/// ("Edit test grid"); no label is reserved. A build that stops at 49 reads `"image"` as an
/// Unknown kind, the new `op_options` keys as damage (ADR 0056 §6) and the new recipe as damaged,
/// so this build's files are refused by version (ADR 0033 §7). No migration: a file of 49 or
/// lower opens unchanged and an operation without the keys reads their defaults. The document
/// stores no pixel, dot or row (criterion 38). Damage (ADR 0069 §7, §13): an unknown `dither`, a
/// value out of range or with too many decimals, an Image key on any kind but Image, or
/// `crosshatch` on an Image is Damaged settings of that operation (it does not run; the file
/// opens); a `dither` on a grid of another kind, Grey, or a `fill` key is a damaged recipe (ADR
/// 0057 §6). The laser extension reads every value. Goldens: `raster_engraving_v50.curvyo` and
/// `raster_engraving_damaged_v50.curvyo`; the older goldens stay.
///
/// Bumped to 51 in `image-optimisation` slice 1 (ADR 0072 §1, §2, §5, §6, §8, `specs/0135`
/// criteria 8, 9, 42, 43, 51): an image may hold the registers `image_preset` (text, absent =
/// `"expert"`), `image_strength` (an integer 0 to 100, absent = 50) and `image_adjustments` (one
/// plain list of `{kind, enabled, fields}` with integer fields, the unit in the key; the slice-1
/// kinds `denoise`, `levels`, `local_contrast`, `contours`, `sharpen`); a linked clone of an image
/// may hold the `own_adjustments` marker and then the same registers (ADR 0016 pattern, the
/// nearest own on the chain is the source); changed in place (ADR 0072 amendment A1, A8): an image
/// and an own set may hold `image_dither` (one plain map `{mode, threshold_pct, screen_size_um,
/// screen_angle_deg}`, absent = Floyd-Steinberg; Lowest power stays the operation's, Frage 284 B), the dither
/// the picture burns with, and a format-50 file's pictures open at Floyd-Steinberg; an Image
/// operation's `dither` may be `"halftone"` or `"line_screen"` with `screen_size_mm` and
/// `screen_angle_deg`, read for its gradient-filled leaves only; an Image test grid recipe may
/// hold `picture` and a `strength` axis. Labels `edit_adjustments` ("Edit adjustments"),
/// `restore_original` ("Restore original") and `follow_original` ("Follow original"); no label
/// is reserved. A build that stops at 50 ignores the registers and would burn other dots, and
/// reads the new dither values as damage, so this build's files are refused by version (ADR 0033
/// §7). No migration: a file of 50 or lower has no adjustments and opens unchanged. The document
/// stores no adjusted pixel (criterion 18). Damage (ADR 0072 §5) never refuses the file: a
/// register that does not read reads as its default, an entry not of the step shape is kept and
/// not run, a non-bool marker reads false; the laser extension counts them for the check line.
/// Goldens: `image_adjust_v51.curvyo` and `image_adjust_damaged_v51.curvyo`; the older goldens
/// stay.
///
/// Version 52 (`specs/0135-image-optimisation` slice 2, ADR 0072 §3, §6, §8): the stack may hold
/// the kinds `crop` (`left_pct`, `top_pct`, `width_pct`, `height_pct`; the drawn frame is the
/// placement's visible part, `image_placement` keeps the whole picture), `resize` (`width_px`),
/// `channel_mix` (`red_pct`, `green_pct`, `blue_pct`) and `curves` (`in_0`..`in_7`, `out_0`..`out_7`, the pairs present are the points); the cells of
/// an Image test grid with the Picture source are image nodes tagged `grid_cell`. A build that
/// stops at 51 would engrave the uncropped picture, so this build's files are refused there by
/// version; a file of 51 opens unchanged. Golden: `image_adjust_crop_v52.curvyo`.
///
/// Version 53 (`specs/0135-image-optimisation` slice 3, ADR 0077 §1, §2, §9): the stack may hold
/// the AI kinds `upscale` (`factor`), `ai_denoise` (`strength_pct`) and `remove_background`
/// (`softness_pct`, `threshold_pct`), each with an optional `result` record naming a derived
/// asset, and the container holds those as `assets/derived/<sha256>.png`. A build that stops at
/// 52 would burn the picture without its stored result, so this build's files are refused there
/// by version; a file of 52 opens unchanged. Golden: `image_adjust_ai_v53.curvyo`.
///
/// Version 54 (`specs/0141-parametric-generators`, ADR 0080 §4): a leaf may have `shape =
/// "generated"` with `generated` (one atomic map `{kind, seed, params, parts}`, the stored
/// outline in frame-local mm) and `generated_frame` (`[a, b, c, d, e, f]`, rotation, mirror and
/// translation only). A build that stops at 53 reads the unknown `shape` tag as a primitive
/// without its registers and would refuse or drop the object, so this build's files are refused
/// there by version (ADR 0033 §7); a file of 53 opens unchanged. An unknown or disabled kind is
/// kept and drawn from its stored outline and a save keeps its record (ADR 0080 §5); a new
/// generator kind needs no bump. Labels `add_generated`, `change_parameter` and
/// `convert_generated`; steps may carry the fields `gk` (kind) and `gp` (parameter). Damage
/// refuses the file: `generated` not a map, a `kind` that is not `<id>/<kind>`, a seed outside
/// `u32`, no part or an empty part, a subpath that is not `2 + 6n` finite numbers within ±1e7, a
/// frame that is not six finite numbers or holds scale or shear beyond 1e-9. Goldens:
/// `generated_v54.curvyo`, `generated_unknown_kind_v54.curvyo`, `generated_damaged_v54.curvyo`.
///
/// Version 55 (`specs/0121-print-and-cut` slice 1, ADR 0081 §1, §4, §9): a leaf may be a
/// registration mark (`shape = "registration_mark"`, `mark_number`, `mark_centre`,
/// `mark_diameter`, `mark_line`) inside a group tagged `registration_marks`, and a placement may
/// hold the register `x.curvyo.laser.print_and_cut` (measured marks in µm, tolerance, Reset
/// matrix). A build that stops at 54 would read the marks as nothing it knows and could cut the
/// sheet in the wrong place, so this build's files are refused there by version; a file of 54
/// opens unchanged. Goldens: `print_and_cut_v55.curvyo`, `print_and_cut_damaged_v55.curvyo`.
///
/// Version 56 (`specs/0145-shape-spiro-handles` slice 1, `adrs.md` decision 4): a polygon or star
/// may carry the register `rounded` (a finite number, absent = 0, limited to -10..=10 where it
/// is evaluated). A build that stops at 55 would draw the shape's corners straight, so this
/// build's files are refused there by version; a file of 55 opens unchanged. Golden:
/// `rounded_v56.curvyo`.
///
/// Version 57 (`specs/0145-shape-spiro-handles` slice 2, `adrs.md` decisions 4 and 9): an
/// ellipse may carry the register `rounded` too (same rules as on a polygon or star; ignored
/// while the ellipse has an arc). A build that stops at 56 would draw the ellipse without its
/// loops, so this build's files are refused there by version; a file of 56 opens unchanged.
/// Golden: `rounded_ellipse_v57.curvyo`.
///
/// Version 58 (`specs/0137-document-library`, ADR 0084 §7, §8): a node may carry the meta key
/// `came_from` and a page the register `page.<id>.came_from`, one plain map each (`document`,
/// `page`, `page_id`, optional `item`, `at` in Unix milliseconds). The record never changes
/// drawing or output: a wrong type or a missing field reads as absent and is kept, the file opens.
/// A build that stops at 57 would drop the record silently on Object to path and never show it,
/// so this build's files are refused there by version; a file of 57 opens unchanged. Goldens:
/// `came_from_v58.curvyo`, `came_from_damaged_v58.curvyo`.
///
/// Version 59 (`specs/0150-operation-always-has-numbers`, `adrs.md` decisions 1, 3, 4, 7): the
/// machine profile copy `mfg_profile` may hold `default_power_pct` (1 to 100, one decimal) and
/// `default_speed_mm_min` (a whole 1 to 100,000), each read on its own, absent or unreadable as
/// 10 % and 1,000 mm/min, the speed limited to the copy's max feed; an operation without
/// `op_settings` no longer means No settings: the laser extension reads it as own numbers from
/// its copy's pair, and writes them into the register when the operation is created, when a step
/// changes the copy's pair (before the change, in that step) and in the reserved commit of a save
/// that changes the project. Labels `edit_default_power` ("Edit default power") and
/// `edit_default_speed` ("Edit default speed"). A build that stops at 58 would drop the two copy
/// keys on its next Machine block edit and block such an operation as "No settings", so this
/// build's files are refused there by version (ADR 0033 §7); a file of 58 opens unchanged and is
/// not rewritten on read. Golden: `operation_defaults_v59.curvyo`.
///
/// Version 60 (`specs/0145-shape-spiro-handles` slice 3, `adrs.md` decisions 4 and 8): a polygon
/// or star may carry the registers `randomized` (a finite number, absent = 0, limited to -10..=10
/// where it is evaluated) and `seed` (a whole number of 32 bits, absent = 0). A build that stops
/// at 59 would draw the vertices unmoved, so this build's files are refused there by version; a
/// file of 59 opens unchanged. Golden: `randomized_v60.curvyo`.
///
/// Version 61 (`specs/0118-machine-control` slice 2c, `adrs.md` O1 to O3): a laser page's
/// manufacturing may hold the register `x.curvyo.laser.user_origin_um` (`[x, y]`, two integers in
/// µm of the job file's coordinates, each within ±10,000,000) and `x.curvyo.laser.start` gains the
/// spelling `"user_origin"`. A build that stops at 60 would read that Start as damaged, so this
/// build's files are refused there by version; a file of 60 opens unchanged. Goldens:
/// `user_origin_v61.curvyo`, `user_origin_damaged_v61.curvyo`.
///
/// Version 62 (`specs/0099-sketch-pages` revision 2026-11-19, `adrs.md` A2, A3, A5): a sketch may
/// carry the register `exposed_params` (one list value of `{ key, name?, min?, max? }`, key
/// `p:<ParameterId>` or `d:<ConstraintId>`), a clone the registers `instance_params` and
/// `instance_fallback` (one map value each, key to `{"mm": n}` or `{"deg": n}`). Only a wrong
/// register shape refuses the file; an entry or value that does not decode is ignored and kept,
/// a stored value outside its limits is clamped at read and not rewritten. A build that stops at
/// 61 would draw every clone with the sketch's own values and cut the wrong part, so this build's
/// files are refused there by version; a file of 61 opens unchanged. Goldens:
/// `sketch_instances_v62.curvyo`, `sketch_instances_damaged_v62.curvyo`.
///
/// Version 63 (`specs/0118-machine-control` slice 2d, `adrs.md` P7, P8): a manufacturing's
/// profile copy `mfg_profile` may hold `focus_pulse` (a boolean), `focus_pulse_power_permille`
/// (1 to 50) and `focus_pulse_length_ms` (50 to 2,000); the labels `focus_pulse_on` / `_off`,
/// `edit_pulse_power` and `edit_pulse_length` are steps. A build that stops at 62 rewrites the
/// whole `mfg_profile` map from its type on a field edit and would drop the three keys silently,
/// so this build's files are refused there by version (ADR 0065 §4). No migration: a file of 62
/// or lower opens with the defaults (off, 1 %, 300 ms), unchanged. Damage: a key of the wrong
/// type or out of range reads its default (a damaged switch reads off) and counts one value;
/// nothing is repaired on read. Goldens: `focus_pulse_v63.curvyo`,
/// `focus_pulse_damaged_v63.curvyo`.
/// Version 64 (`specs/0099-sketch-pages` slice 2, `adrs.md` A13): an `exposed_params` entry may
/// carry `"link": "<group>"` (entries with the same string form a link group; a group of one, a
/// member of another kind and an invalid link read as no group and are kept), a clone the
/// register `instance_unlinked` (one list value of exposed keys it changed alone). A container
/// there refuses the file. Links change no geometry at read, but a build that stops at 63 would
/// not couple the linked values on edit and its limit edit would strip the `link` fields, so this
/// build's files are refused there by version; a file of 63 or lower opens unchanged. Goldens:
/// `sketch_instance_links_v64.curvyo`, `sketch_instance_links_damaged_v64.curvyo`.
/// Version 65 (`specs/0147-rounded-box` slice 3, ADR 0082 §10, customer Frage 370 A): the record of a
/// generated object may carry `part_keys` (one key per part, written whole with `parts`), and an
/// operation may hold `member.<node>#<part key>`, the own operations of a detached part (Detach
/// part). A build that stops at 64 would read such a key as a dangling or unreadable one and cut
/// the part with the box's operation, which is wrong laser output, so this build's files are
/// refused there by version; a file of 64 or lower opens unchanged and gets its keys at its next
/// parameter edit. Damage: `part_keys` that is not a list of valid distinct key strings as long as
/// `parts` refuses the file; a part key of an operation that does not read is kept and never
/// touched. Goldens: `generated_part_ops_v65.curvyo`, `generated_part_keys_damaged_v65.curvyo`.
/// Version 67 (`specs/0146-material-test-picks`, ADR 0093 decision 9): the atomic `material` map of
/// an operation's `op_settings` may carry `"pick"`: `"second_best"`, `"fastest"` or `"deepest"`
/// (absent means Best, `"best"` is never written and reads as a bad link). A build that stops at
/// 65 reads such a link as Damaged settings, so this build's files are refused there by version.
/// No migration: a file of 65 or lower opens unchanged and every link follows Best. Goldens:
/// `material_pick_v66.curvyo`, `material_pick_damaged_v66.curvyo`.
/// Version 67 (`specs/0139-laser-cut-path-options` slice 1, `adrs.md` L2 to L6): the `op_options`
/// of a Cut or Cut-out operation may hold `tab_count`, `tab_width_mm` and `tab_skip_inner`, and an
/// operation node may hold `tabs.<key>` (the own tabs of the item whose membership key is
/// `member.<key>`; a plain map from an outline reference to a list of `{ at, width_mm? }`). A build
/// that stops at 66 reads the new `op_options` keys as damaged settings and ignores the tabs, which
/// would cut a held part loose, so this build's files are refused there by version; a file of 66 or
/// lower opens unchanged with no tabs. The records are opaque here: the laser extension reads them,
/// a damaged one makes its item run nothing and is kept unchanged. They go with the membership
/// key (unassign, delete, the dead-key sweep). Goldens: `laser_path_options_v67.curvyo`,
/// `laser_path_options_damaged_v67.curvyo`.
/// Version 68 (`specs/0139-laser-cut-path-options` slice 2, `adrs.md` L2 to L6): the `op_options`
/// of a Cut or Cut-out operation may hold `lead_in`, `lead_in_mm`, `lead_out`, `lead_out_mm` and
/// `overcut_mm`, and an operation node may hold `path_own.<key>` (an item's own lead-in, lead-out
/// and overcut, a plain map of those five values) and `entry.<key>` (an item's start points and
/// directions, a plain map from an outline reference to `{ start }` or `{ direction }`). A build
/// that stops at 67 reads the new `op_options` keys as damaged settings and ignores the records,
/// which would enter a held part at a wrong place, so this build's files are refused there by
/// version; a file of 67 or lower opens unchanged with every path Automatic and no lead. The
/// records are opaque here, kept unchanged when damaged, and go with the membership key as the
/// tabs do. Goldens: `laser_path_options_v68.curvyo`, `laser_path_options_damaged_v68.curvyo`.
/// Version 69 (`specs/0159-cutting-plotter` slice 1, ADR 0097 §6, `adrs.md` PL3, Frage 431 A): an
/// output page of process `curvyo.cutter` (a plotter page) holds a cutter profile copy in
/// `mfg_profile`, its mats as job fixtures of kind `curvyo.cutter/mat-12x12` and
/// `curvyo.cutter/mat-12x24` with the params `grip` and `grid` (one `active`), and operations of
/// `op_kind` `cut` or `draw` with `op_settings` `{ pressure_pct, speed_mm_s, passes }`, all opaque
/// here and read by the cutter extension. A build that stops at 68 would open such a page as a
/// laser page and read its copy and operations as laser values, so this build's files are refused
/// there by version; a file of 68 or lower opens unchanged. Damage (read by the extension): an
/// unknown mat kind reads as the 12 x 12 in `StandardGrip` mat, a value out of its range as its
/// default, with a notice; nothing is rewritten on read. Goldens (in `curvyo-editor-wasm`):
/// `plotter_page_v69.curvyo`, `plotter_page_damaged_v69.curvyo`.
/// Version 70 (`specs/0160-embroidery` slice 1, ADR 0097 §6, `adrs.md` EM5, Frage 431 A): an
/// output page of process `curvyo.embroidery` (an embroidery page) holds an embroidery profile
/// copy in `mfg_profile`, its hoops as job fixtures of kind `curvyo.embroidery/hoop-150x250` and
/// `curvyo.embroidery/hoop-100x100` (one `active`), and operations of `op_kind` `running`,
/// `tatami` or `satin` with `op_settings` `{ stitch_length_mm, repeats }`, `{ angle_deg,
/// row_spacing_mm, max_stitch_mm }` or `{ zigzag_spacing_mm }` and the thread `thread_colour`
/// ("#RRGGBB"), all opaque here and read by the embroidery extension. A build that stops at 69
/// would open such a page without its hoop, stitches or numbers, so this build's files are refused
/// there by version; a file of 69 or lower opens unchanged. Damage (read by the extension): an
/// unknown hoop kind reads as the profile's first hoop, a value out of its range as its default,
/// with a notice; nothing is rewritten on read. Goldens (in `curvyo-editor-wasm`):
/// `embroidery_page_v70.curvyo`, `embroidery_page_damaged_v70.curvyo`.
/// Version 71 (`specs/0161-cnc-output` slice 1, ADR 0097 §6, `adrs.md` CN2, Frage 431 A): an
/// output page of process `curvyo.cnc` (a CNC page) holds a CNC profile copy in `mfg_profile`, its
/// stock as a job fixture of kind `curvyo.cnc/stock` (`width_mm`, `height_mm`, `thickness_mm`,
/// `material`), its tool list as the atomic register `x.curvyo.cnc.tools`, and operations of
/// `op_kind` `profile` with `op_options` `{ side, depth_mm, tool, direction }` and the typed feeds
/// in `op_settings`, all opaque here and read by the CNC extension. A build that stops at 70 would
/// open such a page without its stock, tools or numbers, so this build's files are refused there
/// by version; a file of 70 or lower opens unchanged. Damage (read by the extension): a tool
/// diameter of 0 or a depth beyond the stock reads as the default, with a notice; nothing is
/// rewritten on read. Goldens (in `curvyo-editor-wasm`): `cnc_page_v71.curvyo`,
/// `cnc_page_damaged_v71.curvyo`.
pub const CURRENT_FORMAT_VERSION: u32 = 71;

pub(crate) const ROOT_MAP: &str = "root";
pub(crate) const KEY_FORMAT_VERSION: &str = "format_version";
pub(crate) const KEY_WIDTH_MM: &str = "size_width_mm";
pub(crate) const KEY_HEIGHT_MM: &str = "size_height_mm";
/// The root register holding the display unit's symbol. Added without a
/// `format_version` bump: an older build ignores the key, shows mm, and keeps
/// the key when it saves (`specs/0015-document-size-and-rulers/adrs.md`,
/// decision 1).
pub(crate) const KEY_DISPLAY_UNIT: &str = "display_unit";

/// The top-level Loro tree container holding every object — path or
/// primitive alike (ADR 0002 §5: sibling order among tree nodes is
/// z-order; `specs/0003-primitive-shapes/adrs.md`: "primitives and paths share
/// one z-order, so they share one tree. They are not two lists.").
///
/// The Rust constant is named `OBJECTS_TREE` as of `primitive-shapes`,
/// but the on-disk/in-CRDT container key stays the literal `"paths"` it
/// has always been: renaming the stored key would be a container
/// migration for no benefit, since it is an opaque name in a format this
/// crate already writes (`adrs.md`). `curvyo-document-core` is the only
/// module that names this key; everything else goes through
/// [`Document`]'s methods — see [`crate::paths`] and [`crate::shapes`].
pub(crate) const OBJECTS_TREE: &str = "paths";

/// An open Curvyo document.
///
/// Its Loro CRDT backing is an implementation detail and never appears in
/// this type's public API (ADR 0004 §3): callers only ever see
/// [`DocumentSize`] and this crate's own error types.
pub struct Document {
    /// Ephemeral validated sketch read results; placement and style reuse the solve.
    pub(crate) sketch_cache: std::cell::RefCell<
        std::collections::HashMap<crate::NodeId, (DocumentVersion, crate::SketchSnapshot)>,
    >,
    /// The solves of sketch instances, keyed by content (`specs/0099-sketch-pages` A4).
    pub(crate) instance_solves: crate::InstanceSolves,
    pub(crate) loro: LoroDoc,
    /// The commit path's session state (`crate::history`).
    pub(crate) commit_state: crate::history::CommitState,
    /// The container chains of the log, for the touched objects of a step.
    pub(crate) container_index: std::cell::RefCell<crate::history::ContainerIndex>,
    /// Every step of the log with the stacks of each peer, for the history list.
    pub(crate) step_index: std::cell::RefCell<crate::history::StepIndex>,
    /// The group new objects are created in (`crate::groups`); view state, never stored.
    pub(crate) draw_parent: std::cell::Cell<Option<loro::TreeID>>,
    /// Whether a new object drawn into a document with no layer makes the Initial Layer
    /// (`crate::groups`); view state of the caller, never stored.
    pub(crate) initial_layer: std::cell::Cell<bool>,
    /// What the clone repair of the last removing command did (`crate::clone_repair`); view
    /// state for its notice, never stored.
    pub(crate) clone_repair: std::cell::Cell<crate::clone_repair::CloneRepair>,
    /// How many texts kept their removed carrier's outline since the host last asked
    /// (`crate::text_carrier`); view state, not stored.
    pub(crate) kept_carriers: std::cell::Cell<usize>,
    /// How many text clones became texts because their source was removed (`0080` criterion
    /// 41), since the session last read it.
    pub(crate) text_clones_unlinked: std::cell::Cell<usize>,
    /// Whether a membership key may exist in the drawing: false only when no operation has written
    /// one, nor a file or a peer brought one in (`operation::sweep_dead_member_keys` reads it, so a
    /// document without keys pays nothing per commit).
    pub(crate) member_keys_may_exist: std::cell::Cell<bool>,
    /// What the held-back rule of the last transform did (`crate::clone_hold`); view state.
    pub(crate) clone_hold: std::cell::Cell<crate::clone_hold::CloneHold>,
    /// The next counter of the anchor ids the clone repair takes (`crate::clone_repair`).
    pub(crate) repair_ids: std::cell::Cell<u64>,
    /// Whether a writer refused a placed page's move off its laser page since the session last
    /// asked (`crate::placement_objects`); view state, never stored.
    pub(crate) placement_refusal: std::cell::Cell<bool>,
    /// The clone named by the last refusal of the limit check (`crate::clone_write`); view state
    /// for its notice, never stored.
    pub(crate) clone_refusal: std::cell::Cell<Option<crate::path_model::NodeId>>,
    /// The steps that touched each object, for the Object scope of the history.
    pub(crate) object_index: std::cell::RefCell<crate::history::ObjectIndex>,
    /// The step an object undo of one object would really write, until the log changes.
    pub(crate) writable_cache: std::cell::RefCell<Option<crate::history::WritableUndo>>,
    /// The picture bytes the image nodes name (ADR 0017 §2); not in the Loro log.
    pub(crate) assets: std::cell::RefCell<crate::assets::AssetStore>,
    /// The workspace and id of a workspace document (ADR 0037 decision 3); kept in the container,
    /// not in the Loro log.
    pub(crate) share: std::cell::RefCell<Option<crate::container_share::WorkspaceShareRecord>>,
    /// The file held the share members of a document shared on its own (ADR 0044 decision 3).
    pub(crate) legacy_share: std::cell::Cell<bool>,
    /// Whether a live collaboration link is attached; shared history is never wiped (ADR 0053).
    pub(crate) live_share: std::cell::Cell<bool>,
    /// A copy of the log that remote updates are tried on before the document takes them
    /// (`crate::history::updates`); kept between imports so a check costs the update, not a
    /// fork of the whole document. Derived state, never stored.
    pub(crate) import_trial: std::cell::RefCell<Option<LoroDoc>>,
}

impl Document {
    /// Creates a new, empty document at the default page size (A4
    /// portrait, 210 × 297 mm), bound to the given Loro peer id.
    ///
    /// The peer id is a parameter rather than something this crate mints
    /// itself: `LoroDoc::new()` would otherwise draw one from `getrandom`,
    /// and a `*-core` crate reaching an entropy source is exactly what
    /// `CLAUDE.md` §6 forbids ("no filesystem, network, clock, threads or
    /// UI. Callers pass everything in.") — the same reason
    /// `curvyo-library-core` takes a record's UUID as a parameter instead
    /// of generating it (ADR 0011 §6). `curvyo-app` mints a fresh id per
    /// open session and passes it in here
    /// (`specs/0001-project-file-foundation/adrs.md`, amended 2026-10-03).
    ///
    /// # Panics
    /// Does not panic in practice: it only inserts known-valid keys into a
    /// freshly created, attached root map, and sets the peer id before any
    /// operation has been recorded — both of which Loro's API cannot
    /// reject.
    #[must_use]
    pub fn new(peer_id: u64) -> Self {
        let document = Self::blank(peer_id, DocumentSize::default());
        document.commit_new_document();
        // A document without history has nothing to read.
        document.commit_state.set_warm();
        document
    }

    /// The setup of a new document of `size`, not committed yet: [`Document::new`] commits it at
    /// once, [`Document::from_drawing`] after writing its content into the same change.
    pub(crate) fn blank(peer_id: u64, size: DocumentSize) -> Self {
        let loro = LoroDoc::new();
        // invariant: setting the peer id on a freshly created doc with no
        // recorded operations yet cannot fail.
        #[allow(clippy::unwrap_used)]
        loro.set_peer_id(peer_id).unwrap();
        crate::text_chars::configure(&loro);
        let root = loro.get_map(ROOT_MAP);
        // invariant: inserting known-valid keys into a freshly created,
        // attached root map cannot fail.
        #[allow(clippy::unwrap_used)]
        {
            root.insert(KEY_FORMAT_VERSION, CURRENT_FORMAT_VERSION)
                .unwrap();
            root.insert(KEY_WIDTH_MM, size.width.as_mm()).unwrap();
            root.insert(KEY_HEIGHT_MM, size.height.as_mm()).unwrap();
        }
        // The first page starts on the version 33 registers, which every build reads as its
        // frame; its first edit writes the frame register (ADR 0031 decision 9a), so a new
        // document's setup change is the same as before format 34.
        crate::pages::initialize(&root);
        Self {
            loro,
            commit_state: crate::history::CommitState::default(),
            container_index: std::cell::RefCell::default(),
            step_index: std::cell::RefCell::default(),
            draw_parent: std::cell::Cell::default(),
            initial_layer: std::cell::Cell::default(),
            clone_repair: std::cell::Cell::default(),
            kept_carriers: std::cell::Cell::default(),
            text_clones_unlinked: std::cell::Cell::default(),
            member_keys_may_exist: std::cell::Cell::default(),
            clone_hold: std::cell::Cell::default(),
            repair_ids: std::cell::Cell::default(),
            placement_refusal: std::cell::Cell::default(),
            clone_refusal: std::cell::Cell::default(),
            object_index: std::cell::RefCell::default(),
            writable_cache: std::cell::RefCell::default(),
            sketch_cache: std::cell::RefCell::default(),
            instance_solves: crate::InstanceSolves::default(),
            assets: std::cell::RefCell::default(),
            share: std::cell::RefCell::default(),
            legacy_share: std::cell::Cell::default(),
            live_share: std::cell::Cell::default(),
            import_trial: std::cell::RefCell::default(),
        }
    }

    /// Rebuilds a document from a previously exported Loro snapshot — the
    /// container's `document.loro` member, the sole source of truth on a
    /// normal open (ADR 0004 §1).
    ///
    /// `peer_id` is a parameter for the same reason [`Document::new`]'s is:
    /// `LoroDoc::new()` would otherwise draw one from `getrandom` itself,
    /// which this `*-core` crate must not do (`CLAUDE.md` §6). Reopening a
    /// file is a fresh session like any other, so it gets a fresh id from
    /// the caller the same way a brand-new document does.
    ///
    /// # Errors
    /// Returns [`OpenError::Damaged`] if the bytes are not a valid Loro
    /// snapshot this build can import, if a background or snapping register is
    /// present but malformed, or if they import cleanly but the
    /// `paths` tree inside them does not match the shape
    /// [`crate::path_codec`]'s writers always produce — a damaged-but-
    /// still-unzippable file (project-file-foundation's acceptance
    /// criterion 7: "not a crash") rather than a panic the first time some
    /// other method reads that path.
    ///
    /// # Panics
    /// Does not panic in practice: setting the peer id on a freshly
    /// created doc, before `import` records anything, cannot fail.
    pub(crate) fn from_loro_snapshot(peer_id: u64, bytes: &[u8]) -> Result<Self, OpenError> {
        let loro = LoroDoc::new();
        // invariant: setting the peer id on a freshly created doc with no
        // recorded operations yet cannot fail.
        #[allow(clippy::unwrap_used)]
        loro.set_peer_id(peer_id).unwrap();
        crate::text_chars::configure(&loro);
        let status = loro.import(bytes).map_err(|_| OpenError::Damaged)?;
        // Changes that continue a history this replica lacks leave no document to open.
        if status.pending.is_some() {
            return Err(OpenError::Damaged);
        }
        if !crate::history::updates::format_is_readable(&loro)
            || crate::history::updates::writes_newer_format(&loro, &loro::VersionVector::default())
            || !crate::path_codec::validate_path_tree(&loro, OBJECTS_TREE)
            || !crate::document_background::validate(&loro)
            || !crate::snapping::validate(&loro)
            || !crate::page_grid::validate(&loro)
            || !crate::page_frame::validate(&loro)
            || !crate::manufacturing_codec::validate(&loro)
        {
            return Err(OpenError::Damaged);
        }
        let doc = Self {
            loro,
            commit_state: crate::history::CommitState::default(),
            container_index: std::cell::RefCell::default(),
            step_index: std::cell::RefCell::default(),
            draw_parent: std::cell::Cell::default(),
            initial_layer: std::cell::Cell::default(),
            clone_repair: std::cell::Cell::default(),
            kept_carriers: std::cell::Cell::default(),
            text_clones_unlinked: std::cell::Cell::default(),
            member_keys_may_exist: std::cell::Cell::default(),
            clone_hold: std::cell::Cell::default(),
            repair_ids: std::cell::Cell::default(),
            placement_refusal: std::cell::Cell::default(),
            clone_refusal: std::cell::Cell::default(),
            object_index: std::cell::RefCell::default(),
            writable_cache: std::cell::RefCell::default(),
            sketch_cache: std::cell::RefCell::default(),
            instance_solves: crate::InstanceSolves::default(),
            assets: std::cell::RefCell::default(),
            share: std::cell::RefCell::default(),
            legacy_share: std::cell::Cell::default(),
            live_share: std::cell::Cell::default(),
            import_trial: std::cell::RefCell::default(),
        };
        // One scan at open: a file with membership keys keeps the sweep on.
        doc.member_keys_may_exist
            .set(!doc.member_entries().is_empty());
        Ok(doc)
    }

    /// The document's size in millimetres.
    ///
    /// Both axes fall back to the A4 default if either stored value is
    /// missing, not a finite number, or outside
    /// [`MIN_DOCUMENT_MM`]`..=`[`MAX_DOCUMENT_MM`]
    /// (`specs/0015-document-size-and-rulers/` criterion 13). The read is
    /// defensive and writes nothing: a file written by a build that stored
    /// no size, or damaged by hand, opens at A4 instead of being refused,
    /// and stays as it is until the maker resizes.
    #[must_use]
    pub fn size(&self) -> DocumentSize {
        let pages = self.pages();
        pages
            .iter()
            .find(|p| p.id == crate::FIRST_PAGE_ID)
            .or_else(|| pages.first())
            .map_or_else(DocumentSize::default, |p| p.size)
    }

    /// The size a root map holds, read as [`Document::size`] reads it; also for a checked-out
    /// version (`history/checkout.rs`).
    pub(crate) fn size_of_root(root: &LoroMap) -> DocumentSize {
        let default = DocumentSize::default();
        let valid = |value: Option<f64>| {
            value.filter(|mm| (MIN_DOCUMENT_MM..=MAX_DOCUMENT_MM).contains(mm))
        };
        match (
            valid(Self::read_mm(root, KEY_WIDTH_MM)),
            valid(Self::read_mm(root, KEY_HEIGHT_MM)),
        ) {
            (Some(width), Some(height)) => {
                DocumentSize::new(Length::from_mm(width), Length::from_mm(height))
            }
            _ => default,
        }
    }

    /// This document's underlying Loro replica, for [`crate::paths`]'s
    /// methods only — never exposed outside this crate (ADR 0004 §3).
    pub(crate) const fn loro(&self) -> &LoroDoc {
        &self.loro
    }

    /// An opaque marker of the document's current state: it differs from an earlier one exactly
    /// when something was committed (or merged) in between. For caches built from the whole
    /// document, such as the Pen's index of path ends (`specs/0034-pen-path-extension`
    /// criterion 24).
    #[must_use]
    pub fn version(&self) -> DocumentVersion {
        DocumentVersion(self.loro.state_frontiers())
    }

    fn read_mm(root: &LoroMap, key: &str) -> Option<f64> {
        match root.get(key)?.get_deep_value() {
            LoroValue::Double(value) => Some(value),
            _ => None,
        }
    }

    /// Exports this document's Loro CRDT snapshot — the authoritative
    /// container member, `document.loro` (ADR 0004 §1).
    ///
    /// # Errors
    /// Returns [`SaveError::Encode`] if the snapshot cannot be encoded.
    pub fn export_loro_snapshot(&self) -> Result<Vec<u8>, SaveError> {
        self.loro
            .export(ExportMode::Snapshot)
            .map_err(|_| SaveError::Encode)
    }

    /// Exports a plain, non-authoritative JSON view of the current state —
    /// the container's `document.json` member (ADR 0004 §1) — read from
    /// this one immutable snapshot of the document rather than live state
    /// (ADR 0009 §4).
    ///
    /// `primitive-shapes` renames this view's array from `paths` to
    /// `objects`, in z-order, each entry tagged with its own `shape`
    /// (`"path"` for a path — `specs/0003-primitive-shapes/adrs.md`,
    /// "`document.json`'s `paths` key → `objects`").
    ///
    /// # Errors
    /// Returns [`SaveError::Encode`] if the view cannot be serialized.
    pub fn export_json(&self) -> Result<Vec<u8>, SaveError> {
        let objects = self.json_children(None);
        let view = DocumentJsonView {
            format_version: CURRENT_FORMAT_VERSION,
            size: self.size(),
            pages: self.pages(),
            display_unit: self.display_unit(),
            background: self.background().into(),
            snapping: self.snapping().into(),
            objects,
            manufacturings: self.manufacturings(),
        };
        serde_json::to_vec_pretty(&view).map_err(|_| SaveError::Encode)
    }
}

impl Document {
    /// Puts the raw instance registers of the sketch or clone `id` into its entry.
    fn add_instance_json(&self, id: crate::path_model::NodeId, object: &mut ObjectJson) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let Ok(meta) = tree.get_meta(crate::paths::tree_id_of(id)) else {
            return;
        };
        let raw = |key: &str| {
            let map = crate::instance_codec::read_values_raw(&meta, key);
            (!map.is_empty()).then_some(serde_json::Value::Object(map))
        };
        match object {
            ObjectJson::Sketch { exposed_params, .. } => {
                let items = crate::instance_codec::read_exposed_raw(&meta);
                *exposed_params = (!items.is_empty()).then_some(serde_json::Value::Array(items));
            }
            ObjectJson::Clone {
                instance_params,
                instance_fallback,
                instance_unlinked,
                ..
            } => {
                *instance_params = raw(crate::instance_codec::KEY_INSTANCE_PARAMS);
                *instance_fallback = raw(crate::instance_codec::KEY_INSTANCE_FALLBACK);
                let marks = crate::instance_codec::read_unlinked_raw(&meta);
                *instance_unlinked = (!marks.is_empty()).then_some(serde_json::Value::Array(marks));
            }
            _ => {}
        }
    }

    /// The `document.json` entries of the children of `parent` (`None` is the document root), in
    /// z-order; a group nests its own children.
    fn json_children(&self, parent: Option<crate::path_model::NodeId>) -> Vec<NodeJson> {
        self.children(parent)
            .into_iter()
            .filter_map(|id| {
                let object = if self.is_layer(id) {
                    ObjectJson::Layer {
                        id,
                        objects: self.json_children(Some(id)),
                    }
                } else if self.is_group(id) {
                    ObjectJson::Group {
                        id,
                        objects: self.json_children(Some(id)),
                    }
                } else if let Some((link, overrides, style)) = self.clone_stored(id) {
                    ObjectJson::Clone {
                        id,
                        origin: link.origin,
                        matrix: link.matrix,
                        own_placement: overrides.own_placement,
                        own_style: overrides.own_style,
                        own_adjustments: overrides.own_adjustments,
                        style,
                        instance_params: None,
                        instance_fallback: None,
                        instance_unlinked: None,
                    }
                } else {
                    ObjectJson::from(self.object(id)?)
                };
                let mut object = object;
                self.add_instance_json(id, &mut object);
                let mut registers = self
                    .node_registers(id)
                    .map(RegistersJson::from)
                    .unwrap_or_default();
                if let ObjectJson::Clone {
                    own_style: false, ..
                } = object
                {
                    // Residue on a clone without own style (ADR 0016 §5).
                    registers.opacity = None;
                }
                let trace_mask = matches!(object, ObjectJson::Image { .. })
                    .then(|| self.trace_mask(id))
                    .flatten()
                    .filter(|mask| !mask.is_empty());
                let adjustments = self.stored_adjustments(id);
                Some(NodeJson {
                    object,
                    registers,
                    trace_mask,
                    adjustments,
                })
            })
            .collect()
    }
}

// Deliberately no `impl Default for Document`: there is no safe default
// peer id to mint without a caller-supplied source of randomness, which is
// exactly what `Document::new`'s `peer_id` parameter exists to avoid this
// crate doing itself.

/// The shape of the non-authoritative `document.json` export
/// (ADR 0004 §1).
#[derive(Serialize)]
struct DocumentJsonView {
    format_version: u32,
    size: DocumentSize,
    pages: Vec<crate::Page>,
    display_unit: DisplayUnit,
    background: crate::document_background::BackgroundJson,
    snapping: crate::snapping::SnappingJson,
    objects: Vec<NodeJson>,
    /// The manufacturings with their generic fields and raw process values (ADR 0033 §6).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    manufacturings: Vec<crate::ManufacturingSnapshot>,
}

/// One node of `document.json`: its kind and data, and its registers where they are not at their
/// defaults (`specs/0050-layers-and-objects-panel/adrs.md`, the note of 2026-10-12 on decision 11).
#[derive(Serialize)]
struct NodeJson {
    #[serde(flatten)]
    object: ObjectJson,
    #[serde(flatten)]
    registers: RegistersJson,
    /// An image's stored trace mask (ADR 0061 §11), left out when nothing is stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_mask: Option<crate::trace_mask_model::TraceMask>,
    /// An image's, or an own-adjustments clone's, adjustment registers as read (ADR 0072 §1),
    /// left out when none is stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    adjustments: Option<crate::image_adjust_model::ImageAdjustments>,
}

/// The registers of one node in `document.json`; each is left out at its default.
#[derive(Serialize, Default)]
struct RegistersJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guide: Option<bool>,
}

impl From<crate::node_registers::NodeRegisters> for RegistersJson {
    fn from(registers: crate::node_registers::NodeRegisters) -> Self {
        let opacity = registers.opacity.get();
        Self {
            name: registers.name,
            visible: (!registers.visible).then_some(false),
            locked: registers.locked.then_some(true),
            #[allow(clippy::float_cmp)] // exactly the stored default
            opacity: (opacity != 1.0).then_some(opacity),
            guide: registers.guide.then_some(true),
        }
    }
}

/// One `document.json` object entry, tagged by its own `shape`
/// (`"path"` for a path) rather than serde's default untagged-enum
/// shape, so a non-Rust reader of this non-authoritative view can tell
/// the five kinds apart without guessing from which fields are present.
#[derive(Serialize)]
#[serde(tag = "shape", rename_all = "lowercase")]
enum ObjectJson {
    Sketch {
        id: crate::NodeId,
        graph: curvyo_sketch_core::Sketch,
        placement: crate::Affine,
        style: crate::Style,
        /// The `exposed_params` list as stored (`specs/0099-sketch-pages` criterion 30).
        #[serde(skip_serializing_if = "Option::is_none")]
        exposed_params: Option<serde_json::Value>,
    },
    /// A group (`specs/0023-groups`): no geometry and no style, only its children in z-order.
    Group {
        id: crate::path_model::NodeId,
        objects: Vec<NodeJson>,
    },
    /// A layer (`specs/0050-layers-and-objects-panel`): a group at the top level with the layer
    /// marker, its children in z-order.
    Layer {
        id: crate::path_model::NodeId,
        objects: Vec<NodeJson>,
    },
    /// A dimension (`specs/0070-measure-and-dimension`, ADR 0027): kind, slots, place and the
    /// last measured value; no geometry of its own.
    Dimension {
        id: crate::path_model::NodeId,
        kind: crate::dimension_model::DimensionKind,
        slots: crate::dimension_model::DimensionSlots,
        place: crate::dimension_model::DimensionPlace,
        kept: crate::dimension_model::Measured,
    },
    /// A registration mark (`specs/0121-print-and-cut`, ADR 0081 §1): number, centre, size.
    RegistrationMark {
        id: crate::path_model::NodeId,
        number: u8,
        centre: crate::units::Point,
        diameter: crate::units::Length,
        line: crate::units::Length,
    },
    /// A linked clone (`specs/0049-linked-clone` criterion 40): the original it follows and its
    /// matrix `[a, b, c, d, e, f]`, no outline. Listed whether or not its chain resolves. The
    /// override flags are listed only when set, and `style` only with `own_style`; its own
    /// `opacity` is the node register (`specs/0057-clone-overrides` criterion 39).
    Clone {
        id: crate::path_model::NodeId,
        origin: crate::path_model::NodeId,
        matrix: crate::affine::Affine,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        own_placement: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        own_style: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        own_adjustments: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<crate::style_model::Style>,
        /// The `instance_params` map as stored (`specs/0099-sketch-pages` criterion 30).
        #[serde(skip_serializing_if = "Option::is_none")]
        instance_params: Option<serde_json::Value>,
        /// The `instance_fallback` map as stored.
        #[serde(skip_serializing_if = "Option::is_none")]
        instance_fallback: Option<serde_json::Value>,
        /// The `instance_unlinked` list as stored (`specs/0099-sketch-pages` criterion 37).
        #[serde(skip_serializing_if = "Option::is_none")]
        instance_unlinked: Option<serde_json::Value>,
    },
    /// A text object (`specs/0074-text-objects`, ADR 0020 §1): string, font reference, size and
    /// placement `[a, b, c, d, e, f]`; never outlines.
    Text {
        id: crate::path_model::NodeId,
        string: String,
        font: crate::text_model::FontRef,
        size: crate::units::Length,
        /// The layout values (`specs/0079-text-layout-controls`); left out at their defaults.
        #[serde(skip_serializing_if = "crate::text_model::TextLayoutValues::is_default")]
        layout: crate::text_model::TextLayoutValues,
        /// The per-character values (ADR 0021); left out when there are none.
        #[serde(skip_serializing_if = "crate::text_chars::TextChars::is_empty")]
        chars: crate::text_chars::TextChars,
        placement: crate::affine::Affine,
        style: crate::style_model::Style,
    },
    Path {
        id: crate::path_model::NodeId,
        closed: bool,
        style: crate::style_model::Style,
        anchors: Vec<crate::path_model::AnchorSnapshot>,
        /// A compound path's further outlines; left out when there are none,
        /// so an ordinary path exports as before.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        extra_subpaths: Vec<crate::path_model::SubpathSnapshot>,
        #[serde(skip_serializing_if = "crate::CurveKind::is_bezier")]
        curve_kind: crate::CurveKind,
        #[serde(skip_serializing_if = "Option::is_none")]
        spiro_transform: Option<crate::Affine>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        width_points: Vec<crate::WidthPoint>,
        /// The path's own `rotation` register (`specs/0005-object-
        /// transform/adrs.md`): orientation only, never consulted to
        /// reconstruct geometry — see [`crate::path_model::PathSnapshot::rotation`].
        rotation: crate::units::Angle,
    },
    /// An image: the asset's hash and its frame, never the bytes (ADR 0017 §3).
    Image {
        id: crate::path_model::NodeId,
        asset: crate::assets::AssetHash,
        placement: crate::affine::Affine,
    },
    Rect {
        id: crate::path_model::NodeId,
        bounds: crate::primitive_model::RectBounds,
        corner_radii: crate::corner_radii::CornerRadii,
        style: crate::style_model::Style,
        rotation: crate::units::Angle,
    },
    Ellipse {
        id: crate::path_model::NodeId,
        frame: crate::primitive_model::EllipseFrame,
        /// The arc; left out for a whole ellipse, so it exports as before.
        #[serde(skip_serializing_if = "Option::is_none")]
        arc: Option<crate::ellipse_arc::EllipseArc>,
        /// The curve as a fraction; left out at the default.
        #[serde(skip_serializing_if = "Option::is_none")]
        curve: Option<f64>,
        /// The Rounded value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        rounded: Option<f64>,
        style: crate::style_model::Style,
        rotation: crate::units::Angle,
    },
    Polygon {
        id: crate::path_model::NodeId,
        frame: crate::primitive_model::StarFrame,
        point_count: u32,
        /// The curve as a fraction; left out at the default.
        #[serde(skip_serializing_if = "Option::is_none")]
        curve: Option<f64>,
        /// The Rounded value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        rounded: Option<f64>,
        /// The Randomized value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        randomized: Option<f64>,
        /// The seed of the Randomized value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        seed: Option<u32>,
        style: crate::style_model::Style,
        rotation: crate::units::Angle,
    },
    Star {
        id: crate::path_model::NodeId,
        frame: crate::primitive_model::StarFrame,
        point_count: u32,
        inner_ratio: f64,
        /// The curve as a fraction; left out at the default.
        #[serde(skip_serializing_if = "Option::is_none")]
        curve: Option<f64>,
        /// The Rounded value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        rounded: Option<f64>,
        /// The Randomized value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        randomized: Option<f64>,
        /// The seed of the Randomized value; left out at 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        seed: Option<u32>,
        style: crate::style_model::Style,
        rotation: crate::units::Angle,
    },
}

/// A curve as `document.json` shows it: `None` at the shape's default.
fn stored_curve(curve: crate::ring_curve::Curve, default: crate::ring_curve::Curve) -> Option<f64> {
    (curve != default).then_some(curve.get())
}

impl From<crate::primitive_model::ObjectSnapshot> for ObjectJson {
    #[allow(clippy::too_many_lines)] // Exhaustive serialization keeps each object variant together.
    fn from(snapshot: crate::primitive_model::ObjectSnapshot) -> Self {
        use crate::primitive_model::{ObjectSnapshot, Shape};
        match snapshot {
            ObjectSnapshot::Sketch(sketch) => Self::Sketch {
                id: sketch.id,
                graph: sketch.graph,
                placement: sketch.placement,
                style: sketch.style,
                exposed_params: None,
            },
            ObjectSnapshot::RegistrationMark(mark) => Self::RegistrationMark {
                id: mark.id,
                number: mark.number,
                centre: mark.centre,
                diameter: mark.diameter,
                line: mark.line,
            },
            ObjectSnapshot::Dimension(dimension) => Self::Dimension {
                id: dimension.id,
                kind: dimension.kind,
                slots: dimension.slots,
                place: dimension.place,
                kept: dimension.kept,
            },
            ObjectSnapshot::Path(path) => Self::Path {
                id: path.id,
                closed: path.closed,
                style: path.style,
                anchors: path.anchors,
                extra_subpaths: path.extra_subpaths,
                curve_kind: path.curve_kind,
                spiro_transform: path.spiro_transform.map(|value| *value),
                width_points: path.width_points,
                rotation: path.rotation,
            },
            ObjectSnapshot::Text(text) => Self::Text {
                id: text.id,
                string: text.string,
                font: text.font,
                size: text.size,
                layout: text.layout,
                chars: text.chars,
                placement: text.placement,
                style: text.style,
            },
            ObjectSnapshot::Image(image) => Self::Image {
                id: image.id,
                asset: image.asset,
                placement: image.placement,
            },
            ObjectSnapshot::Clone(clone) => Self::Clone {
                id: clone.id(),
                origin: clone.chain.origin,
                matrix: clone.chain.matrix,
                own_placement: clone.chain.own_placement,
                own_style: clone.chain.own_style,
                own_adjustments: clone.chain.own_adjustments,
                style: clone.chain.own_style.then_some(clone.resolved.style),
                instance_params: None,
                instance_fallback: None,
                instance_unlinked: None,
            },
            ObjectSnapshot::Primitive(primitive) => {
                let id = primitive.id;
                let style = primitive.style;
                let rotation = primitive.rotation;
                match primitive.shape {
                    Shape::Rect {
                        bounds,
                        corner_radii,
                    } => Self::Rect {
                        id,
                        bounds,
                        corner_radii,
                        style,
                        rotation,
                    },
                    Shape::Ellipse {
                        frame,
                        arc,
                        curve,
                        rounded,
                    } => Self::Ellipse {
                        id,
                        frame,
                        arc,
                        curve: stored_curve(curve, crate::ring_curve::Curve::ELLIPSE_DEFAULT),
                        rounded: (!rounded.is_zero()).then_some(rounded.get()),
                        style,
                        rotation,
                    },
                    Shape::Polygon {
                        frame,
                        point_count,
                        curve,
                        rounded,
                        randomized,
                    } => Self::Polygon {
                        id,
                        frame,
                        point_count: point_count.get(),
                        curve: stored_curve(curve, crate::ring_curve::Curve::POLYGON_DEFAULT),
                        rounded: (!rounded.is_zero()).then_some(rounded.get()),
                        randomized: (!randomized.is_zero()).then_some(randomized.amount()),
                        seed: (!randomized.is_zero()).then_some(randomized.seed().get()),
                        style,
                        rotation,
                    },
                    Shape::Star {
                        frame,
                        point_count,
                        inner_ratio,
                        curve,
                        rounded,
                        randomized,
                    } => Self::Star {
                        curve: stored_curve(curve, crate::ring_curve::Curve::POLYGON_DEFAULT),
                        rounded: (!rounded.is_zero()).then_some(rounded.get()),
                        randomized: (!randomized.is_zero()).then_some(randomized.amount()),
                        seed: (!randomized.is_zero()).then_some(randomized.seed().get()),
                        id,
                        frame,
                        point_count: point_count.get(),
                        inner_ratio: inner_ratio.get(),
                        style,
                        rotation,
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_defaults_to_a4_portrait() {
        let document = Document::new(1);
        let size = document.size();
        assert!((size.width.as_mm() - 210.0).abs() < f64::EPSILON);
        assert!((size.height.as_mm() - 297.0).abs() < f64::EPSILON);
    }

    #[test]
    fn new_document_uses_the_given_peer_id() {
        let document = Document::new(0x1234_5678_9abc_def0);
        assert_eq!(document.loro.peer_id(), 0x1234_5678_9abc_def0);
    }

    #[test]
    fn loro_snapshot_round_trips_the_size() {
        let original = Document::new(1);
        let snapshot = original.export_loro_snapshot().expect("export");
        let reopened = Document::from_loro_snapshot(2, &snapshot).expect("import");
        assert_eq!(original.size(), reopened.size());
    }

    #[test]
    fn from_loro_snapshot_uses_the_given_peer_id() {
        let original = Document::new(1);
        let snapshot = original.export_loro_snapshot().expect("export");
        let reopened =
            Document::from_loro_snapshot(0x1234_5678_9abc_def0, &snapshot).expect("import");
        assert_eq!(reopened.loro.peer_id(), 0x1234_5678_9abc_def0);
    }

    #[test]
    fn json_export_matches_the_live_size() {
        let document = Document::new(1);
        let json = document.export_json().expect("export");
        let value: serde_json::Value = serde_json::from_slice(&json).expect("parse");
        assert_eq!(value["format_version"], CURRENT_FORMAT_VERSION);
        assert!((value["size"]["width"].as_f64().unwrap() - 210.0).abs() < f64::EPSILON);
    }

    /// Caches of the whole document (the Pen's end index, the session's object read) key on
    /// `version()`: it must change on a local commit and on a merge from another replica, and
    /// stay the same when nothing happened.
    #[test]
    fn the_version_changes_on_a_local_commit_and_on_a_remote_merge() {
        let a = Document::new(1);
        let first = a.version();
        assert_eq!(a.version(), first, "reading is not a change");
        let id = a.create_rect(crate::RectBounds {
            origin: crate::Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        let after_local = a.version();
        assert_ne!(after_local, first, "a local commit");

        let b = Document::from_loro_snapshot(2, &a.export_loro_snapshot().expect("snapshot"))
            .expect("peer B opens it");
        assert_eq!(b.version(), after_local, "the same state, the same version");
        b.translate_objects(&[id], crate::Vec2::new(5.0, 0.0))
            .expect("B moves the rectangle");
        assert_eq!(a.version(), after_local, "A has not seen it yet");

        let from_b = b.loro().export(loro::ExportMode::all_updates()).expect("B");
        a.loro().import(&from_b).expect("A merges B");
        assert_ne!(a.version(), after_local, "a remote merge");
        assert_eq!(
            a.version(),
            b.version(),
            "both replicas hold the same state"
        );
    }
}
