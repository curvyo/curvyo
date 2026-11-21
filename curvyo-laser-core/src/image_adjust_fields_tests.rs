use super::*;
use crate::image_adjust_field_specs::{
    LEVELS_GAMMA as LG, LOCAL_RADIUS as LR, SHARPEN_RADIUS as SR,
};

#[test]
fn every_kind_reads_back_its_default_fields() {
    for kind in StepKind::ALL {
        let step = kind.default_step(100);
        let fields = step.fields();
        let read = ManualStep::read(kind.key(), |key| {
            fields.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
        });
        assert_eq!(read, Ok(step), "{kind:?}");
        assert_eq!(StepKind::from_key(kind.key()), Some(kind));
    }
}

#[test]
fn damage_is_named() {
    assert_eq!(
        ManualStep::read("vignette", |_| Some(1)),
        Err(StepDamage::UnknownKind)
    );
    assert_eq!(
        ManualStep::read("levels", |k| (k != "gamma_centi").then_some(5)),
        Err(StepDamage::Missing("gamma_centi"))
    );
    assert_eq!(
        ManualStep::read("sharpen", |k| Some(if k == "amount_pct" {
            501
        } else {
            100
        })),
        Err(StepDamage::OutOfRange("amount_pct")),
    );
    let levels = |b, w| {
        ManualStep::read("levels", move |k| {
            Some(match k {
                "black_pct" => b,
                "white_pct" => w,
                _ => 100,
            })
        })
    };
    assert_eq!(levels(50, 51), Err(StepDamage::OutOfRange("white_pct")));
    assert!(levels(50, 52).is_ok());
}

/// Criterion 5: a crop stays inside the picture, a curve is 2 to 8 points with Input
/// increasing from 0 to 100, Channel mix takes negative weights.
#[test]
fn slice_2_fields_read_and_refuse_their_damage() {
    let crop = |l, w| {
        ManualStep::read("crop", move |k| {
            Some(match k {
                "left_pct" => l,
                "width_pct" => w,
                "top_pct" => 0,
                _ => 100,
            })
        })
    };
    assert_eq!(
        crop(20, 80),
        Ok(ManualStep::Crop {
            pct: [20, 0, 80, 100]
        })
    );
    assert_eq!(crop(21, 80), Err(StepDamage::OutOfRange("width_pct")));
    let mix = ManualStep::read("channel_mix", |k| {
        Some(if k == "red_pct" { -150 } else { 30 })
    });
    assert_eq!(
        mix,
        Ok(ManualStep::ChannelMix {
            weights: [-150, 30, 30]
        })
    );
    let curve = |pairs: &'static [(&'static str, i64)]| {
        ManualStep::read("curves", |k| {
            pairs.iter().find(|(p, _)| *p == k).map(|(_, v)| *v)
        })
    };
    let s = curve(&[
        ("in_0", 0),
        ("out_0", 0),
        ("in_1", 40),
        ("out_1", 60),
        ("in_2", 100),
        ("out_2", 100),
    ]);
    assert_eq!(
        s,
        Ok(ManualStep::curve(&[(0, 0), (40, 60), (100, 100)]).unwrap())
    );
    assert_eq!(
        curve(&[("in_0", 0), ("out_0", 0), ("in_1", 100)]),
        Err(StepDamage::Missing("out_1"))
    );
    assert!(
        curve(&[("in_0", 5), ("out_0", 0), ("in_1", 100), ("out_1", 100)]).is_err(),
        "starts at 0"
    );
    assert!(
        curve(&[
            ("in_0", 0),
            ("out_0", 0),
            ("in_1", 50),
            ("out_1", 0),
            ("in_2", 50),
            ("out_2", 9),
            ("in_3", 100),
            ("out_3", 100)
        ])
        .is_err(),
        "increasing"
    );
    assert_eq!(StepKind::Crop.band(), Band::Geometry);
    assert_eq!(StepKind::ChannelMix.band(), Band::Tone);
    assert!(StepKind::Resize < StepKind::ChannelMix && StepKind::Levels < StepKind::Curves);
}

#[test]
fn typed_values_round_to_the_step_and_refuse_out_of_range() {
    assert_eq!(SR.typed(0.104), Ok(100));
    assert_eq!(LR.typed(3.04), Ok(3_000));
    assert_eq!(LR.typed(20.1), Err("Radius is 0.2 to 20.0 mm."));
    assert_eq!(LG.typed(1.234), Ok(123));
    assert_eq!(LG.typed(f64::NAN), Err("Gamma is 0.10 to 4.00."));
    assert!((LR.shown(3_000) - 3.0).abs() < 1e-9);
}

#[test]
fn radii_become_sixteenths_of_the_pixel_pitch() {
    let s = ManualStep::Sharpen {
        radius_um: 100,
        amount_pct: 100,
        threshold_pct: 0,
    };
    assert_eq!(
        s.pixel_step(0.05),
        PixelStep::Sharpen {
            radius: SubPx(32),
            amount_pct: 100,
            threshold_pct: 0
        }
    );
    let c = ManualStep::Contours {
        width_um: 300,
        sensitivity_pct: 50,
    };
    assert_eq!(
        c.pixel_step(0.1),
        PixelStep::Contours {
            sigma: SubPx(24),
            sensitivity_pct: 50
        }
    );
}
