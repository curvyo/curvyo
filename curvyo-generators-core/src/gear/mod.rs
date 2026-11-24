//! The spur gear: module, teeth, pressure angle, bore with keyway and a solid or spoked web.

mod involute;
mod schema;
mod web;

use curvyo_document_core::Point;
use curvyo_document_core::{Length, Vec2};
use curvyo_extension_core::{
    ClosedPath, Edit, GenerateRequest, Generated, Handle, Param, ParamValue, Part, StoredParams,
};

use crate::values::Values;
use involute::{Profile, Teeth};
use web::Spokes;

pub(crate) use schema::groups;

/// The material left between two features (bore and root circle, hub and bore), in mm.
const WALL: f64 = 1.0;

/// Every gear parameter, in order.
pub(crate) fn params() -> Vec<Param> {
    groups()
        .into_iter()
        .flat_map(|group| group.params)
        .collect()
}

fn raw_mm(params: &StoredParams, key: &str, default: f64) -> f64 {
    params
        .get(key)
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.length().map(Length::as_mm))
        .unwrap_or(default)
}

/// The stored values with the maker's edit applied before the schema clamps them.
fn edited(request: &GenerateRequest<'_>) -> StoredParams {
    let mut raw = request.params.clone();
    let mut put = |key: &str, mm: f64| {
        raw.insert(
            key.to_owned(),
            ParamValue::Length(Length::from_mm(mm)).to_stored(),
        );
    };
    match request.edit {
        Some(Edit::Param("pitch_diameter")) => {
            let teeth = request
                .params
                .get("teeth")
                .and_then(ParamValue::from_stored)
                .and_then(|value| value.count())
                .unwrap_or(24)
                .clamp(schema::TEETH_RANGE.0, schema::TEETH_RANGE.1);
            let diameter = raw_mm(request.params, "pitch_diameter", 48.0);
            put("module", diameter / f64::from(teeth));
        }
        Some(Edit::Scale(factor)) => {
            // The outer diameter m (z + 2) follows the pointer by whole teeth (0141 criterion 5).
            let teeth = request
                .params
                .get("teeth")
                .and_then(ParamValue::from_stored)
                .and_then(|value| value.count())
                .unwrap_or(24);
            // The schema clamps it to 6..=200 and reports that as the teeth's own clamp.
            let scaled = (factor.0 * f64::from(teeth + 2) - 2.0).round().max(0.0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let scaled = scaled.min(f64::from(u32::MAX)) as u32;
            raw.insert("teeth".to_owned(), ParamValue::Count(scaled).to_stored());
        }
        _ => {}
    }
    raw
}

/// Generates the gear `request` asks for.
pub(crate) fn generate(request: &GenerateRequest<'_>) -> Generated {
    let schema = params();
    let mut v = Values::read(&schema, &edited(request));
    let module = v.mm("module");
    let teeth = v.count("teeth");
    v.put_mm("pitch_diameter", module * f64::from(teeth));
    let pressure = v.degrees("pressure_angle").to_radians();
    let profile = Profile::new(Teeth {
        module,
        count: teeth,
        pressure,
    });
    let root = profile.root_radius;
    let spoked = v.choice("web") == "spokes";

    // Dependent bounds, in the order the features sit inside each other.
    if spoked {
        v.limit_mm("bore", 0.0, root);
        v.limit_mm("rim_width", 0.0, 0.3 * root);
    } else {
        v.limit_mm("bore", 0.0, 2.0 * (root - WALL).max(0.0));
    }
    let bore = v.mm("bore");
    let spoke_count = v.count("spokes");
    let rim = v.mm("rim_width");
    let rim_inner = root - rim;
    let fillet = (0.5 * rim).min(0.2 * rim_inner).max(0.1);
    if spoked {
        let hub_min =
            (bore + 2.0 * WALL).max(2.0 * Spokes::min_hub_radius(spoke_count, rim, fillet) + 1e-3);
        let hub_max = 2.0 * (rim_inner - 3.0 * WALL - 2.0 * fillet);
        v.limit_mm("hub_diameter", hub_min, hub_max.max(hub_min));
    }
    let hub = v.mm("hub_diameter");
    let keyway = keyway(&mut v, bore, spoked.then_some(hub / 2.0), root);

    let paths = paths(
        &profile,
        spoked.then_some(Spokes {
            count: spoke_count,
            spoke_width: rim,
            hub_radius: hub / 2.0,
            rim_inner_radius: rim_inner,
            fillet,
        }),
        bore,
        keyway,
    );

    let handles = handles(&Sizes {
        bore,
        hub: spoked.then_some(hub),
        rim_inner,
    });
    let (params, clamps) = v.finish();
    Generated {
        params,
        parts: vec![Part { paths }],
        handles,
        clamps,
        automatic: Vec::new(),
        states: Vec::new(),
        limits: Vec::new(),
    }
}

/// The closed paths of the gear: the toothed outline, the spoke windows of a spoked web, the bore.
fn paths(
    profile: &Profile,
    spokes: Option<Spokes>,
    bore: f64,
    keyway: Option<(f64, f64)>,
) -> Vec<ClosedPath> {
    let mut paths = vec![profile.outline()];
    if let Some(spokes) = spokes {
        paths.extend(spokes.windows());
    }
    if bore > 0.0 {
        paths.push(web::bore(bore / 2.0, keyway));
    }
    paths
}

/// The keyway the bore carries as (width, depth), clamping its two numbers inside the web: `hub_radius`
/// is the hub of a spoked gear, `None` for a solid one.
fn keyway(v: &mut Values, bore: f64, hub_radius: Option<f64>, root: f64) -> Option<(f64, f64)> {
    if bore <= 0.0 || !v.switch("keyway") {
        return None;
    }
    let limit = hub_radius.map_or(root - WALL, |hub| hub - WALL);
    v.limit_mm("keyway_width", 0.0, 0.9 * bore);
    v.limit_mm("keyway_depth", 0.0, (limit - bore / 2.0).max(0.0));
    let (width, depth) = (v.mm("keyway_width"), v.mm("keyway_depth"));
    let (min_depth, _) = v.schema_range("keyway_depth");
    (depth >= min_depth).then_some((width, depth))
}

/// The radii the drag handles sit on.
struct Sizes {
    bore: f64,
    /// The hub diameter of a spoked gear.
    hub: Option<f64>,
    rim_inner: f64,
}

/// The drag handles: bore when there is one, hub and rim when spoked. There is no pitch diameter
/// knob: the corner handle writes Teeth on the same diagonal (UX notes `0141` §15.6, Proposal).
fn handles(sizes: &Sizes) -> Vec<Handle> {
    let mut handles = Vec::new();
    if sizes.bore > 0.0 {
        handles.push(radial("bore", sizes.bore / 2.0, 0.0, 1.0, 2.0));
    }
    if let Some(hub) = sizes.hub {
        handles.push(radial("hub_diameter", hub / 2.0, 180.0, 1.0, 2.0));
        handles.push(radial("rim_width", sizes.rim_inner, 135.0, -1.0, 1.0));
    }
    handles
}

/// A handle on a circle of `radius` at `degrees`, growing outward (`sign` 1) or inward (-1).
fn radial(key: &'static str, radius: f64, degrees: f64, sign: f64, rate: f64) -> Handle {
    let angle = degrees.to_radians();
    let (across, up) = (libm::cos(angle), libm::sin(angle));
    Handle {
        key,
        anchor: Point::new(radius * across, radius * up),
        direction: Vec2::new(sign * across, sign * up),
        rate,
    }
}
