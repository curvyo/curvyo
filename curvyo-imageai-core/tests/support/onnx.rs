//! Tiny ONNX test nets written byte by byte (no Python, no model file in the repository): an
//! identity (the AI denoise stand-in), a nearest-neighbour x4 resize (the Upscale stand-in) and a
//! channel mean (the Remove background stand-in), each with the static shapes of the real
//! models (ADR 0075 §2). Shared by the editor's end-to-end tests through `#[path]`.

#![allow(dead_code, clippy::missing_panics_doc)]

fn varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = u8::try_from(v & 0x7f).unwrap();
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn key(out: &mut Vec<u8>, field: u64, wire: u64) {
    varint(out, (field << 3) | wire);
}

fn int(out: &mut Vec<u8>, field: u64, v: i64) {
    key(out, field, 0);
    varint(out, u64::from_ne_bytes(v.to_ne_bytes()));
}

fn bytes(out: &mut Vec<u8>, field: u64, b: &[u8]) {
    key(out, field, 2);
    varint(out, b.len() as u64);
    out.extend_from_slice(b);
}

fn text(out: &mut Vec<u8>, field: u64, s: &str) {
    bytes(out, field, s.as_bytes());
}

/// A float tensor value info of `shape`.
fn value_info(name: &str, shape: &[i64]) -> Vec<u8> {
    let mut dims = Vec::new();
    for &d in shape {
        let mut dim = Vec::new();
        int(&mut dim, 1, d);
        bytes(&mut dims, 1, &dim);
    }
    let mut tensor = Vec::new();
    int(&mut tensor, 1, 1);
    bytes(&mut tensor, 2, &dims);
    let mut ty = Vec::new();
    bytes(&mut ty, 1, &tensor);
    let mut info = Vec::new();
    text(&mut info, 1, name);
    bytes(&mut info, 2, &ty);
    info
}

enum Attr<'a> {
    Int(i64),
    Ints(&'a [i64]),
    Text(&'a str),
}

fn attribute(name: &str, value: &Attr<'_>) -> Vec<u8> {
    let mut a = Vec::new();
    text(&mut a, 1, name);
    match value {
        Attr::Int(v) => {
            int(&mut a, 20, 2);
            int(&mut a, 3, *v);
        }
        Attr::Ints(vs) => {
            int(&mut a, 20, 7);
            for v in *vs {
                int(&mut a, 8, *v);
            }
        }
        Attr::Text(s) => {
            int(&mut a, 20, 3);
            bytes(&mut a, 4, s.as_bytes());
        }
    }
    a
}

fn node(op: &str, inputs: &[&str], output: &str, attrs: &[(&str, Attr<'_>)]) -> Vec<u8> {
    let mut n = Vec::new();
    for i in inputs {
        text(&mut n, 1, i);
    }
    text(&mut n, 2, output);
    text(&mut n, 3, op);
    text(&mut n, 4, op);
    for (name, value) in attrs {
        bytes(&mut n, 5, &attribute(name, value));
    }
    n
}

fn float_tensor(name: &str, dims: &[i64], values: &[f32]) -> Vec<u8> {
    let mut t = Vec::new();
    for &d in dims {
        int(&mut t, 1, d);
    }
    int(&mut t, 2, 1);
    text(&mut t, 8, name);
    let raw: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    bytes(&mut t, 9, &raw);
    t
}

fn model(nodes: &[Vec<u8>], inits: &[Vec<u8>], input: &[i64], output: &[i64]) -> Vec<u8> {
    let mut graph = Vec::new();
    for n in nodes {
        bytes(&mut graph, 1, n);
    }
    text(&mut graph, 2, "test");
    for i in inits {
        bytes(&mut graph, 5, i);
    }
    bytes(&mut graph, 11, &value_info("x", input));
    bytes(&mut graph, 12, &value_info("y", output));
    let mut opset = Vec::new();
    text(&mut opset, 1, "");
    int(&mut opset, 2, 13);
    let mut m = Vec::new();
    int(&mut m, 1, 8);
    bytes(&mut m, 8, &opset);
    bytes(&mut m, 7, &graph);
    m
}

/// `y = x` on a 1 x 3 x 256 x 256 tile: the AI denoise stand-in.
#[must_use]
pub fn identity_net() -> Vec<u8> {
    let shape = [1, 3, 256, 256];
    model(&[node("Identity", &["x"], "y", &[])], &[], &shape, &shape)
}

/// `y` = `x` resized x4 by the nearest neighbour: the Upscale stand-in.
#[must_use]
pub fn nearest_x4_net() -> Vec<u8> {
    model(
        &[node(
            "Resize",
            &["x", "", "scales"],
            "y",
            &[
                ("mode", Attr::Text("nearest")),
                ("coordinate_transformation_mode", Attr::Text("asymmetric")),
                ("nearest_mode", Attr::Text("floor")),
            ],
        )],
        &[float_tensor("scales", &[4], &[1.0, 1.0, 4.0, 4.0])],
        &[1, 3, 256, 256],
        &[1, 3, 1024, 1024],
    )
}

/// The weights of [`blur_net`]: an uneven 3 x 3 kernel per channel, so each output pixel depends
/// on its neighbours in every direction differently.
const BLUR: [f32; 9] = [0.05, 0.15, 0.1, 0.0, 0.4, 0.05, 0.1, 0.05, 0.1];

fn blur_node(output: &str) -> (Vec<u8>, Vec<u8>) {
    let weights: Vec<f32> = (0..3).flat_map(|_| BLUR).collect();
    (
        node(
            "Conv",
            &["x", "w"],
            output,
            &[
                ("group", Attr::Int(3)),
                ("kernel_shape", Attr::Ints(&[3, 3])),
                ("pads", Attr::Ints(&[1, 1, 1, 1])),
            ],
        ),
        float_tensor("w", &[3, 1, 3, 3], &weights),
    )
}

/// `y` = `x` through an uneven 3 x 3 blur per channel on a 1 x 3 x 256 x 256 tile: an AI
/// denoise stand-in whose output depends on the neighbours, so a misplaced tile shows.
#[must_use]
pub fn blur_net() -> Vec<u8> {
    let (conv, weights) = blur_node("y");
    let shape = [1, 3, 256, 256];
    model(&[conv], &[weights], &shape, &shape)
}

/// `y` = [`blur_net`] then a nearest-neighbour x4 resize: an Upscale stand-in whose output
/// depends on the neighbours.
#[must_use]
pub fn blur_x4_net() -> Vec<u8> {
    let (conv, weights) = blur_node("b");
    model(
        &[
            conv,
            node(
                "Resize",
                &["b", "", "scales"],
                "y",
                &[
                    ("mode", Attr::Text("nearest")),
                    ("coordinate_transformation_mode", Attr::Text("asymmetric")),
                    ("nearest_mode", Attr::Text("floor")),
                ],
            ),
        ],
        &[weights, float_tensor("scales", &[4], &[1.0, 1.0, 4.0, 4.0])],
        &[1, 3, 256, 256],
        &[1, 3, 1024, 1024],
    )
}

/// `y` = the mean of the three channels of a 1 x 3 x 320 x 320 input: the Remove background
/// stand-in (dark subject, light background give a dark mask where the subject is).
#[must_use]
pub fn channel_mean_net() -> Vec<u8> {
    model(
        &[node(
            "ReduceMean",
            &["x"],
            "y",
            &[("axes", Attr::Ints(&[1])), ("keepdims", Attr::Int(1))],
        )],
        &[],
        &[1, 3, 320, 320],
        &[1, 1, 320, 320],
    )
}
