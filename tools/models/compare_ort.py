"""Compares the outputs `curvyo-imageai-core/tests/real_models.rs` wrote with ONNX Runtime on the
same inputs (ADR 0075 §7, step 0). Usage: compare_ort.py <models dir> <outputs dir> [upscale
model] [denoise model] (default the shipped `realesr-animevideov3` and `ffdnet-color`).
Prints the largest difference per model in grey levels (0 to 255)."""
import pathlib
import sys

import numpy as np
import onnxruntime as ort


def model(models, name):
    return next((pathlib.Path(models) / f"{name}-1").glob("*.onnx"))


def rgba(out, name, side):
    return np.fromfile(pathlib.Path(out) / name, dtype=np.uint8).reshape(side, side, 4)


def tile_input(img, pad=16, side=256):
    """The 224 px picture with its edge repeated 16 px around it, RGB 0 to 1, NCHW."""
    rgb = img[..., :3].astype(np.float32) / 255.0
    padded = np.pad(rgb, ((pad, side - img.shape[0] - pad), (pad, side - img.shape[1] - pad), (0, 0)), mode="edge")
    return padded.transpose(2, 0, 1)[None]


def to_u8(x):
    return np.clip(np.round(x * 255.0), 0, 255).astype(np.int32)


def main(models, out, up_name="realesr-animevideov3", dn_name="ffdnet-color"):
    mean = np.array([0.485, 0.456, 0.406], np.float32)[:, None, None]
    dev = np.array([0.229, 0.224, 0.225], np.float32)[:, None, None]
    src = rgba(out, "mask_in.rgba", 320)[..., :3].astype(np.float32) / 255.0
    x = ((src.transpose(2, 0, 1) - mean) / dev)[None]
    y = ort.InferenceSession(str(model(models, "u2netp"))).run(None, {"input": x})[0][0, 0]
    y = (y - y.min()) / max(y.max() - y.min(), 1e-12)
    ours = np.fromfile(pathlib.Path(out) / "mask_out.grey", dtype=np.uint8).reshape(320, 320).astype(np.int32)
    print("u2netp max diff", int(np.abs(to_u8(y) - ours).max()))
    for name, key, scale in ((up_name, "up", 4), (dn_name, "dn", 1)):
        img = rgba(out, f"{key}_in.rgba", 224)
        y = ort.InferenceSession(str(model(models, name))).run(None, {"input": tile_input(img)})[0][0]
        core = y[:, 16 * scale:(16 + 224) * scale, 16 * scale:(16 + 224) * scale].transpose(1, 2, 0)
        ours = rgba(out, f"{key}_out.rgba", 224 * scale)[..., :3].astype(np.int32)
        d = np.abs(to_u8(core) - ours)
        print(name, "max diff", int(d.max()), "mean", float(d.mean()))


if __name__ == "__main__":
    main(*sys.argv[1:5])
