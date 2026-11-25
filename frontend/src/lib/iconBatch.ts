// The icon bodies of one screen asked for together (`specs/0138-template-library`, ADR 0085
// decision 5): the cells that want a picture within one short window go out as one request per set,
// at most 100 names each, and an icon fetched once is kept for the session. Pure: the caller gives
// the asking function and the timer, so no request is made here.

/** Asks the host for the SVG of `names` of set `prefix` of source `key`: `[name, svg]`. */
export type AskIcons = (key: string, prefix: string, names: string[]) => Promise<[string, string][]>;

/** The most names one request carries. */
export const MAX_NAMES_PER_REQUEST = 100;

export function createIconBatcher(ask: AskIcons, schedule: (run: () => void) => void) {
  const waiting = new Map<string, Map<string, ((svg: string) => void)[]>>();
  const held = new Map<string, string>();
  let scheduled = false;

  const flush = () => {
    scheduled = false;
    const batches = [...waiting.entries()];
    waiting.clear();
    for (const [group, names] of batches) {
      const [key, prefix] = group.split("\u001f") as [string, string];
      const all = [...names.keys()];
      for (let at = 0; at < all.length; at += MAX_NAMES_PER_REQUEST) {
        const chunk = all.slice(at, at + MAX_NAMES_PER_REQUEST);
        const settle = (found: [string, string][]) => {
          const bodies = new Map(found);
          for (const name of chunk) {
            const svg = bodies.get(name) ?? "";
            if (svg) {
              held.set(`${group}\u001f${name}`, svg);
            }
            for (const resolve of names.get(name) ?? []) {
              resolve(svg);
            }
          }
        };
        void ask(key, prefix, chunk).then(settle, () => settle([]));
      }
    }
  };

  return {
    /** The SVG of one icon, "" when the source did not give it. */
    request(key: string, prefix: string, name: string): Promise<string> {
      const group = `${key}\u001f${prefix}`;
      const have = held.get(`${group}\u001f${name}`);
      if (have) {
        return Promise.resolve(have);
      }
      return new Promise((resolve) => {
        const names = waiting.get(group) ?? new Map<string, ((svg: string) => void)[]>();
        names.set(name, [...(names.get(name) ?? []), resolve]);
        waiting.set(group, names);
        if (!scheduled) {
          scheduled = true;
          schedule(flush);
        }
      });
    },
    /** Forgets the icons held for a source (Remove, Refresh). */
    forget(key: string): void {
      for (const k of [...held.keys()]) {
        if (k.startsWith(`${key}\u001f`)) {
          held.delete(k);
        }
      }
    },
  };
}
