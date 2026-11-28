import { TriangleAlert } from "lucide-react";
import { GLYPH } from "@/components/ui/looks";

import { InlinePickList } from "@/components/InlinePickList";
import { MachineBlock } from "@/components/workbench/MachineBlock";
import type { UnitSymbol } from "@/hooks/useDocumentPanel";
import type { ManufacturingView, WorkbenchesApi } from "@/hooks/useWorkbenches";
import type { LibrarySyncApi } from "@/lib/devicesApi";
import type { PageRow } from "@/lib/pages";
import type { ManifestBlock, ManifestWorkbenchContribution } from "@/workbenches/registry";

interface ManifestTabProps {
  contribution: ManifestWorkbenchContribution;
  /** The document's pages: Place page lists the drawing pages (`specs/0100` criterion 15). */
  pages: PageRow[];
  view: ManufacturingView;
  workbenches: WorkbenchesApi;
  /** The document's display unit (the Machine block's work area line). */
  unit: UnitSymbol;
  /** The library sync's toggles (`specs/0116-library-sync`). */
  library?: LibrarySyncApi;
  onReturnFocus: () => void;
}

/**
 * A manufacturing workbench's tab, drawn from its extension's manifest (ADR 0034 §4,
 * `specs/0093-laser-workbench-shell` UX notes §4): the block kinds `Notice`, `MachineProfile`
 * (`specs/0026-machine-profile`) in the manifest's order; a `FixtureList` block draws nothing here (the Fixtures tab, `specs/0113`). Nothing here names a
 * process.
 */
export function ManifestTab({ contribution, pages, view, workbenches, unit, library, onReturnFocus }: ManifestTabProps) {
  const blocks = contribution.tabs[0]?.blocks ?? [];
  return (
    <div className="flex flex-col gap-4">
      {blocks.map((block, index) => (
        <Block
          key={`${block.kind}-${index}`}
          block={block}
          contribution={contribution}
          pages={pages}
          view={view}
          workbenches={workbenches}
          unit={unit}
          library={library}
          onReturnFocus={onReturnFocus}
        />
      ))}
    </div>
  );
}

function Block({
  block,
  pages,
  view,
  workbenches,
  unit,
  library,
  onReturnFocus,
}: ManifestTabProps & { block: ManifestBlock }) {
  switch (block.kind) {
    case "notice":
      return <NoticeBlock text={view.notices[block.id]} />;
    case "machine_profile":
      return view.machine ? (
        <>
          <MachineBlock label={block.label} machine={view.machine} workbenches={workbenches} unit={unit} library={library} onReturnFocus={onReturnFocus} />
          <PlacePage pages={pages} workbenches={workbenches} onReturnFocus={onReturnFocus} />
        </>
      ) : null;
    case "fixture_list":
      // The Fixtures tab has them since `0113`; the Document tab keeps no Fixtures block.
      return null;
  }
}

/** A `Notice` block: shown only while its text applies (UX notes §6). */
function NoticeBlock({ text }: { text: string | undefined }) {
  if (!text) {
    return null;
  }
  return (
    <div
      role="status"
      className="flex items-start gap-2 rounded-[5px] p-2 text-xs text-[var(--toolbar-icon)]"
      style={{
        background: "var(--toolbar-bg)",
        border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
      }}
    >
      <TriangleAlert {...GLYPH} aria-hidden className="shrink-0" />
      <span>{text}</span>
    </div>
  );
}

/** The full-width outline look of "Fit to content" (`specs/0100` UX notes §4). */
const placeButton =
  "flex h-7 w-full items-center justify-between rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-2 text-xs pointer-coarse:min-h-9";

/** Place page (`specs/0100` criterion 15): the drawing pages in list order with their size;
 * choosing one places it at the laser page's centre, focus goes to the canvas. Hidden when the
 * project has no drawing page. */
function PlacePage({
  pages,
  workbenches,
  onReturnFocus,
}: {
  pages: PageRow[];
  workbenches: WorkbenchesApi;
  onReturnFocus: () => void;
}) {
  const drawing = pages.filter((page) => page.kind === "drawing");
  if (drawing.length === 0) return null;
  return (
    <div className="flex flex-col gap-1">
      <InlinePickList
        label="Place page"
        className={placeButton}
        rows={drawing.map((page) => ({ id: page.id, name: page.name, detail: page.size }))}
        onPick={(id) => {
          if (workbenches.placePage(id)) onReturnFocus();
        }}
      />
    </div>
  );
}
