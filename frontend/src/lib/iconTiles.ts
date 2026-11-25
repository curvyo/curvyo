// The icon tiles of the library browser (`specs/0152-library-browser` criterion 6, ADR 0086
// decisions 7 and 8): the bodies of the tiles that want a picture within one short window go out
// as one request per set, at most 100 names each; each comes back as SVG text drawn in the UI's
// ink and is shown only as an `<img>` of a `blob:` URL, never as markup. At most 2,000 are held.
import { MAX_NAMES_PER_REQUEST } from "@/lib/iconBatch";
import { invoke } from "@/platform/host";

/** One drawn icon: its picture and whether its body has its own colours. */
export interface IconTile {
  url: string;
  fixed: boolean;
}

const MAX_HELD = 2000;
const held = new Map<string, IconTile>();
const waiting = new Map<string, Map<string, ((tile: IconTile | null) => void)[]>>();
let scheduled = false;

/** The UI's ink (`--toolbar-icon`) as three bytes, for monochrome icons. */
function ink(): [number, number, number] {
  const probe = document.createElement("span");
  probe.style.color = "var(--toolbar-icon)";
  document.body.appendChild(probe);
  const text = getComputedStyle(probe).color;
  probe.remove();
  const parts = text.match(/\d+(\.\d+)?/g)?.map(Number) ?? [];
  return [parts[0] ?? 31, parts[1] ?? 41, parts[2] ?? 55];
}

function keep(id: string, tile: IconTile): void {
  held.set(id, tile);
  while (held.size > MAX_HELD) {
    const [oldest, gone] = held.entries().next().value as [string, IconTile];
    held.delete(oldest);
    URL.revokeObjectURL(gone.url);
  }
}

function flush(): void {
  scheduled = false;
  const colour = ink();
  const batches = [...waiting.entries()];
  waiting.clear();
  for (const [group, names] of batches) {
    const [key, prefix] = group.split("\u001f") as [string, string];
    const all = [...names.keys()];
    for (let at = 0; at < all.length; at += MAX_NAMES_PER_REQUEST) {
      const chunk = all.slice(at, at + MAX_NAMES_PER_REQUEST);
      const settle = (found: [string, string, boolean][]) => {
        const bodies = new Map(found.map(([name, svg, fixed]) => [name, { svg, fixed }]));
        for (const name of chunk) {
          const body = bodies.get(name);
          let tile: IconTile | null = null;
          if (body?.svg) {
            tile = { url: URL.createObjectURL(new Blob([body.svg], { type: "image/svg+xml" })), fixed: body.fixed };
            keep(`${group}\u001f${name}`, tile);
          }
          for (const resolve of names.get(name) ?? []) {
            resolve(tile);
          }
        }
      };
      void invoke<[string, string, boolean][]>("online_icon_tiles", { key, prefix, names: chunk, colour }).then(
        settle,
        () => settle([]),
      );
    }
  }
}

/** The tile of one icon; null when the source did not give it. Asked after the caller's 100 ms in
 * view, collected for 50 ms. */
export function iconTile(key: string, prefix: string, name: string): Promise<IconTile | null> {
  const group = `${key}\u001f${prefix}`;
  const have = held.get(`${group}\u001f${name}`);
  if (have) {
    return Promise.resolve(have);
  }
  return new Promise((resolve) => {
    const names = waiting.get(group) ?? new Map<string, ((tile: IconTile | null) => void)[]>();
    names.set(name, [...(names.get(name) ?? []), resolve]);
    waiting.set(group, names);
    if (!scheduled) {
      scheduled = true;
      window.setTimeout(flush, 50);
    }
  });
}

/** The tile held for an icon, if any (no request). */
export function heldIconTile(key: string, prefix: string, name: string): IconTile | undefined {
  return held.get(`${key}\u001f${prefix}\u001f${name}`);
}
