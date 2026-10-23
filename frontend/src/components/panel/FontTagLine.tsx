// The tag line of a family row (`specs/0075-font-library` criterion 10, UX tags amendment §2 to
// §5 and §8): the family's tags as chips with an x each, "+N" when they do not fit, Add tag at
// the end, and the add field with inline completion in place of the line.
import { CircleAlert, Plus, X } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { chipTooltip, moreTagsName, tagNameError } from "@/lib/fontLibraryText";
import type { RowTag } from "@/lib/fontRows";

const CONTROL =
  "outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface AddFieldProps {
  family: string;
  completions: (prefix: string) => string[];
  /** Enter: an error code, or `null` when the tag was added. */
  onAdd: (name: string) => { error?: string } | null;
  onClose: () => void;
}

/** The add field: inline completion of the first matching tag, Up and Down through the matches,
 * Right or End takes it, Enter adds, Escape or blur closes. */
function AddField({ family, completions, onAdd, onClose }: AddFieldProps) {
  const [typed, setTyped] = useState("");
  const [choice, setChoice] = useState(0);
  const [dropped, setDropped] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLInputElement | null>(null);
  const matches = typed && !dropped ? completions(typed) : [];
  const match = matches.length > 0 ? matches[((choice % matches.length) + matches.length) % matches.length] : null;
  const shown = match ? typed + match.slice(typed.length) : typed;

  useEffect(() => ref.current?.focus(), []);
  // The completion is the selected rest of the text after the caret.
  useLayoutEffect(() => {
    ref.current?.setSelectionRange(typed.length, shown.length);
  }, [typed, shown]);

  const take = () => {
    setTyped(shown);
    setChoice(0);
  };

  return (
    <div className="relative min-w-0 flex-1">
      <input
        ref={ref}
        type="text"
        value={shown}
        aria-label={`Add tag to ${family}`}
        aria-invalid={error !== null}
        onChange={(event) => {
          setTyped(event.target.value);
          setChoice(0);
          setDropped(false);
          setError(null);
        }}
        onBlur={onClose}
        onKeyDown={(event) => {
          event.stopPropagation();
          switch (event.key) {
            case "ArrowDown":
            case "ArrowUp":
              event.preventDefault();
              setChoice((at) => at + (event.key === "ArrowDown" ? 1 : -1));
              return;
            case "ArrowRight":
            case "End":
              if (match) {
                event.preventDefault();
                take();
              }
              return;
            case "Backspace":
            case "Delete":
              if (match && shown !== typed) {
                event.preventDefault();
                setDropped(true);
              }
              return;
            case "Enter": {
              event.preventDefault();
              if (!shown.trim()) {
                onClose();
                return;
              }
              const out = onAdd(shown);
              if (out?.error) {
                setError(tagNameError(out.error));
              } else {
                onClose();
              }
              return;
            }
            case "Escape":
              event.preventDefault();
              onClose();
              return;
            default:
          }
        }}
        onClick={(event) => event.stopPropagation()}
        className="h-5 w-full rounded-[5px] bg-white px-1.5 text-xs outline-none selection:bg-transparent selection:text-[var(--panel-muted-fg)] ring-1 ring-border focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      />
      {error ? (
        <p
          role="alert"
          className="absolute top-full left-0 z-30 mt-1 flex w-max max-w-[244px] items-center gap-1 rounded-md bg-popover px-2 py-1 text-xs text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]"
        >
          <CircleAlert size={12} aria-hidden />
          {error}
        </p>
      ) : null}
    </div>
  );
}

interface FontTagLineProps {
  family: string;
  tags: RowTag[];
  /** How many chips show before "+N"; all of them in an expanded row. */
  shown: number;
  /** The expanded row: the line wraps and has no "+N". */
  wrap: boolean;
  adding: boolean;
  completions: (prefix: string) => string[];
  onAdd: (name: string) => { error?: string } | null;
  /** `byKeyboard` is true for Enter or Space on the button. */
  onStartAdd: (byKeyboard: boolean) => void;
  onCloseAdd: () => void;
  onRemove: (tag: RowTag, byKeyboard: boolean) => void;
  onMore: () => void;
  /** Escape in the line: the focus goes back to the family row. */
  onLeave: () => void;
  /** Indented under the row's name (the tree); a preview tile sets it beside the name. */
  inset?: boolean;
}

/** The tag line. Its controls are not Tab stops; Shift+F2 or F2 then Escape enter it, Left and
 * Right move, Delete or Backspace on a chip removes it, Escape returns to the row; Up and Down
 * reach the tree, which moves to the next row. */
export function FontTagLine({
  family,
  tags,
  shown,
  wrap,
  adding,
  completions,
  onAdd,
  onStartAdd,
  onCloseAdd,
  onRemove,
  onMore,
  onLeave,
  inset = true,
}: FontTagLineProps) {
  const lineRef = useRef<HTMLDivElement | null>(null);
  const visible = wrap ? tags : tags.slice(0, shown);
  const more = wrap ? 0 : tags.length - visible.length;

  const controls = () => Array.from(lineRef.current?.querySelectorAll<HTMLElement>("[data-tag-control]") ?? []);

  return (
    <div
      ref={lineRef}
      data-tag-line
      onKeyDown={(event) => {
        if (adding) {
          return;
        }
        const list = controls();
        const at = list.indexOf(event.target as HTMLElement);
        if (at < 0) {
          return;
        }
        switch (event.key) {
          case "ArrowLeft":
          case "ArrowRight":
            event.preventDefault();
            event.stopPropagation();
            list[Math.max(0, Math.min(list.length - 1, at + (event.key === "ArrowRight" ? 1 : -1)))]?.focus();
            return;
          case "Home":
          case "End":
            event.preventDefault();
            event.stopPropagation();
            list[event.key === "Home" ? 0 : list.length - 1]?.focus();
            return;
          case "Enter":
          case " ":
            // The control's own click runs; the tree must not pick the family.
            event.stopPropagation();
            return;
          case "Delete":
          case "Backspace": {
            const tag = visible.find((t) => list[at].dataset.tagId === t.id);
            if (tag) {
              event.preventDefault();
              event.stopPropagation();
              onRemove(tag, true);
            }
            return;
          }
          case "Escape":
            event.preventDefault();
            event.stopPropagation();
            onLeave();
            return;
          default:
        }
      }}
      className={`flex items-center gap-1 ${inset ? "pr-2 pl-7" : ""} ${wrap ? "flex-wrap content-start py-0.5" : adding ? "h-6" : "h-6 overflow-hidden"}`}
    >
      {adding ? (
        <AddField family={family} completions={completions} onAdd={onAdd} onClose={onCloseAdd} />
      ) : (
        <>
          {visible.map((tag) => {
            const tip = chipTooltip(tag.source);
            const chip = (
              <span
                key={tag.id}
                className="flex h-5 max-w-full shrink-0 items-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] pl-1.5 text-xs text-[var(--toolbar-icon)] has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-[var(--editor-accent)]"
              >
                <span className="min-w-0 truncate">{tag.name}</span>
                <button
                  type="button"
                  tabIndex={-1}
                  data-tag-control
                  data-tag-id={tag.id}
                  aria-label={`Remove tag ${tag.name} from ${family}`}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={(event) => {
                    event.stopPropagation();
                    onRemove(tag, event.detail === 0);
                  }}
                  className="flex h-5 w-[18px] shrink-0 items-center justify-center rounded-r-[5px] text-[var(--panel-muted-fg)] outline-none hover:bg-[var(--editor-accent-hover)] hover:text-[var(--toolbar-icon)]"
                >
                  <X size={10} aria-hidden />
                </button>
              </span>
            );
            return tip ? (
              <Tooltip key={tag.id} side="bottom" content={tip}>
                {chip}
              </Tooltip>
            ) : (
              chip
            );
          })}
          {more > 0 ? (
            <button
              type="button"
              tabIndex={-1}
              data-tag-control
              aria-label={moreTagsName(more)}
              onClick={(event) => {
                event.stopPropagation();
                onMore();
              }}
              className={`flex h-5 shrink-0 items-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-1.5 text-xs tabular-nums text-[var(--panel-muted-fg)] hover:bg-[var(--editor-accent-hover)] ${CONTROL}`}
            >
              +{more}
            </button>
          ) : null}
          <Tooltip side="bottom" content="Add tag">
            <button
              type="button"
              tabIndex={-1}
              data-tag-control
              data-add-tag
              aria-label="Add tag"
              onMouseDown={(event) => event.preventDefault()}
              onClick={(event) => {
                event.stopPropagation();
                onStartAdd(event.detail === 0);
              }}
              className={`ml-auto flex size-5 shrink-0 items-center justify-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] ${CONTROL}`}
            >
              <Plus size={14} aria-hidden />
            </button>
          </Tooltip>
        </>
      )}
    </div>
  );
}
