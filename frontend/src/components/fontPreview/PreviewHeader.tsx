// The font preview's header (`specs/0077-font-preview` criteria 4, 5, 7, 12 and 26, UX notes §2;
// `specs/0082-glyph-table` criteria 1, 4 and 16, UX notes §2): the mode switch, the sample field
// (the search field in Glyphs mode) with Clear, the Size slider and its readout, the subject
// line, Full view and Close; one row from 560px of pane width, two rows below. Info mode has no
// field and no Size slider (`specs/0083-font-info-and-zip` UX notes §1).
import { Maximize2, Minimize2, Search, X } from "lucide-react";

import { ModeSwitch } from "@/components/fontPreview/ModeSwitch";
import { SizeSlider } from "@/components/fontPreview/SizeSlider";
import { Tooltip } from "@/components/ui/tooltip";
import { MAX_GLYPH_SEARCH_CHARS, MAX_SAMPLE_CHARS, type PreviewMode } from "@/lib/fontPreviewState";

interface PreviewHeaderProps {
  rows: 1 | 2;
  mode: PreviewMode;
  onMode: (mode: PreviewMode) => void;
  /** The sample, or in Glyphs mode the search text. */
  sample: string;
  onSample: (sample: string) => void;
  sampleRef: React.RefObject<HTMLInputElement | null>;
  /** Down or Enter in the search field: the focus goes to the first cell (UX notes P5). */
  onFieldDown: () => void;
  size: number;
  onSize: (size: number) => void;
  subject: string;
  /** The Full view button is in the tree (not when a split would leave the drawing under 360px, criterion 3). */
  canSplit: boolean;
  full: boolean;
  onFull: () => void;
  onClose: () => void;
  onReturnFocus: () => void;
}

const ICON_BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

export function PreviewHeader(props: PreviewHeaderProps) {
  const { rows, mode, onMode, sample, onSample, sampleRef, size, onSize, subject, canSplit, full, onFull, onClose, onReturnFocus } =
    props;
  const search = mode === "glyphs";
  const field = (
    <div className={`relative h-7 min-w-0 flex-1 ${rows === 1 ? "max-w-[480px] min-w-[200px]" : ""}`}>
      {search ? (
        <Search
          size={16}
          strokeWidth={1.5}
          aria-hidden
          className="pointer-events-none absolute top-1.5 left-2 text-[var(--panel-muted-fg)]"
        />
      ) : null}
      <input
        ref={sampleRef}
        type="text"
        value={sample}
        maxLength={search ? MAX_GLYPH_SEARCH_CHARS : MAX_SAMPLE_CHARS}
        aria-label={search ? "Search characters" : "Sample text"}
        spellCheck={false}
        onChange={(event) => onSample(event.target.value)}
        onKeyDown={(event) => {
          if (search && (event.key === "ArrowDown" || event.key === "Enter")) {
            event.preventDefault();
            props.onFieldDown();
          }
        }}
        className={`h-7 w-full rounded-[5px] bg-white pr-7 ${search ? "pl-7" : "pl-2"} text-sm text-[var(--toolbar-icon)] outline-none ring-1 ring-border focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]`}
      />
      {sample ? (
        <button
          type="button"
          aria-label={search ? "Clear search" : "Clear sample"}
          onClick={() => {
            onSample("");
            sampleRef.current?.focus();
          }}
          className="absolute top-0.5 right-0.5 flex size-6 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
        >
          <X size={16} strokeWidth={1.5} aria-hidden />
        </button>
      ) : null}
    </div>
  );
  const sizeGroup = (
    <div className="flex shrink-0 items-center gap-1">
      <span className="mr-1 text-xs text-[var(--panel-muted-fg)]">Size</span>
      <SizeSlider size={size} width={rows === 1 ? 120 : 96} onSize={onSize} onReturnFocus={onReturnFocus} />
    </div>
  );
  const subjectLine = (
    <Tooltip side="bottom" content={subject}>
      <p className="min-w-0 flex-1 truncate text-xs text-[var(--panel-muted-fg)]">{subject}</p>
    </Tooltip>
  );
  const buttons = (
    <div className="flex shrink-0 gap-1">
      {canSplit ? (
        <Tooltip side="bottom" content={full ? "Split view" : "Full view"}>
          <button type="button" aria-label={full ? "Split view" : "Full view"} onClick={onFull} className={ICON_BUTTON}>
            {full ? <Minimize2 size={16} strokeWidth={1.5} aria-hidden /> : <Maximize2 size={16} strokeWidth={1.5} aria-hidden />}
          </button>
        </Tooltip>
      ) : null}
      <Tooltip side="bottom" content="Close preview">
        <button type="button" aria-label="Close preview" onClick={onClose} className={ICON_BUTTON}>
          <X size={16} strokeWidth={1.5} aria-hidden />
        </button>
      </Tooltip>
    </div>
  );
  const frame = "shrink-0 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-2";
  if (mode === "info") {
    const switcher = <ModeSwitch mode={mode} onMode={onMode} onReturnFocus={onReturnFocus} />;
    return (
      <div className={frame}>
        {rows === 1 ? (
          <div className="flex h-7 items-center">
            {switcher}
            <div className="w-3 shrink-0" />
            {subjectLine}
            <div className="w-2 shrink-0" />
            {buttons}
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            <div className="flex h-7 items-center justify-between">
              {switcher}
              {buttons}
            </div>
            <div className="flex h-5 min-w-0 items-center">{subjectLine}</div>
          </div>
        )}
      </div>
    );
  }
  return (
    <div className={frame}>
      {rows === 1 ? (
        <div className="flex h-7 items-center">
          <ModeSwitch mode={mode} onMode={onMode} onReturnFocus={onReturnFocus} />
          <div className="w-3 shrink-0" />
          {field}
          <div className="w-4 shrink-0" />
          {sizeGroup}
          <div className="w-4 shrink-0" />
          {subjectLine}
          <div className="w-2 shrink-0" />
          {buttons}
        </div>
      ) : (
        // Grid areas keep the Tab order of criterion 22: sample, Size, Full view, Close.
        <div
          className="grid gap-x-1 gap-y-2"
          style={{ gridTemplateColumns: "minmax(0, 1fr) auto", gridTemplateAreas: '"field buttons" "second second"' }}
        >
          <div className="flex h-7 min-w-0 gap-2" style={{ gridArea: "field" }}>
            <ModeSwitch mode={mode} onMode={onMode} onReturnFocus={onReturnFocus} />
            {field}
          </div>
          <div className="flex h-7 min-w-0 items-center gap-3" style={{ gridArea: "second" }}>
            {sizeGroup}
            {subjectLine}
          </div>
          <div className="flex h-7 items-center" style={{ gridArea: "buttons" }}>
            {buttons}
          </div>
        </div>
      )}
    </div>
  );
}
