import { Check, Star, X } from "lucide-react";
import { useEffect, useId, useRef } from "react";

import { PresetStrip } from "@/components/panel/PresetStrip";
import { scrollBodyTo } from "@/lib/panelScroll";
import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import type { FormField, FormUnit, FormatGroup, FormatsApi } from "@/hooks/useFormats";
import { Button } from "@/components/ui/button";
import { ToggleButton } from "@/components/ui/toggle-button";
import { fieldClass, PANEL_LABEL } from "@/components/ui/looks";
import { ActionNotice } from "@/components/ui/notice";

const NEW_GROUP = "__new__";

const UNIT_OPTIONS: readonly ToggleOption<FormUnit>[] = (
  [
    ["mm", "mm, millimetres"],
    ["cm", "cm, centimetres"],
    ["in", "in, inches"],
    ["px", "px, pixels at 96 per inch"],
  ] as const
).map(([value, label]) => ({
  value,
  label,
  tooltip: "",
  icon: <span className="text-sm">{value}</span>,
}));



interface TextFieldProps {
  field: FormField;
  label: string;
  rowLabel: string;
  value: string;
  suffix?: string;
  error: string | undefined;
  showChip: boolean;
  setRef: (field: FormField, element: HTMLInputElement | null) => void;
  onChange: (value: string) => void;
  onFocus: () => void;
  numeric?: boolean;
}

/** One labelled text field of the form with its validation chip. */
function TextField({
  field,
  label,
  rowLabel,
  value,
  suffix,
  error,
  showChip,
  setRef,
  onChange,
  onFocus,
  numeric = false,
}: TextFieldProps) {
  const chipId = useId();
  return (
    <div className="flex min-h-7 items-center gap-2">
      <span className={`${PANEL_LABEL} leading-4`}>{rowLabel}</span>
      <div className="relative w-[158px]">
        <input
          ref={(element) => setRef(field, element)}
          type="text"
          autoComplete="off"
          spellCheck={false}
          inputMode={numeric ? "decimal" : "text"}
          aria-label={label}
          aria-invalid={error ? true : undefined}
          aria-describedby={error && showChip ? chipId : undefined}
          value={value}
          onFocus={onFocus}
          onChange={(event) => onChange(event.target.value)}
          className={fieldClass({ invalid: Boolean(error), extra: `style-field w-full px-1.5 ${numeric ? "text-right tabular-nums" : ""}` })}
          style={{ paddingRight: suffix ? 34 : 6 }}
        />
        {suffix && (
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-sm text-[var(--panel-muted-fg)]"
          >
            {suffix}
          </span>
        )}
        {error && showChip && (
          <ActionNotice
            refusal
            maxWidth={226}
            id={chipId}
            className="absolute top-full right-0 z-30 mt-1"
          >
            {error}
          </ActionNotice>
        )}
      </div>
    </div>
  );
}

interface FormatFormProps {
  formats: FormatsApi;
  onReturnFocus: () => void;
}

/**
 * The inline form that adds or edits a format (`specs/0045-document-formats-
 * library/` criteria 18, 19 and 21): Name, Width, Height, Unit, Group, a new
 * group's name, "Add to quick selection", and the buttons. It
 * holds typed text only; Rust checks it and names each refused field. Enter in a
 * field presses Add (or Save), Escape closes the form and drops the draft.
 */
export function FormatForm({ formats, onReturnFocus }: FormatFormProps) {
  const { form } = formats;
  const refs = useRef<Partial<Record<FormField, HTMLInputElement>>>({});
  const root = useRef<HTMLFormElement>(null);
  const headingId = useId();
  const request = form?.focusRequest ?? null;

  // The form opens in view with the keyboard in Name; a refused press moves
  // the keyboard to the first refused field.
  useEffect(() => {
    if (request) {
      refs.current[request.field]?.focus();
    }
  }, [request]);
  useEffect(() => {
    scrollBodyTo(root.current);
  }, []);

  if (!form) {
    return null;
  }
  const { values, errors, chip } = form;
  const editing = form.editing !== null;
  const groups: FormatGroup[] = formats.list.groups;
  const newGroup = values.groupId === "";
  const setRef = (field: FormField, element: HTMLInputElement | null) => {
    if (element) {
      refs.current[field] = element;
    } else {
      delete refs.current[field];
    }
  };
  const common = (field: FormField) => ({
    field,
    error: errors[field],
    showChip: chip === field,
    setRef,
    onFocus: () => formats.focusFormField(field),
  });

  return (
    <form
      ref={root}
      aria-labelledby={headingId}
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        formats.submitForm();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          formats.cancelForm(true);
        }
      }}
      className="flex w-[244px] flex-col gap-2 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] p-2"
    >
      <h3 id={headingId} className="sr-only">
        {editing ? "Edit format" : "Add format"}
      </h3>
      <TextField
        {...common("name")}
        label="Format name"
        rowLabel="Name"
        value={values.name}
        onChange={(name) => formats.changeForm({ name })}
      />
      <TextField
        {...common("width")}
        label="Format width"
        rowLabel="Width"
        value={values.width}
        suffix={values.unit}
        numeric
        onChange={(width) => formats.changeForm({ width })}
      />
      <TextField
        {...common("height")}
        label="Format height"
        rowLabel="Height"
        value={values.height}
        suffix={values.unit}
        numeric
        onChange={(height) => formats.changeForm({ height })}
      />
      <div className="flex h-7 items-center gap-2">
        <span className={PANEL_LABEL}>Unit</span>
        <ToggleGroup
          label="Format unit"
          options={UNIT_OPTIONS}
          value={values.unit}
          onChange={(unit) => formats.changeForm({ unit })}
          itemWidth={36}
          onReturnFocus={onReturnFocus}
        />
      </div>
      <div className="flex flex-col gap-1">
        <span id={`${headingId}-group`} className="h-4 text-xs leading-4 font-semibold text-[var(--toolbar-icon)]">
          Group
        </span>
        <PresetStrip
          labelledBy={`${headingId}-group`}
          cells={[
            ...groups.map((group) => ({
              id: group.id,
              label: `Group ${group.name}`,
              content: group.name,
              tooltip: "",
            })),
            { id: NEW_GROUP, label: "New group", content: "New group…", tooltip: "" },
          ]}
          value={newGroup ? NEW_GROUP : values.groupId}
          width="w-full"
          onPress={(id) => formats.changeForm({ groupId: id === NEW_GROUP ? "" : id })}
          onReturnFocus={onReturnFocus}
        />
      </div>
      {newGroup && (
        <TextField
          {...common("group-name")}
          label="Group name"
          rowLabel="Group name"
          value={values.newGroupName}
          onChange={(newGroupName) => formats.changeForm({ newGroupName })}
        />
      )}
      {!editing && (
        <ToggleButton icon={Star}
          label="Add to quick selection"
          pressed={values.favourite}
          onPressedChange={(favourite) => formats.changeForm({ favourite })}
          onReturnFocus={onReturnFocus}
        />
      )}
      <div className="flex gap-2">
        <Button variant="primary" icon={Check} type="submit" className="min-w-[109px]">
          {editing ? "Save" : "Add"}
        </Button>
        <Button icon={X}
          onClick={(event) => {
            formats.cancelForm(event.detail === 0);
            if (event.detail > 0) {
              onReturnFocus();
            }
          }} className="min-w-[109px]"
        >
          Cancel
        </Button>
      </div>
    </form>
  );
}
