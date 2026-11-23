#!/usr/bin/env python3
"""Export the three AI model files of specs/0135 slice 3 to ONNX (docs/runbooks/ai-models.md).

Usage:
    export_onnx.py WORK_DIR [NAME ...]

WORK_DIR must hold `dl/` with the upstream weights (`u2netp.pth`, `realesr-animevideov3.pth`,
`ffdnet_color.pth`; the rejected `realesr-general-x4v3.pth`, `scunet_color_real_psnr.pth`) and
`src/` with the upstream checkouts (`U-2-Net`, `Real-ESRGAN` at tag v0.2.5.0, `KAIR`, `SCUNet`).
Without NAMEs the three shipped models are exported. Writes `WORK_DIR/out/<name>-<version>/<sha>.onnx`
and `LICENSE.txt`, and prints the check against torch and the operator list.

Pinned environment (CPU wheels): torch 2.9.1+cpu, onnx 1.23.2, onnxruntime 1.32.0,
numpy 2.5.5, einops 0.8.2. `timm` and `thop` are stubbed (inference only, no drop path).
"""
import hashlib
import importlib.util
import shutil
import sys
import types
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
import torch
from torch import nn

OPSET = 17
CONVERTED = "Converted to ONNX by Curvyo, weights unchanged.\n"


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def stub_scunet_imports():
    """SCUNet imports timm (init and DropPath) and thop (FLOP counting); neither matters here."""
    thop = types.ModuleType("thop")
    thop.profile = lambda *a, **k: (0, 0)
    layers = types.ModuleType("timm.models.layers")
    layers.trunc_normal_ = lambda t, *a, **k: t
    layers.DropPath = nn.Identity
    for n, m in (("thop", thop), ("timm", types.ModuleType("timm")),
                 ("timm.models", types.ModuleType("timm.models")), ("timm.models.layers", layers)):
        sys.modules[n] = m


def stub_basicsr():
    """Real-ESRGAN's arch file only needs the registry decorator."""
    class Reg:
        def register(self):
            return lambda cls: cls
    reg = types.ModuleType("basicsr.utils.registry")
    reg.ARCH_REGISTRY = Reg()
    for n, m in (("basicsr", types.ModuleType("basicsr")),
                 ("basicsr.utils", types.ModuleType("basicsr.utils")),
                 ("basicsr.utils.registry", reg)):
        sys.modules[n] = m


class FirstOutput(nn.Module):
    """u2netp returns seven maps; the runbook wants `d0` only."""
    def __init__(self, net):
        super().__init__()
        self.net = net

    def forward(self, x):
        return self.net(x)[0]


def build_u2netp(work):
    mod = load_module("u2net", work / "src/U-2-Net/model/u2net.py")
    net = mod.U2NETP(3, 1)
    net.load_state_dict(torch.load(work / "dl/u2netp.pth", map_location="cpu"), strict=True)
    return FirstOutput(net).eval(), (1, 3, 320, 320), "Apache-2.0", work / "src/U-2-Net"


def build_esrgan(work):
    stub_basicsr()
    mod = load_module("srvgg_arch", work / "src/Real-ESRGAN/realesrgan/archs/srvgg_arch.py")
    net = mod.SRVGGNetCompact(num_in_ch=3, num_out_ch=3, num_feat=64, num_conv=32,
                              upscale=4, act_type="prelu")
    sd = torch.load(work / "dl/realesr-general-x4v3.pth", map_location="cpu")
    net.load_state_dict(sd["params"], strict=True)
    return net.eval(), (1, 3, 256, 256), "BSD-3-Clause", work / "src/Real-ESRGAN"


def build_animevideo(work):
    """Upscale (ADR 0075 note 2026-11-21): the compact x4 net with 16 convolutions."""
    stub_basicsr()
    mod = load_module("srvgg_arch", work / "src/Real-ESRGAN/realesrgan/archs/srvgg_arch.py")
    net = mod.SRVGGNetCompact(num_in_ch=3, num_out_ch=3, num_feat=64, num_conv=16,
                              upscale=4, act_type="prelu")
    sd = torch.load(work / "dl/realesr-animevideov3.pth", map_location="cpu")
    net.load_state_dict(sd["params"], strict=True)
    return net.eval(), (1, 3, 256, 256), "BSD-3-Clause", work / "src/Real-ESRGAN"


class FixedSigma(nn.Module):
    """FFDNet with its noise level fixed as a constant (the step's Strength blends afterwards)."""
    def __init__(self, net, sigma):
        super().__init__()
        self.net = net
        self.register_buffer("sigma", torch.full((1, 1, 1, 1), sigma))

    def forward(self, x):
        return self.net(x, self.sigma)


FFDNET_SIGMA = 15.0 / 255.0


def build_ffdnet(work):
    sys.path.insert(0, str(work / "src/KAIR"))
    from models.network_ffdnet import FFDNet
    net = FFDNet(in_nc=3, out_nc=3, nc=96, nb=12, act_mode="R")
    net.load_state_dict(torch.load(work / "dl/ffdnet_color.pth", map_location="cpu"), strict=True)
    return FixedSigma(net.eval(), FFDNET_SIGMA).eval(), (1, 3, 256, 256), "MIT", work / "src/KAIR"


def build_scunet(work):
    stub_scunet_imports()
    sys.path.insert(0, str(work / "src/SCUNet"))
    from models.network_scunet import SCUNet
    net = SCUNet(in_nc=3, config=[4, 4, 4, 4, 4, 4, 4], dim=64)
    net.load_state_dict(torch.load(work / "dl/scunet_color_real_psnr.pth", map_location="cpu"),
                        strict=True)
    return net.eval(), (1, 3, 256, 256), "Apache-2.0", work / "src/SCUNet"


MODELS = [
    ("u2netp", "1", build_u2netp),
    ("realesr-animevideov3", "1", build_animevideo),
    ("ffdnet-color", "1", build_ffdnet),
]

# Measured and rejected for the browser (ADR 0075 note 2026-11-21: 143 s and 470 s on 1 Mpx).
REJECTED = [
    ("realesr-general-x4v3", "1", build_esrgan),
    ("scunet-color-real-psnr", "1", build_scunet),
]


def export(work, name, version, build):
    net, shape, _licence, src = build(work)
    tmp = work / f"{name}.tmp.onnx"
    x = torch.rand(*shape, generator=torch.Generator().manual_seed(1))
    torch.onnx.export(net, (x,), str(tmp), opset_version=OPSET, dynamo=False,
                      input_names=["input"], output_names=["output"],
                      do_constant_folding=True, external_data=False)
    model = onnx.load(str(tmp))
    onnx.checker.check_model(model)
    assert not any(t.data_location == onnx.TensorProto.EXTERNAL for t in model.graph.initializer)
    assert {o.domain for o in model.opset_import} <= {"", "ai.onnx"}, "custom operator domain"
    ops = sorted({n.op_type for n in model.graph.node})

    with torch.no_grad():
        ref = net(x).numpy()
    got = ort.InferenceSession(str(tmp), providers=["CPUExecutionProvider"]).run(
        None, {"input": x.numpy()})[0]
    diff = float(np.abs(ref - got).max())

    sha = hashlib.sha256(tmp.read_bytes()).hexdigest()
    out = work / "out" / f"{name}-{version}"
    out.mkdir(parents=True, exist_ok=True)
    final = out / f"{sha}.onnx"
    shutil.move(tmp, final)
    licence = (src / "LICENSE").read_text()
    notice = src / "NOTICE"
    if notice.exists():
        licence += "\n" + notice.read_text()
    (out / "LICENSE.txt").write_text(licence.rstrip("\n") + "\n\n" + CONVERTED)
    print(f"{name}-{version}: {final.stat().st_size} bytes sha256={sha} "
          f"out_shape={tuple(got.shape)} max_abs_diff={diff:.3e}\n  ops: {' '.join(ops)}")


def main():
    work = Path(sys.argv[1]).resolve()
    torch.set_grad_enabled(False)
    names = sys.argv[2:]
    for name, version, build in MODELS + REJECTED:
        if (not names and (name, version, build) in MODELS) or name in names:
            export(work, name, version, build)


if __name__ == "__main__":
    main()
