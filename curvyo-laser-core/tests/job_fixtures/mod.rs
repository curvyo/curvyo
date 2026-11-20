//! `job_input` fixtures for the job tests of `specs/0029-laser-job-preview-and-output`: pages,
//! blocks and leaves built without a session, the node ids taken from a scratch document.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value
)]

use std::collections::BTreeMap;

use curvyo_document_core::{Document, DocumentSize, Length, NodeId, Point, RectBounds, Vec2};
use curvyo_laser_core::adjust::DitherSettings;
use curvyo_laser_core::job::{
    BlockInput, BlockSource, CutOutItem, JobInput, LeafGeometry, LeafInput, LeafOutline,
    MachineInput, Ring, Start,
};
use curvyo_laser_core::library::{LaserSets, Material, MaterialRead, Set, SetSource};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{
    ImageSettings, Interval, MachineTie, OperationOptions, Overscan, Passes, PowerPercent,
    ScanAngle, SetKind, SetNumbers, Settings, SettingsRead, StoredSettings, Thickness, ZOffset,
    read_settings,
};
use curvyo_laser_core::{FeedRate, OriginCorner, ProfileValues};
use serde_json::{Value, json};

/// The profile id the fixtures' page copy carries.
pub const PROFILE: &str = "6f1c2b0a-5d4e-4f3a-9b2c-1d0e8f7a6b5c";
/// The material id of the fixture jobs' pick.
pub const BIRCH: &str = "0b3c7f1e-8f0e-4c4a-9a51-2f6c1c1e9d10";

/// `n` distinct node ids.
/// The dither fields of an operation's Image settings.
pub fn dither_of(image: &ImageSettings) -> DitherSettings {
    DitherSettings {
        mode: image.dither,
        threshold_pct: image.threshold_pct,
        screen_size_um: image.screen_size_hundredths * 10,
        screen_angle_deg: image.screen_angle_deg,
    }
}

/// `leaf` with the picture's dither set to `dither`, any other leaf unchanged: the goldens that
/// set a picture's dither on its operation before the Dither moved to the picture (ADR 0072
/// amendment A4) keep their dots.
pub fn with_dither(mut leaf: LeafGeometry, dither: DitherSettings) -> LeafGeometry {
    if let LeafGeometry::Image { adjust, .. } = &mut leaf {
        adjust.dither = dither;
    }
    leaf
}

pub fn ids(n: usize) -> Vec<NodeId> {
    let d = Document::new(1);
    (0..n)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let x = i as f64;
            d.create_rect(RectBounds::from_corners(
                Point::new(x, 0.0),
                Point::new(x + 1.0, 1.0),
            ))
        })
        .collect()
}

/// A builder of one job input with a 400 x 300 mm GRBL bed, bottom-left origin, M4, S-max 1000.
pub struct Page {
    pub input: JobInput,
    ids: Vec<NodeId>,
    next: usize,
}

impl Page {
    pub fn new() -> Self {
        Self {
            input: JobInput {
                name: "Laser 1".to_owned(),
                machine: MachineInput {
                    profile_id: PROFILE.to_owned(),
                    values: ProfileValues {
                        name: "Diode 10 W".to_owned(),
                        ..ProfileValues::default()
                    },
                    bed: DocumentSize::from_mm(400.0, 300.0),
                    damaged: false,
                },
                start: Start::default(),
                start_damaged: false,
                fixture_material: None,
                blocks: Vec::new(),
            },
            ids: ids(400),
            next: 0,
        }
    }

    pub fn id(&mut self) -> NodeId {
        self.next += 1;
        self.ids[self.next - 1]
    }

    pub fn values(&mut self) -> &mut ProfileValues {
        &mut self.input.machine.values
    }

    pub fn corner(mut self, corner: OriginCorner) -> Self {
        self.values().origin = corner;
        self
    }

    pub fn start(mut self, start: Start) -> Self {
        self.input.start = start;
        self
    }

    /// The next operation row's digit: one more than the operation blocks so far.
    fn next_digit(&self) -> Ring {
        let operations = self
            .input
            .blocks
            .iter()
            .filter(|b| b.ring != Ring::Fixture)
            .count();
        Ring::Operation(u32::try_from(operations).unwrap() + 1)
    }

    /// Adds an operation block of `kind` with own numbers and the leaves `leaves`. Superseded by
    /// the Dither on the picture (2026-11-20): each picture leaf holds the dither of `options`
    /// (the mode moved from the operation onto the picture, ADR 0072 amendment A4), so the fixtures
    /// burn the dots they burned; [`Page::operation_as_is`] keeps the leaves' own dither.
    pub fn operation(
        &mut self,
        kind: OperationKind,
        name: &str,
        settings: SettingsRead,
        options: OperationOptions,
        leaves: Vec<LeafGeometry>,
    ) -> NodeId {
        let dither = dither_of(&options.image);
        let leaves = leaves.into_iter().map(|l| with_dither(l, dither)).collect();
        self.operation_as_is(kind, name, settings, options, leaves)
    }

    /// Adds an operation block of `kind` with own numbers and the leaves `leaves` as they are.
    pub fn operation_as_is(
        &mut self,
        kind: OperationKind,
        name: &str,
        settings: SettingsRead,
        options: OperationOptions,
        leaves: Vec<LeafGeometry>,
    ) -> NodeId {
        let id = self.id();
        let leaves = leaves
            .into_iter()
            .enumerate()
            .map(|(i, geometry)| LeafInput {
                node: self.id(),
                stacking: u32::try_from(i).unwrap(),
                geometry,
            })
            .collect();
        let ring = self.next_digit();
        self.input.blocks.push(BlockInput {
            id,
            ring,
            name: name.to_owned(),
            kind: KindRead::Known(kind),
            source: BlockSource::Operation {
                settings,
                options: Ok(options),
                margin: Length::from_mm(0.0),
                grid: None,
            },
            leaves,
            cut_out: Vec::new(),
        });
        id
    }

    /// Adds a Cut-out block whose items are the given leaf lists.
    pub fn cut_out(
        &mut self,
        name: &str,
        settings: SettingsRead,
        margin: f64,
        items: Vec<Vec<Vec<LeafOutline>>>,
    ) -> NodeId {
        let id = self.id();
        let cut_out = items
            .into_iter()
            .enumerate()
            .map(|(i, leaves)| CutOutItem {
                item: self.id(),
                stacking: u32::try_from(i).unwrap(),
                leaves,
            })
            .collect();
        let ring = self.next_digit();
        self.input.blocks.push(BlockInput {
            id,
            ring,
            name: name.to_owned(),
            kind: KindRead::Known(OperationKind::CutOut),
            source: BlockSource::Operation {
                settings,
                options: Ok(OperationOptions::default()),
                margin: Length::from_mm(margin),
                grid: None,
            },
            leaves: Vec::new(),
            cut_out,
        });
        id
    }

    /// Adds a fixture block of `kind`.
    pub fn fixture(&mut self, kind: OperationKind, leaves: Vec<LeafGeometry>) -> NodeId {
        let id = self.id();
        let leaves = leaves
            .into_iter()
            .map(|geometry| LeafInput {
                node: self.id(),
                stacking: 0,
                geometry,
            })
            .collect();
        self.input.blocks.push(BlockInput {
            id,
            ring: Ring::Fixture,
            name: format!("Fixture jobs · {}", kind.label()),
            kind: KindRead::Known(kind),
            source: BlockSource::Fixture { jobs: vec![id] },
            leaves,
            cut_out: Vec::new(),
        });
        id
    }
}

/// Own numbers: `power` %, `speed` mm/min, `passes`.
pub fn own(kind: OperationKind, power: f64, speed: u32, passes: u8) -> SettingsRead {
    settings(
        kind,
        json!({"main": {"power_pct": power, "speed_mm_min": speed, "passes": passes}}),
    )
}

/// Own Engrave numbers with an interval.
pub fn own_engrave(kind: OperationKind, power: f64, speed: u32, interval: f64) -> SettingsRead {
    settings(
        kind,
        json!({"main": {"power_pct": power, "speed_mm_min": speed, "passes": 1, "interval_mm": interval}}),
    )
}

pub fn settings(kind: OperationKind, value: Value) -> SettingsRead {
    let read = read_settings(Some(&value), kind);
    assert!(matches!(read, SettingsRead::Stored(_)), "{value}: {read:?}");
    read
}

/// The stored settings of a read value.
pub fn stored(read: &SettingsRead) -> &StoredSettings {
    match read {
        SettingsRead::Stored(stored) => stored,
        _ => panic!("not stored"),
    }
}

/// Options: bidirectional, `angle` degrees, crosshatch, overscan %, Z offset mm.
pub fn options(
    bidirectional: bool,
    angle: f64,
    crosshatch: bool,
    overscan: u64,
    z: f64,
) -> OperationOptions {
    OperationOptions {
        crosshatch,
        bidirectional,
        scan_angle: ScanAngle::from_degrees(angle).unwrap(),
        overscan: Overscan::new(overscan).unwrap(),
        z_offset: ZOffset::from_mm(z).unwrap(),
        image: curvyo_laser_core::settings::ImageSettings::DEFAULT,
    }
}

/// Options with no overscan, the defaults otherwise.
pub fn plain() -> OperationOptions {
    options(true, 0.0, false, 0, 0.0)
}

fn corner(p: Point) -> (Point, Vec2, Vec2) {
    (p, Vec2::ZERO, Vec2::ZERO)
}

/// A rectangle outline from `(x, y)`, `w` by `h` mm, clockwise as seen (Y down).
pub fn rect(x: f64, y: f64, w: f64, h: f64) -> LeafOutline {
    LeafOutline {
        anchors: vec![
            corner(Point::new(x, y)),
            corner(Point::new(x + w, y)),
            corner(Point::new(x + w, y + h)),
            corner(Point::new(x, y + h)),
        ],
        closed: true,
    }
}

/// The same rectangle drawn the other way (a hole by the nonzero rule).
pub fn rect_reversed(x: f64, y: f64, w: f64, h: f64) -> LeafOutline {
    let mut outline = rect(x, y, w, h);
    outline.anchors.reverse();
    outline
}

/// A circle of four cubics around `(cx, cy)`.
pub fn circle(cx: f64, cy: f64, r: f64) -> LeafOutline {
    let k = 0.552_284_749_830_793_6 * r;
    LeafOutline {
        anchors: vec![
            (
                Point::new(cx + r, cy),
                Vec2::new(0.0, -k),
                Vec2::new(0.0, k),
            ),
            (
                Point::new(cx, cy + r),
                Vec2::new(k, 0.0),
                Vec2::new(-k, 0.0),
            ),
            (
                Point::new(cx - r, cy),
                Vec2::new(0.0, k),
                Vec2::new(0.0, -k),
            ),
            (
                Point::new(cx, cy - r),
                Vec2::new(-k, 0.0),
                Vec2::new(k, 0.0),
            ),
        ],
        closed: true,
    }
}

/// An open straight line.
pub fn line(a: Point, b: Point) -> LeafOutline {
    LeafOutline {
        anchors: vec![corner(a), corner(b)],
        closed: false,
    }
}

pub fn leaf(outlines: Vec<LeafOutline>) -> LeafGeometry {
    LeafGeometry::Outlines(outlines)
}

pub fn image(x: f64, y: f64, w: f64, h: f64) -> LeafGeometry {
    LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(x, y, w, h),
        asset: curvyo_document_core::AssetHash::of(b"unreadable"),
        pixels: None,
    }
}

/// The frame corners of an unrotated `w` x `h` box at `(x, y)`.
pub fn frame(x: f64, y: f64, w: f64, h: f64) -> [Point; 4] {
    [
        Point::new(x, y),
        Point::new(x + w, y),
        Point::new(x + w, y + h),
        Point::new(x, y + h),
    ]
}

/// An image leaf of `pixels` at `(x, y)`, `w` x `h` mm (`specs/0123`).
pub fn picture(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    pixels: curvyo_image_core::WorkingImage,
) -> LeafGeometry {
    LeafGeometry::Image {
        adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
        frame: frame(x, y, w, h),
        asset: curvyo_document_core::AssetHash::of(&pixels.pixels),
        pixels: Some(std::sync::Arc::new(pixels)),
    }
}

/// A `width` x `height` opaque working image whose pixel `(x, y)` has the grey `grey(x, y)`.
pub fn grey_image(
    width: u32,
    height: u32,
    grey: impl Fn(u32, u32) -> u8,
) -> curvyo_image_core::WorkingImage {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            let g = grey(x, y);
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    curvyo_image_core::WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

/// A loaded material `id` named `name` with the sets `sets` for [`PROFILE`].
pub fn material(id: &str, name: &str, sets: &[(SetKind, f64, u32, u8)]) -> MaterialRead {
    let sets: BTreeMap<SetKind, Set> = sets
        .iter()
        .map(|&(kind, power, speed, passes)| {
            (
                kind,
                Set {
                    numbers: SetNumbers {
                        settings: Settings {
                            power: PowerPercent::from_percent(power).unwrap(),
                            speed: FeedRate::new(u64::from(speed)).unwrap(),
                            passes: Passes::new(u64::from(passes)).unwrap(),
                        },
                        interval: (kind == SetKind::Engrave)
                            .then(|| Interval::from_mm(0.1).unwrap()),
                    },
                    source: SetSource::Typed,
                },
            )
        })
        .collect();
    MaterialRead::Loaded(Material {
        id: id.to_owned(),
        modified: 1,
        name: name.to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        lasers: vec![LaserSets {
            machine: MachineTie {
                id: PROFILE.to_owned(),
                name: "Diode 10 W".to_owned(),
            },
            sets,
        }],
        tests: Vec::new(),
        history: Vec::new(),
    })
}
