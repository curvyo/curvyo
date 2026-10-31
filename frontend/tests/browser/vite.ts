// Starts and stops the dev server a browser test file talks to. Vite runs in its own process
// group, because killing `npx` alone leaves vite holding the port: the next test of the file then
// talks to a dying server (and `--strictPort` refuses to start its own).
import { spawn, type ChildProcess, type StdioOptions } from "node:child_process";

export function startVite(port: number, cwd: string, stdio: StdioOptions = "ignore"): ChildProcess {
  return spawn("npx", ["vite", "--port", String(port), "--strictPort"], { cwd, stdio, detached: true });
}

export async function stopVite(vite: ChildProcess | null | undefined, port: number): Promise<void> {
  if (!vite?.pid) {
    return;
  }
  try {
    process.kill(-vite.pid, "SIGKILL");
  } catch {
    // already gone
  }
  const until = Date.now() + 10_000;
  while (Date.now() < until) {
    const free = await fetch(`http://localhost:${port}/`).then(
      () => false,
      () => true,
    );
    if (free) {
      return;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
}
