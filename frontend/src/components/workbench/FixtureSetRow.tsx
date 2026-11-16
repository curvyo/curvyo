import { Check, ChevronDown, ChevronRight, ChevronUp, Trash2 } from "lucide-react";
import { type KeyboardEvent, type PointerEvent, useRef, useState } from "react";

import { FixtureLockButton } from "@/components/workbench/FixtureLockButton";
import { FixtureAddBar } from "@/components/workbench/FixtureAddBar";
import { RenameField } from "@/components/workbench/FixtureFields";
import { FixtureRows } from "@/components/workbench/FixtureRowItem";
import { ARMED_TRASH, ICON_BUTTON, checkBoxClass, focusAfterRemoval, walkRowControls } from "@/components/workbench/rowKeys";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import { setAddKinds } from "@/lib/fixtureRows";
import { onPageName, removedStatus, renamedText, setArmStatus, setKeyAction } from "@/lib/fixtureSets";
import type { FixtureRow, FixturesView, SetRow } from "@/lib/fixturesView";

const LINE1_CONTROLS =
  "[data-line1] > button[role=checkbox], [data-line1] > button[data-arm-trash], [data-line1] > button[data-order], [data-line1] > button[aria-pressed]";

export interface SetRowProps {
  set: SetRow;
  index: number;
  count: number;
  view: FixturesView;
  api: FixturesApi;
  arm: DeleteArm;
  openId: string | null;
  onOpen: (row: FixtureRow) => void;
  tabStop: boolean;
  onFocusRow: () => void;
  /** The 2px insertion line of a set drag, above or under this row. */
  insertion: "top" | "bottom" | null;
  /** This row is the one a drag picked up. */
  dragSource: boolean;
  onDragPress: (event: PointerEvent<HTMLElement>, index: number) => void;
}

/**
 * One machine set row (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 4.1 and 4.3): the
 * disclosure, the name (renamed in place), the On this page check, the armed trash and the order
 * arrows on line 1, the kinds and count on line 2; unfolded, the set's add bar and its fixture rows.
 */
export function FixtureSetRow(props: SetRowProps) {
  const { set, index, count, view, api, arm, tabStop, onFocusRow, insertion, dragSource, onDragPress } = props;
  // A page whose profile is not on this computer shows its sets read only (criterion 46).
  const readOnly = view.readOnly !== null;
  const [renaming, setRenaming] = useState(false);
  const item = useRef<HTMLLIElement>(null);
  const backToRow = () => requestAnimationFrame(() => item.current?.focus());
  const armed = arm.armed === set.id;
  const toggle = () => api.setUnfolded(set.id, !set.unfolded);
  const removeSet = () =>
    arm.press(set.id, setArmStatus(set), () => {
      // The next set row takes the focus, else "+ Set" (the button after the list).
      const addSet = item.current?.parentElement?.nextElementSibling as HTMLElement | null | undefined;
      focusAfterRemoval(item.current, () => (addSet?.isConnected ? addSet : null));
      api.removeSet(set.id);
      return removedStatus(set.name);
    });
  const onKey = (event: KeyboardEvent<HTMLLIElement>) => {
    if (walkRowControls(event, LINE1_CONTROLS)) return;
    if (event.target !== event.currentTarget || event.ctrlKey || event.metaKey) return;
    const action = setKeyAction(event.key, event.altKey, index, count, armed);
    if (action === null || (readOnly && action.kind !== "focus" && action.kind !== "toggle")) return;
    event.preventDefault();
    switch (action.kind) {
      case "focus":
        (event.currentTarget.parentElement?.children[action.index] as HTMLElement | undefined)?.focus();
        break;
      case "toggle":
        toggle();
        break;
      case "rename":
        setRenaming(true);
        break;
      case "arm":
      case "remove":
        removeSet();
        break;
      case "move":
        api.moveSet(set.id, action.to);
        // The row keeps focus across the re-render: it is the same node, only its place changed.
        requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-set-row="${set.id}"]`)?.focus());
        break;
    }
  };
  const trashLabel = armed ? `Press again to delete ${set.name}` : `Delete ${set.name}`;
  return (
    <li
      ref={item}
      data-set-row={set.id}
      {...{ [ARM_ZONE]: set.id }}
      tabIndex={tabStop ? 0 : -1}
      onFocus={(event) => {
        if (event.target === event.currentTarget) onFocusRow();
      }}
      onKeyDown={onKey}
      onPointerDown={readOnly ? undefined : (event) => onDragPress(event, index)}
      className={`relative flex flex-col rounded-md px-1 outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${armed ? "bg-[var(--destructive-soft)]" : ""} ${dragSource ? "outline outline-[1.5px] -outline-offset-[1.5px] outline-dashed outline-[var(--drag-ghost)]" : ""}`}
    >
      {insertion ? (
        <div
          aria-hidden
          data-insertion={insertion}
          className="pointer-events-none absolute left-0 right-0 z-10 h-[2px] bg-[var(--editor-accent)]"
          style={insertion === "top" ? { top: 0 } : { bottom: 0 }}
        />
      ) : null}
      <div data-line1 className="flex h-6 items-center gap-1 pointer-coarse:h-7">
        {renaming ? null : (
          <button
            type="button"
            aria-expanded={set.unfolded}
            aria-label={`${set.unfolded ? "Fold" : "Unfold"} ${set.name}`}
            tabIndex={-1}
            className="flex h-4 w-4 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            onClick={toggle}
          >
            {set.unfolded ? <ChevronDown size={12} aria-hidden /> : <ChevronRight size={12} aria-hidden />}
          </button>
        )}
        {renaming ? (
          <RenameField
            name={set.name}
            label={`Name of ${set.name}`}
            onCommit={(text, byKey) => {
              setRenaming(false);
              const name = renamedText(text, set.name);
              if (name !== null) api.renameSet(set.id, name);
              if (byKey) backToRow();
            }}
            onCancel={() => {
              setRenaming(false);
              backToRow();
            }}
          />
        ) : (
          <button
            type="button"
            title={set.name}
            tabIndex={-1}
            className={`min-w-0 flex-1 truncate text-left text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${set.onPage ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}
            onClick={toggle}
            onDoubleClick={readOnly ? undefined : () => setRenaming(true)}
          >
            {set.name}
          </button>
        )}
        {renaming || readOnly ? null : (
          <FixtureLockButton name={set.name} locked={set.locked} onPress={() => api.setSetLocked(set.id, !set.locked)} />
        )}
        {renaming ? null : (
          <>
            <button
              type="button"
              role="checkbox"
              aria-checked={set.onPage}
              aria-label={onPageName(set.name)}
              title="On this page"
              tabIndex={-1}
              className={checkBoxClass(set.onPage)}
              onClick={() => api.setOnPage(set.id, !set.onPage)}
            >
              {set.onPage ? <Check size={12} strokeWidth={3} absoluteStrokeWidth aria-hidden /> : null}
            </button>
            {readOnly ? null : (
              <>
                <button
                  type="button"
                  {...{ [ARM_TRASH]: set.id }}
                  aria-label={trashLabel}
                  title={trashLabel}
                  tabIndex={-1}
                  className={armed ? ARMED_TRASH : `${ICON_BUTTON} text-[var(--panel-muted-fg)]`}
                  onClick={removeSet}
                >
                  <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </button>
                <span className="w-2 shrink-0" aria-hidden />
                <OrderButton set={set} up available={index > 0} onPress={() => api.moveSet(set.id, index - 1)} />
                <OrderButton set={set} up={false} available={index < count - 1} onPress={() => api.moveSet(set.id, index + 1)} />
              </>
            )}
          </>
        )}
      </div>
      <p
        title={set.line2}
        className="h-4 truncate pl-5 text-[11px] leading-4 text-[var(--panel-muted-fg)] pointer-coarse:h-5 pointer-coarse:leading-5"
        onClick={toggle}
      >
        {set.line2}
      </p>
      {set.unfolded ? (
        <div data-set-body className="ml-4 mt-1 flex flex-col gap-1 border-l border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pl-1">
          {set.canAdd ? (
            <FixtureAddBar label={`Add fixture to ${set.name}`} kinds={setAddKinds(set)} onPress={(kind) => api.addSetFixture(set.id, kind)} />
          ) : null}
          <FixtureRows
            label={`Fixtures of ${set.name}`}
            addBarLabel={`Add fixture to ${set.name}`}
            rows={set.fixtures}
            setName={set.name}
            view={view}
            api={api}
            arm={arm}
            openId={props.openId}
            onOpen={props.onOpen}
          />
        </div>
      ) : null}
    </li>
  );
}

/** One order arrow: the slot stays when the set is at that end, the arrow is hidden (not disabled). */
function OrderButton({ set, up, available, onPress }: { set: SetRow; up: boolean; available: boolean; onPress: () => void }) {
  if (!available) return <span className="h-6 w-6 shrink-0" aria-hidden />;
  const Icon = up ? ChevronUp : ChevronDown;
  return (
    <button
      type="button"
      data-order=""
      aria-label={`Move ${set.name} ${up ? "up" : "down"}`}
      title={up ? "Move up" : "Move down"}
      tabIndex={-1}
      className={ICON_BUTTON}
      onClick={onPress}
    >
      <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
    </button>
  );
}
