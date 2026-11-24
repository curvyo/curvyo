// The ghost of a Library drag (`specs/0137` UX notes 6.5, 7.1): the page thumbnail at 70 % (or the
// name over the panel) and, over the canvas, the Drop badge `Import` "New page", `Copy` "Copy" or
// `Ban` "Not on a sketch page".
import { DropBadgePill } from "@/components/DropBadgePill";

/** The badge's top-left from the hotspot (design system, "Drop badge"). */
const BADGE_OFFSET_PX = 16;

/** What the drag shows over the canvas: a page row's "New page", picks' "Copy", or `null` for the
 * sketch-page refusal. */
export type LibraryDropKind = "import" | "copy" | null;

const WORDS = { import: "New page", copy: "Copy" } as const;

export function LibraryDragGhost({ label, picture, over, kind }: { label: string; picture: string | null; over: boolean; kind: LibraryDropKind }) {
  const refused = over && kind === null;
  return (
    <>
      {picture ? (
        <img src={picture} alt="" className="library-drag-picture" style={{ opacity: refused ? 0.4 : 0.7 }} />
      ) : over ? null : (
        <span className="library-drag-label">{label}</span>
      )}
      {over ? (
        <DropBadgePill
          position="absolute"
          badge={{
            kind: kind === null ? "not_on_sketch_page" : kind,
            word: kind === null ? "Not on a sketch page" : WORDS[kind],
            refused,
            x: BADGE_OFFSET_PX,
            y: BADGE_OFFSET_PX,
          }}
        />
      ) : null}
    </>
  );
}

