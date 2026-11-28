// A library browser item's picture (`specs/0152-library-browser` criterion 6, ADR 0086 decision
// 8): an icon drawn from its body as an `<img>` of a `blob:` URL (never inline markup), a
// template's thumbnail or a page's stored picture, each asked once the tile has been in the page
// for 100 ms; until then a quiet bar, never another icon.
import { FileText, Files, TriangleAlert } from "lucide-react";
import { GLYPH, SMALL_GLYPH } from "@/components/ui/looks";
import { useEffect, useState } from "react";

import { heldIconTile, iconTile } from "@/lib/iconTiles";
import { pagePicture } from "@/lib/libraryApi";
import { noteBody, type TileWords } from "@/lib/libraryBrowserQuery";
import { onlineThumbnail } from "@/lib/onlineApi";
import { Tooltip } from "@/components/ui/tooltip";

const LOAD_AFTER_MS = 100;
/** The stored picture's size: never drawn larger (UX U22). */
const PICTURE_MAX = { width: 224, height: 160 };
/** Bodies already noted for the Colour refinement. */
const noted = new Set<number>();

function usePicture(words: TileWords): string | null {
  const icon = words.kind === "icon";
  // A stable dependency: the words are made again whenever the visible rows change.
  const pathKey = words.path.join("\u001f");
  const [url, setUrl] = useState<string | null>(() =>
    icon ? (heldIconTile(words.host, words.prefix, words.name)?.url ?? null) : null,
  );
  useEffect(() => {
    if (words.refusal || (!icon && !words.picture)) {
      return;
    }
    let live = true;
    let made: string | null = null;
    const timer = window.setTimeout(() => {
      if (icon) {
        void iconTile(words.host, words.prefix, words.name).then((tile) => {
          if (tile && !noted.has(words.index)) {
            noted.add(words.index);
            noteBody(words.index, tile.fixed);
          }
          if (live && tile) {
            setUrl(tile.url);
          }
        });
        return;
      }
      const png =
        words.kind === "template"
          ? onlineThumbnail(words.host, pathKey.split("\u001f").join("/"))
          : pagePicture(words.host, pathKey.split("\u001f"), words.page_id);
      void png.then(
        (bytes) => {
          if (live && bytes.length > 0) {
            made = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
            setUrl(made);
          }
        },
        () => undefined,
      );
    }, LOAD_AFTER_MS);
    return () => {
      live = false;
      window.clearTimeout(timer);
      if (made) {
        URL.revokeObjectURL(made);
      }
    };
  }, [icon, words.kind, words.index, words.host, words.prefix, words.name, pathKey, words.page_id, words.picture, words.refusal]);
  return words.refusal || (!icon && !words.picture) ? null : url;
}

export function ItemPreview({
  words,
  size,
  onShowPages,
}: {
  words: TileWords;
  size: number;
  onShowPages?: (index: number) => void;
}) {
  const url = usePicture(words);
  if (words.refusal) {
    return (
      <div className="flex items-center justify-center rounded-[5px] bg-white" style={{ width: size, height: size }}>
        <TriangleAlert {...GLYPH} aria-hidden className="text-[var(--panel-muted-fg)]" />
      </div>
    );
  }
  return (
    <div className="relative flex items-center justify-center overflow-hidden" style={{ width: size, height: size }}>
      {url ? (
        words.kind === "icon" ? (
          <img src={url} alt="" draggable={false} style={{ width: size * 0.75, height: size * 0.75 }} />
        ) : (
          <img
            src={url}
            alt=""
            draggable={false}
            className="object-contain"
            style={{ maxWidth: Math.min(size, PICTURE_MAX.width), maxHeight: Math.min(size, PICTURE_MAX.height) }}
          />
        )
      ) : words.kind !== "icon" && !words.picture ? (
        // Known to have no picture: the placeholder glyph, not a loading bar that never ends.
        <FileText size={Math.max(16, Math.round(size / 4))} strokeWidth={1.5} aria-hidden className="text-[var(--panel-muted-fg)]" />
      ) : (
        <span
          aria-hidden
          className="block rounded-[3px] bg-[var(--tile-skeleton)]"
          style={{ width: size * 0.6, height: Math.max(4, size / 4) }}
        />
      )}
      {words.pages > 1 ? (
        // A press on the chip shows the pages, as Show pages (H4); a 24 px target around it.
        <Tooltip side="bottom" content="Show pages">
          <button
            type="button"
            tabIndex={-1}
            aria-label={`Show ${words.pages} pages`}
            disabled={!onShowPages}
            onClick={(event) => {
              event.stopPropagation();
              onShowPages?.(words.index);
            }}
            onDoubleClick={(event) => event.stopPropagation()}
            onPointerDown={(event) => event.stopPropagation()}
            className="absolute right-0 bottom-0 flex h-6 items-center p-1"
          >
            <span className="flex h-4 items-center gap-0.5 rounded-[5px] border border-[var(--tile-edge)] bg-[var(--panel-bg)] px-1 text-[11px] tabular-nums text-[var(--toolbar-icon)]">
              <Files {...SMALL_GLYPH} aria-hidden />
              {words.pages}
            </span>
          </button>
        </Tooltip>
      ) : null}
    </div>
  );
}
