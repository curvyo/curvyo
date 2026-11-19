#!/usr/bin/env python3 -I
"""Measures the native Curvyo app (Tauri on WebKitGTK) under Xvfb: process CPU and RSS from /proc
every second, joined with the frame counters of the page probe (`frontend/src/lib/perfProbe.ts`).
Standard library only. See README.md. Usage: tools/perf/measure.py [--no-build] [--out DIR]
"""
import argparse, asyncio, base64, hashlib, json, os, signal, struct, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PORT = 47126
TICKS = os.sysconf("SC_CLK_TCK")
PAGE = os.sysconf("SC_PAGE_SIZE")


def build():
    # `vite build` alone bundles whatever wasm `frontend/src/wasm-bindings` holds: bindings older
    # than the page leave the render loop dead (zero counters in every scenario).
    subprocess.run(["npm", "run", "build:wasm"], cwd=ROOT / "frontend", check=True)
    env = dict(os.environ, VITE_CURVYO_PERF=f"ws://127.0.0.1:{PORT}")
    subprocess.run(["npx", "vite", "build", "--outDir", "dist-perf", "--emptyOutDir"], cwd=ROOT / "frontend", env=env, check=True)
    env = dict(os.environ, TAURI_CONFIG=json.dumps({"build": {"frontendDist": "../frontend/dist-perf"}}))
    subprocess.run(["cargo", "build", "-p", "curvyo-app"], cwd=ROOT, env=env, check=True)


def descendants(root_pid):
    """Pids of root_pid and all its descendants (the WebKit processes are its children)."""
    children = {}
    for entry in os.listdir("/proc"):
        if entry.isdigit():
            try:
                stat = Path(f"/proc/{entry}/stat").read_text()
                children.setdefault(int(stat[stat.rindex(")") + 2:].split()[1]), []).append(int(entry))
            except OSError:
                pass
    found, todo = [], [root_pid]
    while todo:
        pid = todo.pop()
        found.append(pid)
        todo += children.get(pid, [])
    return found


def sample(root_pid):
    """(cpu seconds, rss MiB, {process name: cpu seconds}) summed over the app's process tree."""
    cpu, rss, by_name = 0.0, 0.0, {}
    for pid in descendants(root_pid):
        try:
            stat = Path(f"/proc/{pid}/stat").read_text()
            name = stat[stat.index("(") + 1:stat.rindex(")")]
            fields = stat[stat.rindex(")") + 2:].split()
            seconds = (int(fields[11]) + int(fields[12])) / TICKS
            cpu += seconds
            by_name[name] = by_name.get(name, 0.0) + seconds
            rss += int(Path(f"/proc/{pid}/statm").read_text().split()[1]) * PAGE / 2**20
        except OSError:
            pass
    return cpu, rss, by_name


async def read_frame(reader):
    head = await reader.readexactly(2)
    opcode, length = head[0] & 15, head[1] & 127
    if length == 126:
        length = struct.unpack(">H", await reader.readexactly(2))[0]
    elif length == 127:
        length = struct.unpack(">Q", await reader.readexactly(8))[0]
    mask = await reader.readexactly(4) if head[1] & 128 else b"\0\0\0\0"
    data = bytearray(await reader.readexactly(length))
    for i in range(length):
        data[i] ^= mask[i % 4]
    return opcode, bytes(data)


async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-build", action="store_true", help="reuse the last perf build")
    parser.add_argument("--out", default=str(ROOT / "target" / "perf"))
    # Software rendering (Xvfb) is slow: wheel-zoom alone can take about a minute of wall time.
    parser.add_argument("--timeout", type=int, default=600)
    # On the real display (a GPU, the compositor's own pacing) instead of Xvfb's software GL; no
    # screenshots, since the window is wherever the window manager puts it.
    parser.add_argument("--desktop", action="store_true", help="run on the current DISPLAY instead of Xvfb")
    args = parser.parse_args()
    if not args.no_build:
        build()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    events, procs, state = [], [], {"pid": None, "done": asyncio.Event(), "env": None}
    log = open(out / "events.ndjson", "w")

    async def on_client(reader, writer):
        request = (await reader.readuntil(b"\r\n\r\n")).decode()
        key = next(l.split(": ")[1] for l in request.split("\r\n") if l.lower().startswith("sec-websocket-key"))
        accept = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
        writer.write(f"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n".encode())
        await writer.drain()
        try:
            while True:
                opcode, data = await read_frame(reader)
                if opcode == 8:
                    break
                if opcode == 1:
                    event = json.loads(data)
                    event["wall"] = time.time()
                    events.append(event)
                    log.write(json.dumps(event) + "\n")
                    log.flush()
                    if event["event"] == "done":
                        state["done"].set()
                    if event["event"] == "scenario" and state["env"]:
                        await screenshot(state["env"], out / f"{event['name']}-{event['state']}.png", event.get("canvas"))
                    if event["event"] == "snap" and state["env"]:
                        snaps = state.setdefault("snaps", {})
                        number = snaps[event["scenario"]] = snaps.get(event["scenario"], 0) + 1
                        event["file"] = f"snap-{event['scenario']}-{number}.png"
                        await screenshot(state["env"], out / event["file"], event.get("canvas"))
        except (asyncio.IncompleteReadError, ConnectionError):
            pass

    server = await asyncio.start_server(on_client, "127.0.0.1", PORT)
    app = ROOT / "target" / "debug" / "curvyo-app"
    # xvfb-run keeps Xvfb alive for the app's lifetime; the app is its direct child.
    command = [str(app)] if args.desktop else ["xvfb-run", "-a", "-s", "-screen 0 1600x1000x24", str(app)]
    xvfb = subprocess.Popen(command, cwd=ROOT, start_new_session=True,
                            env=os.environ if args.desktop else dict(os.environ, GDK_BACKEND="x11", WEBKIT_DISABLE_COMPOSITING_MODE=os.environ.get("WEBKIT_DISABLE_COMPOSITING_MODE", "0")))
    t0 = time.time()
    last = None
    try:
        while not state["done"].is_set() and time.time() - t0 < args.timeout:
            if xvfb.poll() is not None:
                print("app exited early", file=sys.stderr)
                break
            tree = next((p for p in descendants(xvfb.pid) if "curvyo-app" in Path(f"/proc/{p}/cmdline").read_text().replace("\0", " ").split(" ")[0]), None)
            if tree and not state["env"] and not args.desktop:
                state["env"] = display_env(tree)
            if tree:
                cpu, rss, names = sample(tree)
                procs.append({"wall": time.time(), "cpu": cpu, "rss": rss, "names": names})
            await asyncio.sleep(1)
    finally:
        os.killpg(xvfb.pid, signal.SIGTERM)
        server.close()
    json.dump(procs, open(out / "procs.json", "w"))
    if not state["done"].is_set():
        print(f"timed out before scenario {unfinished(events)} (--timeout {args.timeout} s)")
    report(events, procs, out)
    if args.desktop:
        print("lag: not measured in desktop mode (no screenshots)")


SCENARIOS = ["idle-empty", "empty-pan", "empty-zoom", "idle-5000", "wheel-zoom", "pan-drag"]


def display_env(pid):
    """DISPLAY and XAUTHORITY of the app under xvfb-run, None while it has none."""
    try:
        pairs = Path(f"/proc/{pid}/environ").read_bytes().split(b"\0")
    except OSError:
        return None
    env = dict(p.decode(errors="replace").split("=", 1) for p in pairs if b"=" in p)
    if "DISPLAY" not in env:
        return None
    return {k: env[k] for k in ("DISPLAY", "XAUTHORITY") if k in env}


async def screenshot(env, path, canvas):
    """The canvas on the Xvfb screen as PNG (ImageMagick `import`, cropped to `canvas` = left, top,
    width, height in device pixels; the window sits at 0,0 without a window manager); skipped
    quietly when the tool is missing."""
    crop = ["-crop", "{2}x{3}+{0}+{1}".format(*canvas), "+repage"] if canvas else []
    try:
        process = await asyncio.create_subprocess_exec("import", "-window", "root", *crop, str(path), env=dict(os.environ, **env),
                                                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        await process.wait()
    except OSError:
        pass


def pixels(path):
    """ImageMagick's signature of the pixels of `path`, None without the file or the tool."""
    try:
        return subprocess.run(["identify", "-format", "%#", str(path)], capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None


def quantile(values, q):
    """The `q` quantile of `values` (nearest rank), 0 for none."""
    if not values:
        return 0.0
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(q * len(ordered)))]


def cadence(rows, n):
    """Frame pacing of a scenario: the animation-frame interval histogram, stalls, React renders per
    second, the main-thread ms of each timed section and the input-to-draw latency."""
    intervals = [i for r in rows for i in r.get("intervals", [])]
    if intervals:
        over = lambda ms: sum(1 for i in intervals if i > ms)
        print(f"  rAF interval ms ({len(intervals)}): p50 {quantile(intervals, 0.5):.1f}, p95 {quantile(intervals, 0.95):.1f}, "
              f"p99 {quantile(intervals, 0.99):.1f}, max {max(intervals):.1f}; > 20 ms {over(20)}, > 33 ms {over(33)}, > 50 ms {over(50)}")
    total = lambda k: sum(r.get(k, 0) for r in rows)
    print(f"  stalls: long tasks {total('longTasks')} ({total('longMs'):.0f} ms), timer gaps > 50 ms {total('gaps')} ({total('gapMs'):.0f} ms)")
    renders = {}
    spans = {}
    for r in rows:
        for k, v in r.get("renders", {}).items():
            renders[k] = renders.get(k, 0) + v
        for k, (count, ms, worst) in r.get("spans", {}).items():
            c, m, w = spans.get(k, (0, 0.0, 0.0))
            spans[k] = (c + count, m + ms, max(w, worst))
    if renders:
        print("  React renders/s: " + ", ".join(f"{k} {v / n:.1f}" for k, v in sorted(renders.items())))
    for k, (count, ms, worst) in sorted(spans.items(), key=lambda kv: -kv[1][1]):
        print(f"    {k:<28} {count / n:6.1f}/s  avg {ms / max(count, 1):6.3f} ms  max {worst:6.2f} ms  {ms / n:6.2f} ms/s")
    if total("pagesInFrame") + total("pagesLater"):
        print(f"  page-frame commits: {total('pagesInFrame')} in the animation frame that read them, {total('pagesLater')} in a later task (painted a frame after it)")
    inputs, waits = total("inputs"), [w for r in rows for w in r.get("inputToDraw", [])]
    if inputs:
        print(f"  view inputs {inputs / n:.1f}/s, drawn frames with inputs {len(waits) / n:.1f}/s ({inputs / max(len(waits), 1):.2f} inputs per frame); "
              f"input to render() ms p50 {quantile(waits, 0.5):.1f}, p95 {quantile(waits, 0.95):.1f}, max {max(waits, default=0):.1f}")


def ppm(path):
    """(width, height, rgb bytes) of an image, through ImageMagick; None without the tool."""
    try:
        data = subprocess.run(["convert", str(path), "ppm:-"], capture_output=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    parts = data.split(maxsplit=4)
    return int(parts[1]), int(parts[2]), parts[4]


def page_edges(image):
    """On a row near the canvas's bottom (below the tool strips): x of the first dark pixel (the DOM
    page frame's outline) and of the first pixel of the page's light fill (drawn by the canvas), or
    None. The pasteboard is a middle grey, the outline dark, the page fill light."""
    width, height, rgb = image
    row = height * 9 // 10
    pixel = lambda x: rgb[3 * (row * width + x):3 * (row * width + x) + 3]
    outline = next((x for x in range(width) if max(pixel(x)) < 120), None)
    fill = next((x for x in range(width) if min(pixel(x)) > 220), None)
    return None if outline is None or fill is None else (outline, fill)


def lag(events, out):
    """How far the canvas trails the DOM page frame in the `empty-pan` snapshots, in pan moves (one
    move per frame): the white page edge's offset from the outline, minus the offset at rest, over
    the move's step."""
    snaps = [e for e in events if e["event"] == "snap" and "file" in e]
    rest = next((page_edges(ppm(out / e["file"])) for e in snaps if e["scenario"] == "empty-rest" and ppm(out / e["file"])), None)
    if not rest:
        if snaps:
            print("lag: no page edges found in the rest snapshot (see target/perf/snap-*.png)")
        return
    base = rest[1] - rest[0]
    moves = []
    for e in snaps:
        image = ppm(out / e["file"]) if e["scenario"] == "empty-pan" else None
        edges = page_edges(image) if image else None
        if edges:
            # Panning right moves the page right: a canvas that is late shows the page left of
            # its frame (white edge closer to the outline), so the lag is minus the offset change.
            moves.append(-((edges[1] - edges[0]) - base) / e["step"])
    if moves:
        print(f"lag of the canvas behind the DOM page frame in empty-pan ({len(moves)} snapshots): "
              + ", ".join(f"{m:.1f}" for m in moves) + f" moves; median {quantile(moves, 0.5):.1f} (rest offset {base} px)")


def unfinished(events):
    """The first scenario of the probe's list that sent no end event."""
    ended = {e["name"] for e in events if e["event"] == "scenario" and e["state"] == "end"}
    return next((name for name in SCENARIOS if name not in ended), "(all ended)")


def report(events, procs, out):
    marks, views = {}, {}
    for e in events:
        if e["event"] == "scenario":
            marks.setdefault(e["name"], {})[e["state"]] = e["wall"]
            views.setdefault(e["name"], {})[e["state"]] = e.get("view", "?")
    if not procs or not marks:
        print("no data: the page sent no scenario events (see target/perf/events.ndjson)")
        return
    print(f"{'scenario':<12} {'s':>5} {'cpu %core':>9} {'peak RSS MiB':>12} {'built/s':>8} {'submit/s':>9} {'changed/s':>9} {'render ms avg':>13} {'max':>6} {'pages/s':>8} {'pages ms avg':>12}"
          f" {'rAF p50':>7} {'p95':>5} {'>33ms':>5}")
    for name, m in marks.items():
        if "end" not in m:
            continue
        window = [p for p in procs if m["start"] <= p["wall"] <= m["end"]]
        if len(window) < 2:
            continue
        secs = window[-1]["wall"] - window[0]["wall"]
        cpu = 100 * (window[-1]["cpu"] - window[0]["cpu"]) / secs
        rss = max(p["rss"] for p in window)
        rows = [e for e in events if e["event"] == "second" and e["scenario"] == name]
        n = max(len(rows), 1)
        total = lambda k: sum(r[k] for r in rows)
        built = total("built")
        intervals = [i for r in rows for i in r.get("intervals", [])]
        print(f"{name:<12} {secs:5.0f} {cpu:9.1f} {rss:12.0f} {built / n:8.1f} {total('submitted') / n:9.1f} {total('changed') / n:9.1f} "
              f"{total('renderMs') / max(built, 1):13.2f} {max((r['renderMax'] for r in rows), default=0):6.1f} {total('pagesTicks') / n:8.1f} {total('pagesMs') / max(total('pagesTicks'), 1):12.2f}"
              f" {quantile(intervals, 0.5):7.1f} {quantile(intervals, 0.95):5.1f} {sum(1 for i in intervals if i > 33):5d}")
        a, b = window[0]["names"], window[-1]["names"]
        parts = ", ".join(f"{k} {100 * (b.get(k, 0) - a.get(k, 0)) / secs:.1f}%" for k in sorted(b) if b[k] - a.get(k, 0) > 0.01 * secs)
        print(f"  by process: {parts}")
        sites = {}
        for r in rows:
            for site, count in r.get("sites", {}).items():
                sites[site] = sites.get(site, 0) + count
        if sites:
            top = sorted(sites.items(), key=lambda kv: -kv[1])[:3]
            print("  changes by: " + ", ".join(f"{site} {count / n:.1f}/s" for site, count in top))
        phases = [r["phases"] for r in rows if len(r.get("phases", [])) == 7]
        frames = sum(p[0] for p in phases)
        if frames:
            sums = [sum(p[i] for p in phases) / frames for i in range(1, 7)]
            print(f"  per drawn frame ({frames:.0f}): draw list {sums[0]:.1f} ms, pictures {sums[1]:.2f} ms, upload {sums[2]:.1f} ms, "
                  f"submit {sums[3]:.1f} ms; {sums[4]:.0f} layers, {sums[5]:.0f} vertices")
        cadence(rows, n)
        v = views.get(name, {})
        before, after = pixels(out / f"{name}-start.png"), pixels(out / f"{name}-end.png")
        screen = "no screenshots" if before is None or after is None else ("canvas changed" if before != after else "canvas unchanged")
        print(f"  view (x, y, scale): {v.get('start', '?')} -> {v.get('end', '?')}; {screen}")
        if built == 0:
            print(f"  WARNING: render loop not running ({name}: built/s = 0)")
        elif not name.startswith("idle") and total("changed") == 0:
            print(f"  WARNING: scenario did not move the view ({name}: changed/s = 0)")
    lag(events, out)
    for e in events:
        if e["event"] == "observer":
            print(f"long-task observer: {'yes' if e['longtask'] else 'no (gap detector only)'}; entry types {e['entryTypes']}")
    for e in events:
        if e["event"] == "error":
            print(f"page error in {e['scenario']}: {e['message']}")


asyncio.run(main())
