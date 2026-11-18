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
    parser.add_argument("--timeout", type=int, default=150)
    args = parser.parse_args()
    if not args.no_build:
        build()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    events, procs, state = [], [], {"pid": None, "done": asyncio.Event()}
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
        except (asyncio.IncompleteReadError, ConnectionError):
            pass

    server = await asyncio.start_server(on_client, "127.0.0.1", PORT)
    app = ROOT / "target" / "debug" / "curvyo-app"
    # xvfb-run keeps Xvfb alive for the app's lifetime; the app is its direct child.
    xvfb = subprocess.Popen(["xvfb-run", "-a", "-s", "-screen 0 1600x1000x24", str(app)], cwd=ROOT, start_new_session=True,
                            env=dict(os.environ, GDK_BACKEND="x11", WEBKIT_DISABLE_COMPOSITING_MODE=os.environ.get("WEBKIT_DISABLE_COMPOSITING_MODE", "0")))
    t0 = time.time()
    last = None
    try:
        while not state["done"].is_set() and time.time() - t0 < args.timeout:
            if xvfb.poll() is not None:
                print("app exited early", file=sys.stderr)
                break
            tree = next((p for p in descendants(xvfb.pid) if "curvyo-app" in Path(f"/proc/{p}/cmdline").read_text().replace("\0", " ").split(" ")[0]), None)
            if tree:
                cpu, rss, names = sample(tree)
                procs.append({"wall": time.time(), "cpu": cpu, "rss": rss, "names": names})
            await asyncio.sleep(1)
    finally:
        os.killpg(xvfb.pid, signal.SIGTERM)
        server.close()
    json.dump(procs, open(out / "procs.json", "w"))
    report(events, procs)


def report(events, procs):
    marks = {}
    for e in events:
        if e["event"] == "scenario":
            marks.setdefault(e["name"], {})[e["state"]] = e["wall"]
    if not procs or not marks:
        print("no data: the page sent no scenario events (see target/perf/events.ndjson)")
        return
    print(f"{'scenario':<12} {'s':>5} {'cpu %core':>9} {'peak RSS MiB':>12} {'built/s':>8} {'submit/s':>9} {'changed/s':>9} {'render ms avg':>13} {'max':>6} {'pages/s':>8} {'pages ms avg':>12}")
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
        print(f"{name:<12} {secs:5.0f} {cpu:9.1f} {rss:12.0f} {built / n:8.1f} {total('submitted') / n:9.1f} {total('changed') / n:9.1f} "
              f"{total('renderMs') / max(built, 1):13.2f} {max((r['renderMax'] for r in rows), default=0):6.1f} {total('pagesTicks') / n:8.1f} {total('pagesMs') / max(total('pagesTicks'), 1):12.2f}")
        a, b = window[0]["names"], window[-1]["names"]
        parts = ", ".join(f"{k} {100 * (b.get(k, 0) - a.get(k, 0)) / secs:.1f}%" for k in sorted(b) if b[k] - a.get(k, 0) > 0.01 * secs)
        print(f"  by process: {parts}")


asyncio.run(main())
