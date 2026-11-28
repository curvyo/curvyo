import { useState } from "react";

import { FixtureSetsSection } from "@/components/workbench/FixtureSetsSection";
import { JobFixturesSection } from "@/components/workbench/JobFixturesSection";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import { FieldNoticeContext } from "@/components/workbench/fieldNotice";
import { noticeInTab, setsCountText } from "@/lib/fixtureSets";
import type { FixtureRow } from "@/lib/fixturesView";
import { SECTION_HEADING } from "@/components/ui/looks";

/** A section heading: the word 11px semibold muted, an optional count at the right. */
function Heading({ text, count }: { text: string; count?: string | null }) {
  return (
    <div className="flex h-6 items-center justify-between">
      <h3 className={SECTION_HEADING}>{text}</h3>
      {count ? <span className="text-[11px] tabular-nums text-[var(--panel-muted-fg)]">{count}</span> : null}
    </div>
  );
}

/**
 * The Fixtures tab of a laser page (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 3): Machine
 * sets, a divider, then Job fixtures, in one column that scrolls as a whole. The tab owns the one
 * armed trash and the one unfolded fixture row (F13); the session holds every rule.
 */
export function FixturesTab({ api }: { api: FixturesApi }) {
  const arm = useDeleteArm();
  const view = api.view;
  const [openId, setOpenId] = useState<string | null>(null);
  const selected = view?.selected ?? null;
  // A fixture selected on the canvas unfolds its row (state adjusted while rendering, no effect).
  const [seen, setSeen] = useState(selected);
  if (selected !== seen) {
    setSeen(selected);
    if (selected !== null) setOpenId(selected);
  }
  if (view === null) return null;
  // A press selects the fixture on the canvas (only while its set is on this page) and unfolds the row.
  const open = (row: FixtureRow) => {
    setOpenId(row.id);
    api.select(row.id);
  };
  const tabNotice = noticeInTab(api.notice);
  // Generic laser (criterion 3) or read only (criterion 46): one line under "Machine sets".
  const sectionLine = view.profileHint ?? view.readOnly;
  return (
    <FieldNoticeContext value={{ notice: api.notice, claim: api.claimField }}>
      <div className="flex flex-col gap-2">
        <Heading text="Machine sets" count={setsCountText(view)} />
        {sectionLine ? <p className="text-[11px] leading-4 text-[var(--panel-muted-fg)]">{sectionLine}</p> : null}
        <FixtureSetsSection view={view} api={api} arm={arm} openId={openId} onOpen={open} />
        <div
          aria-hidden
          className="my-1 h-px w-full"
          style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
        />
        <Heading text="Job fixtures" />
        <JobFixturesSection view={view} api={api} arm={arm} openId={openId} onOpen={open} />
        {view.damageNotices.map((text) => (
          <p key={text} role="status" className="text-xs text-[var(--panel-muted-fg)]">
            {text}
          </p>
        ))}
        {/* A clamp's notice shows under its field (UX notes section 10); only the rest shows here. */}
        {tabNotice ? (
          <p role="status" className="text-xs text-[var(--toolbar-icon)]">
            {tabNotice}
          </p>
        ) : null}
        <span role="status" className="sr-only">
          {arm.status}
        </span>
        {/* The selection announcer of `0044` (UX notes §11): the sizes are Rust's words. */}
        <span role="status" className="sr-only">
          {view.selectionAnnouncement ?? ""}
        </span>
      </div>
    </FieldNoticeContext>
  );
}
