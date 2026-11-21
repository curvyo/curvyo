//! `specs/0131` criteria 1 and 3: the offer of the controller's bed size, as a table.

#![allow(clippy::float_cmp)]

use curvyo_document_core::DocumentSize;
use curvyo_laser_core::machine_size_offer;

fn offer(page: [f64; 2], reported: [f64; 2]) -> Option<[f64; 2]> {
    machine_size_offer(DocumentSize::from_mm(page[0], page[1]), reported)
        .map(|size| [size.width.as_mm(), size.height.as_mm()])
}

#[test]
fn the_table() {
    let page = [400.0, 300.0];
    let cases: [([f64; 2], Option<[f64; 2]>); 16] = [
        ([400.0, 300.0], None),
        ([400.01, 300.0], None),
        ([400.0, 300.01], None),
        ([400.011, 300.0], Some([400.011, 300.0])),
        ([400.0, 300.011], Some([400.0, 300.011])),
        ([400.0, 850.0], Some([400.0, 850.0])),
        ([500.0, 300.0], Some([500.0, 300.0])),
        ([1.0, 1.0], Some([1.0, 1.0])),
        ([10_000.0, 10_000.0], Some([10_000.0, 10_000.0])),
        ([0.99, 300.0], None),
        ([400.0, 10_000.01], None),
        ([0.0, 300.0], None),
        ([-400.0, 850.0], None),
        ([f64::NAN, 850.0], None),
        ([400.0, f64::INFINITY], None),
        ([f64::NEG_INFINITY, f64::NAN], None),
    ];
    for (reported, expected) in cases {
        assert_eq!(offer(page, reported), expected, "{reported:?}");
    }
}
