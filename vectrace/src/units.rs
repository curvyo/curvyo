//! The units of the tracer: millimetres, radians and the map from pixels to millimetres.
//!
//! The arithmetic of [`PixelMap`] is the arithmetic of `curvyo-document-core`'s `Affine`, entry
//! for entry with `mul_add` unchanged, so a trace gives the same last bit as before (ADR 0048 §3);
//! a test pins the bits.

use kurbo::{Point, Vec2};

/// Below this absolute determinant a map has no inverse.
const SINGULAR_DETERMINANT: f64 = 1e-9;

/// A length in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mm(f64);

impl Mm {
    /// A length from a value already in millimetres.
    #[must_use]
    pub const fn from_mm(millimetres: f64) -> Self {
        Self(millimetres)
    }

    /// The length as a plain millimetre value.
    #[must_use]
    pub const fn as_mm(self) -> f64 {
        self.0
    }
}

/// An angle in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(f64);

impl Angle {
    /// An angle from a value already in radians.
    #[must_use]
    pub const fn from_radians(radians: f64) -> Self {
        Self(radians)
    }

    /// The angle as a plain radian value.
    #[must_use]
    pub const fn as_radians(self) -> f64 {
        self.0
    }
}

/// The map from pixel corners to millimetres: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`, held as
/// the list `[a, b, c, d, e, f]` in SVG order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelMap([f64; 6]);

#[allow(clippy::many_single_char_names)] // the SVG names of the six entries
impl PixelMap {
    /// The map with these six entries, in SVG order.
    #[must_use]
    pub const fn new(entries: [f64; 6]) -> Self {
        Self(entries)
    }

    /// The map that changes nothing.
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// The six entries in SVG order.
    #[must_use]
    pub const fn to_array(self) -> [f64; 6] {
        self.0
    }

    /// A scale by `(sx, sy)` about the origin.
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    /// A pure translation by `offset`.
    #[must_use]
    pub const fn translation(offset: Vec2) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, offset.x, offset.y])
    }

    /// `self · then`: the map that applies `then` first and `self` after it.
    #[must_use]
    pub fn compose(self, then: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let t = then.0;
        Self([
            a.mul_add(t[0], c * t[1]),
            b.mul_add(t[0], d * t[1]),
            a.mul_add(t[2], c * t[3]),
            b.mul_add(t[2], d * t[3]),
            a.mul_add(t[4], c.mul_add(t[5], e)),
            b.mul_add(t[4], d.mul_add(t[5], f)),
        ])
    }

    /// `p` mapped by the whole map.
    #[must_use]
    pub fn apply_point(self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.0;
        Point::new(
            a.mul_add(p.x, c.mul_add(p.y, e)),
            b.mul_add(p.x, d.mul_add(p.y, f)),
        )
    }

    /// `v` mapped by the linear part only.
    #[must_use]
    pub fn apply_vector(self, v: Vec2) -> Vec2 {
        let [a, b, c, d, ..] = self.0;
        Vec2::new(a.mul_add(v.x, c * v.y), b.mul_add(v.x, d * v.y))
    }

    /// The determinant of the linear part.
    #[must_use]
    pub fn determinant(self) -> f64 {
        let [a, b, c, d, ..] = self.0;
        a.mul_add(d, -(c * b))
    }

    /// The inverse map, or `None` when the absolute determinant is below `1e-9` or not finite.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if !det.is_finite() || det.abs() < SINGULAR_DETERMINANT {
            return None;
        }
        let [sa, sb, sc, sd, se, sf] = self.0;
        let (a, b, c, d) = (sd / det, -sb / det, -sc / det, sa / det);
        Some(Self([
            a,
            b,
            c,
            d,
            -a.mul_add(se, c * sf),
            -b.mul_add(se, d * sf),
        ]))
    }

    /// Whether every entry is a finite number.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.0.iter().all(|v| v.is_finite())
    }

    /// The same map as a `kurbo` affine, to map outlines.
    #[must_use]
    pub fn to_kurbo(self) -> kurbo::Affine {
        kurbo::Affine::new(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAPS: [[f64; 6]; 5] = [
        [0.1, 0.0, 0.0, 0.1, 5.0, 7.0],
        [
            0.264_583_333_333_333_3,
            0.0,
            0.0,
            -0.264_583_333_333_333_3,
            12.345_678_9,
            99.9,
        ],
        [0.7, 0.3, -0.2, 1.3, 0.1, -0.3],
        [1e-5, 3e-6, -7e-6, 2e-5, 1e3, -1e3],
        [1.0, 2.0, 2.0, 4.0, 0.0, 0.0],
    ];

    // Frozen once from `curvyo-document-core`'s `Affine` (ADR 0048 §3): the bits of the results
    // for `MAPS`, so the arithmetic stays the same last bit without that crate.
    const COMPOSED: [[[u64; 6]; 5]; 5] = [
        [
            [
                0x3f84_7ae1_47ae_147c,
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0000,
                0x3f84_7ae1_47ae_147c,
                0x4016_0000_0000_0000,
                0x401e_cccc_cccc_cccd,
            ],
            [
                0x3f9b_17e4_b17e_4b17,
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0000,
                0xbf9b_17e4_b17e_4b17,
                0x4018_f032_90a0_f787,
                0x4030_fd70_a3d7_0a3e,
            ],
            [
                0x3fb1_eb85_1eb8_51eb,
                0x3f9e_b851_eb85_1eb8,
                0xbf94_7ae1_47ae_147c,
                0x3fc0_a3d7_0a3d_70a4,
                0x4014_0a3d_70a3_d70a,
                0x401b_e147_ae14_7ae1,
            ],
            [
                0x3eb0_c6f7_a0b5_ed8e,
                0x3e94_21f5_f40d_8377,
                0xbea7_7cf4_4765_1960,
                0x3ec0_c6f7_a0b5_ed8e,
                0x405a_4000_0000_0000,
                0xc057_4000_0000_0000,
            ],
            [
                0x3fb9_9999_9999_999a,
                0x3fc9_9999_9999_999a,
                0x3fc9_9999_9999_999a,
                0x3fd9_9999_9999_999a,
                0x4014_0000_0000_0000,
                0x401c_0000_0000_0000,
            ],
        ],
        [
            [
                0x3f9b_17e4_b17e_4b17,
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0000,
                0xbf9b_17e4_b17e_4b17,
                0x402b_5652_287a_2af7,
                0x4058_8311_1111_1111,
            ],
            [
                0x3fb1_ebcd_f012_3454,
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0000,
                0x3fb1_ebcd_f012_3454,
                0x402f_396a_6279_c7c1,
                0x4052_5df5_c28f_5c2a,
            ],
            [
                0x3fc7_b4e8_1b4e_81b3,
                0xbfb4_51eb_851e_b851,
                0xbfab_17e4_b17e_4b17,
                0xbfd6_0369_d036_9d02,
                0x4028_be88_c57d_94c8,
                0x4058_feae_147a_e148,
            ],
            [
                0x3ec6_31e2_41f0_aced,
                0xbeaa_a242_b587_35e8,
                0xbebf_12a3_291d_bee4,
                0xbed6_31e2_41f0_aced,
                0x4071_4edd_3bee_7c01,
                0x4076_c7bb_bbbb_bbbb,
            ],
            [
                0x3fd0_eeee_eeee_eeee,
                0xbfe0_eeee_eeee_eeee,
                0x3fe0_eeee_eeee_eeee,
                0xbff0_eeee_eeee_eeee,
                0x4028_b0fc_d324_d5a2,
                0x4058_f999_9999_999a,
            ],
        ],
        [
            [
                0x3fb1_eb85_1eb8_51eb,
                0x3f9e_b851_eb85_1eb8,
                0xbf94_7ae1_47ae_147c,
                0x3fc0_a3d7_0a3d_70a4,
                0x4001_9999_9999_9999,
                0x4024_9999_9999_999a,
            ],
            [
                0x3fc7_b4e8_1b4e_81b3,
                0x3fb4_51eb_851e_b851,
                0x3fab_17e4_b17e_4b17,
                0xbfd6_0369_d036_9d02,
                0xc026_79de_61f5_936d,
                0x4060_a8c2_2e32_ee40,
            ],
            [
                0x3fdb_851e_b851_eb84,
                0x3fe3_3333_3333_3333,
                0xbfd9_9999_9999_999a,
                0x3ffa_147a_e147_ae15,
                0x3fcd_70a3_d70a_3d71,
                0xbfe5_1eb8_51eb_851e,
            ],
            [
                0x3eda_d7f2_9abc_af48,
                0x3edc_f0d1_8ed3_6cfa,
                0xbee2_aa26_af97_31e0,
                0x3ef9_0f9b_7e76_27a5,
                0x408c_20cc_cccc_cccd,
                0xc08f_4266_6666_6666,
            ],
            [
                0x3fd3_3333_3333_3332,
                0x4007_3333_3333_3333,
                0x3fe3_3333_3333_3332,
                0x4017_3333_3333_3333,
                0x3fb9_9999_9999_999a,
                0xbfd3_3333_3333_3333,
            ],
        ],
        [
            [
                0x3eb0_c6f7_a0b5_ed8e,
                0x3e94_21f5_f40d_8377,
                0xbea7_7cf4_4765_1960,
                0x3ec0_c6f7_a0b5_ed8e,
                0x408f_4000_0086_37bd,
                0xc08f_3fff_aebc_408e,
            ],
            [
                0x3ec6_31e2_41f0_aced,
                0x3eaa_a242_b587_35e8,
                0x3ebf_12a3_291d_bee4,
                0xbed6_31e2_41f0_aced,
                0x408f_3ffe_d217_a1ee,
                0xc08f_3ffb_d50d_f3da,
            ],
            [
                0x3ed4_8d55_be78_7634,
                0x3ee0_fca7_85eb_66ec,
                0xbee7_4744_622f_a001,
                0x3efa_a242_b587_35ea,
                0x408f_4000_01a0_1330,
                0xc08f_4000_02fd_0a82,
            ],
            [
                0x3dd5_b721_7b74_acee,
                0x3dd8_bd2f_dda8_9129,
                0xbdec_dcb7_d7ef_5405,
                0x3dfa_0b70_423e_f989,
                0x408f_4022_d0e5_6041,
                0xc08f_4022_d0e5_6041,
            ],
            [
                0xbed0_c6f7_a0b5_ed8c,
                0x3f06_8b5c_bff4_7736,
                0xbee0_c6f7_a0b5_ed8c,
                0x3f16_8b5c_bff4_7736,
                0x408f_4000_0000_0000,
                0xc08f_4000_0000_0000,
            ],
        ],
        [
            [
                0x3fb9_9999_9999_999a,
                0x3fc9_9999_9999_999a,
                0x3fc9_9999_9999_999a,
                0x3fd9_9999_9999_999a,
                0x4033_0000_0000_0000,
                0x4043_0000_0000_0000,
            ],
            [
                0x3fd0_eeee_eeee_eeee,
                0x3fe0_eeee_eeee_eeee,
                0xbfe0_eeee_eeee_eeee,
                0xbff0_eeee_eeee_eeee,
                0x406a_84a9_66cb_e6f4,
                0x407a_84a9_66cb_e6f4,
            ],
            [
                0x3ff4_cccc_cccc_cccc,
                0x4004_cccc_cccc_cccc,
                0x4003_3333_3333_3333,
                0x4013_3333_3333_3333,
                0xbfe0_0000_0000_0000,
                0xbff0_0000_0000_0000,
            ],
            [
                0x3ef0_c6f7_a0b5_ed8e,
                0x3f00_c6f7_a0b5_ed8e,
                0x3f01_4d2f_5dbb_9cfa,
                0x3f11_4d2f_5dbb_9cfa,
                0xc08f_4000_0000_0000,
                0xc09f_4000_0000_0000,
            ],
            [
                0x4014_0000_0000_0000,
                0x4024_0000_0000_0000,
                0x4024_0000_0000_0000,
                0x4034_0000_0000_0000,
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0000,
            ],
        ],
    ];
    const DETERMINANTS: [u64; 5] = [
        0x3f84_7ae1_47ae_147c,
        0xbfb1_ebcd_f012_3454,
        0x3fef_0a3d_70a3_d70a,
        0x3dee_5fbf_0909_4623,
        0x0000_0000_0000_0000,
    ];
    const INVERSES: [Option<[u64; 6]>; 5] = [
        Some([
            0x4023_ffff_ffff_ffff,
            0x8000_0000_0000_0000,
            0x8000_0000_0000_0000,
            0x4023_ffff_ffff_ffff,
            0xc048_ffff_ffff_ffff,
            0xc051_7fff_ffff_ffff,
        ]),
        Some([
            0x400e_3c78_f1e3_c792,
            0x0000_0000_0000_0000,
            0x0000_0000_0000_0000,
            0xc00e_3c78_f1e3_c792,
            0xc047_5496_3257_36b8,
            0x4077_9932_64c9_9329,
        ]),
        Some([
            0x3ff5_717c_0a8e_83f6,
            0xbfd3_cb37_6c34_c894,
            0x3fca_6449_e59b_b61b,
            0x3fe7_17c0_a8e8_3f57,
            0xbfb2_7966_ed86_9914,
            0x3fcf_ab8b_e054_741f,
        ]),
        None,
        None,
    ];
    const POINTS: [[[u64; 2]; 4]; 5] = [
        [
            [0x4014_0000_0000_0000, 0x401c_0000_0000_0000],
            [0x4014_6666_6666_6666, 0x401c_6666_6666_6666],
            [0x4031_5879_3dd9_7f63, 0xbfec_7ae1_47ae_1484],
            [0x4014_0000_00ab_cc77, 0x401d_51eb_851e_b852],
        ],
        [
            [0x4028_b0fc_d324_d5a2, 0x4058_f999_9999_999a],
            [0x4029_3874_4a9c_4d19, 0x4058_e8aa_aaaa_aaab],
            [0x4046_814a_43f0_f098, 0x405e_31a3_d70a_3d71],
            [0x4028_b0fc_d408_1c20, 0x4058_c1b8_51eb_851f],
        ],
        [
            [0x3fb9_9999_9999_999a, 0xbfd3_3333_3333_3333],
            [0x3fe3_3333_3333_3333, 0x3ff4_cccc_cccc_cccd],
            [0x4059_9326_17c1_bda5, 0xc050_7553_2617_c1be],
            [0xbfe1_eb84_f923_97df, 0x400f_eb85_22bf_1cb5],
        ],
        [
            [0x408f_4000_0000_0000, 0xc08f_4000_0000_0000],
            [0x408f_4000_0192_a737, 0xc08f_3fff_f3f0_fe05],
            [0x408f_4003_a8d4_4a19, 0xc08f_4002_7925_9f91],
            [0x408f_3fff_f3e3_9214, 0xc08f_3fff_dd65_a142],
        ],
        [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x4008_0000_0000_0000, 0x4018_0000_0000_0000],
            [0xc041_2c08_3126_e97a, 0xc051_2c08_3126_e97a],
            [0x401a_6666_6d1c_630d, 0x402a_6666_6d1c_630d],
        ],
    ];
    const VECTORS: [[[u64; 2]; 4]; 5] = [
        [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x3fb9_9999_9999_999a, 0x3fb9_9999_9999_999a],
            [0x4028_b0f2_7bb2_fec6, 0xc01f_8f5c_28f5_c290],
            [0x3e45_798e_e230_8c3a, 0x3fd5_1eb8_51eb_851f],
        ],
        [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x3fd0_eeee_eeee_eeee, 0xbfd0_eeee_eeee_eeee],
            [0x4040_550b_0f27_bb2f, 0x4034_e028_f5c2_8f5b],
            [0x3e5c_68cf_b090_3980, 0xbfeb_f0a3_d70a_3d6f],
        ],
        [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x3fdf_ffff_ffff_ffff, 0x3ff9_9999_9999_999a],
            [0x4059_8cbf_b15b_573f, 0xc050_621f_f2e4_8e8b],
            [0xbfe5_1eb8_2c56_cb13, 0x4011_28f5_c492_c18e],
        ],
        [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x3ec9_2a73_7110_e456, 0x3ef8_1e03_f705_857c],
            [0x3f5d_46a2_50cd_4cfb, 0xbf53_c92c_fc84_c2c0],
            [0xbef8_38db_d808_a8a9, 0x3f11_4d2f_5f0d_6217],
        ],
        [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x4008_0000_0000_0000, 0x4018_0000_0000_0000],
            [0xc041_2c08_3126_e97a, 0xc051_2c08_3126_e97a],
            [0x401a_6666_6d1c_630d, 0x402a_6666_6d1c_630d],
        ],
    ];

    fn bits(a: [f64; 6]) -> [u64; 6] {
        a.map(f64::to_bits)
    }

    #[test]
    fn matches_the_frozen_document_core_bits() {
        let pts = [(0.0, 0.0), (1.0, 1.0), (123.456, -78.9), (1e-7, 3.3)];
        for (i, m) in MAPS.into_iter().enumerate() {
            for (j, n) in MAPS.into_iter().enumerate() {
                let ours = PixelMap::new(m).compose(PixelMap::new(n)).to_array();
                assert_eq!(bits(ours), COMPOSED[i][j], "compose {i} {j}");
            }
            let pm = PixelMap::new(m);
            assert_eq!(pm.determinant().to_bits(), DETERMINANTS[i]);
            assert!(pm.is_finite());
            assert_eq!(pm.inverse().map(|inv| bits(inv.to_array())), INVERSES[i]);
            for (k, (x, y)) in pts.into_iter().enumerate() {
                let o = pm.apply_point(Point::new(x, y));
                assert_eq!(
                    [o.x.to_bits(), o.y.to_bits()],
                    POINTS[i][k],
                    "point {i} {k}"
                );
                let o = pm.apply_vector(Vec2::new(x, y));
                assert_eq!(
                    [o.x.to_bits(), o.y.to_bits()],
                    VECTORS[i][k],
                    "vector {i} {k}"
                );
            }
        }
    }

    #[test]
    fn scale_and_translation_are_the_plain_matrices() {
        assert_eq!(
            PixelMap::scale(0.5, 0.25).to_array(),
            [0.5, 0.0, 0.0, 0.25, 0.0, 0.0]
        );
        assert_eq!(
            PixelMap::translation(Vec2::new(3.0, -4.0)).to_array(),
            [1.0, 0.0, 0.0, 1.0, 3.0, -4.0]
        );
    }

    #[test]
    fn a_singular_or_non_finite_map_has_no_inverse() {
        assert!(
            PixelMap::new([1.0, 2.0, 2.0, 4.0, 0.0, 0.0])
                .inverse()
                .is_none()
        );
        assert!(
            PixelMap::new([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0])
                .inverse()
                .is_none()
        );
        assert!(!PixelMap::new([f64::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0]).is_finite());
    }

    #[test]
    fn units_hold_their_value() {
        assert!((Mm::from_mm(2.5).as_mm() - 2.5).abs() < f64::EPSILON);
        assert!((Angle::from_radians(1.5).as_radians() - 1.5).abs() < f64::EPSILON);
    }
}
