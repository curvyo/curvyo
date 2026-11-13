import { ChevronLeft, MapPin, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { JudgeTable } from "@/components/panel/JudgeTable";
import { TestPhoto } from "@/components/panel/TestPhoto";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import type { MaterialsApi } from "@/hooks/useMaterials";
import { NOTE_MAX, deletedText, testArmText } from "@/lib/materialRows";
import type { MaterialBody, TestDetail as Detail } from "@/lib/materialsView";

const OUTLINE_BUTTON =
  "flex h-7 w-[228px] items-center gap-1 rounded border border-[var(--lane-line)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface TestDetailProps {
  material: MaterialBody;
  test: Detail;
  api: MaterialsApi;
  arm: DeleteArm;
  /** Back: the detail closes and the focus returns to the test's row. */
  onBack: () => void;
  /** The test was deleted: the detail closed, the focus moves on. */
  onDeleted: () => void;
}

/** The note: 3 lines high, growing to 8, at most 500 characters, saved on leaving, Escape reverts. */
function NoteField({ material, test, api }: { material: string; test: Detail; api: MaterialsApi }) {
  const [text, setText] = useState(test.note);
  const [seen, setSeen] = useState(test.note);
  const field = useRef<HTMLTextAreaElement>(null);
  // The note follows the document (an undo) while the maker is not typing.
  if (seen !== test.note) {
    setSeen(test.note);
    setText(test.note);
  }
  useEffect(() => {
    const element = field.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${Math.min(Math.max(element.scrollHeight, 3 * 18 + 8), 8 * 18 + 8)}px`;
  }, [text]);
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={`note-${test.id}`} className="text-xs font-semibold text-[var(--toolbar-icon)]">
        Note
      </label>
      <textarea
        id={`note-${test.id}`}
        ref={field}
        value={text}
        maxLength={NOTE_MAX}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => {
          if (text !== test.note) api.setNote(material, test.id, text);
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            setText(test.note);
          }
        }}
        className="w-[228px] resize-none rounded border border-[var(--lane-line)] bg-white px-1 py-1 text-[13px] leading-[18px] text-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      />
    </div>
  );
}

/**
 * The open test in the Materials tab (`specs/0112` `ux-notes.md` 8.2): back, the head with the
 * test's armed trash, the state and axes lines, the best-cell table with the set line, the note,
 * the photo and the place button.
 */
export function TestDetail({ material, test, api, arm, onBack, onDeleted }: TestDetailProps) {
  const key = `test:${material.id}:${test.id}`;
  const armed = arm.armed === key;
  return (
    <div
      role="group"
      aria-label={`Test ${test.id}`}
      onKeyDown={(event) => {
        if (event.key === "Escape" && !(event.target instanceof HTMLTextAreaElement) && !(event.target instanceof HTMLInputElement)) {
          event.preventDefault();
          event.stopPropagation();
          onBack();
        }
      }}
      className="flex flex-col gap-2"
    >
      <button
        type="button"
        data-test-back=""
        onClick={onBack}
        className="flex h-7 w-[228px] items-center gap-1 rounded text-left text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        <ChevronLeft size={16} aria-hidden className="shrink-0" />
        <span className="min-w-0 truncate">{material.name}</span>
      </button>
      <div className="flex items-center gap-2" {...{ [ARM_ZONE]: key }}>
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="font-mono text-base font-semibold tracking-[0.08em] text-[var(--toolbar-icon)]">{test.id}</span>
          <span className="truncate text-[13px] text-[var(--toolbar-icon)]">{test.head}</span>
        </div>
        <button
          type="button"
          {...{ [ARM_TRASH]: key }}
          aria-label={armed ? testArmText(test.id, test.photo !== null) : `Delete test ${test.id}`}
          title={armed ? "Press again to delete" : "Delete"}
          onClick={() =>
            arm.press(key, testArmText(test.id, test.photo !== null), () => {
              const refused = api.deleteTest(material.id, test.id);
              if (refused === "") {
                api.openTest("");
                onDeleted();
              }
              return deletedText(`Test ${test.id}`);
            })
          }
          className={
            armed
              ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
              : "flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          }
        >
          <Trash2 size={16} aria-hidden />
        </button>
      </div>
      <p className="text-xs text-[var(--panel-muted-fg)]">{test.stateLine}</p>
      <p className="text-[11px] tabular-nums text-[var(--panel-muted-fg)]">{test.axesLine}</p>
      <h3 className="text-xs font-semibold text-[var(--toolbar-icon)]">Best cell</h3>
      <JudgeTable key={`judge:${test.id}`} test={test} onChoose={(column, row) => api.judge(material.id, test.id, column, row)} />
      {test.setLine ? <p className="text-[13px] tabular-nums text-[var(--toolbar-icon)]">{test.setLine}</p> : null}
      <NoteField material={material.id} test={test} api={api} />
      <TestPhoto key={`photo:${test.id}`} material={material.id} test={test} api={api} arm={arm} />
      <button type="button" onClick={() => api.place(material.id, test.id)} className={OUTLINE_BUTTON}>
        <MapPin size={16} aria-hidden />
        <span className="truncate">{test.place.label}</span>
      </button>
    </div>
  );
}
