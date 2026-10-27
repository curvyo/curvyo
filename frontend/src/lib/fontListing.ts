// Lists each font block once and says when the asked blocks are listed, so the face matching of
// an SVG import or Open runs on the full listing (`specs/0024-svg-import-export` criterion 106).

/** The font blocks the host lists. */
export type FontBlock = "curvyo" | "included" | "system";

/** Returns `ensure(blocks)`: lists every block of `blocks` (all of `sources` when left out) that
 * was not asked for before, stores each into `store` and calls `listed` for it, and resolves once
 * all of them are settled. A block whose loader fails stays unlisted and does not hold the call. */
export function createFontListing<L>(
  sources: Partial<Record<FontBlock, () => Promise<L>>>,
  store: Partial<Record<FontBlock, L>>,
  listed: (block: FontBlock) => void,
): (blocks?: FontBlock[]) => Promise<void> {
  const asked = new Map<FontBlock, Promise<void>>();
  return (blocks) => {
    const wanted = blocks ?? (Object.keys(sources) as FontBlock[]);
    return Promise.all(
      wanted.map((block) => {
        const load = sources[block];
        if (!load) {
          return Promise.resolve();
        }
        let pending = asked.get(block);
        if (!pending) {
          pending = load()
            .then((value) => {
              store[block] = value;
              listed(block);
            })
            .catch(() => {});
          asked.set(block, pending);
        }
        return pending;
      }),
    ).then(() => {});
  };
}
