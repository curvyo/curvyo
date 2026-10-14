interface StatusBarProps {
  /** The cursor readout, e.g. "x: 12.3  y: 45.6 mm". */
  cursorText: string;
  /** The document size, e.g. "210.0 × 297.0 mm". */
  sizeText: string;
  /** The current zoom level's integer percentage (acceptance criterion
   * 9), range 2-8000. */
  zoomPercent: number;
  /** The Node tool's node count, "5 of 120 nodes" (`specs/0053-node-marquee-selection/`
   * criterion 23), or `null` outside the Node tool or with no node to count. */
  nodeCountText?: string | null;
}

/**
 * The permanent piece of chrome `project-file-foundation` added
 * (specs/0001-project-file-foundation: "Canvas and empty state") and
 * `canvas-navigation-and-selection` extends: left, the cursor position;
 * **center, the zoom percentage** (acceptance criterion 9); right, the
 * document size. Both texts come from the session
 * (`specs/0015-document-size-and-rulers/` criteria 12 and 21): fixed decimals
 * and the display unit written once, formatted in Rust.
 */
export function StatusBar({ cursorText, sizeText, zoomPercent, nodeCountText }: StatusBarProps) {
  return (
    // Three columns, not `justify-between`, so the zoom stays centred however wide the right
    // segment grows with the node count (`0053` UX notes).
    <div
      className="grid h-6 shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-2 px-2 text-xs"
      style={{ background: "var(--statusbar-bg)" }}
    >
      {/* `whitespace-pre` keeps the two spaces between x and y; `tabular-nums`
          keeps the unit at the end from moving with the digits. */}
      <span className="min-w-0 truncate whitespace-pre tabular-nums">{cursorText}</span>
      <span>{zoomPercent}%</span>
      <span className="flex items-center justify-end gap-4 whitespace-pre tabular-nums">
        {nodeCountText && (
          <>
            <span className="text-xs">{nodeCountText}</span>
            <span
              aria-hidden
              className="h-3 w-px"
              style={{
                background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
              }}
            />
          </>
        )}
        <span>{sizeText}</span>
      </span>
    </div>
  );
}
