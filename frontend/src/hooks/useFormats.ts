import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { FormatListRecord, WasmSession } from "@/lib/editorSession";
import { pickFormatsFile, saveFormatsFile, setFormatsAside, storeFormats } from "@/lib/formatsFile";
import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import { type DeleteArm, useDeleteArm } from "@/hooks/useDeleteArm";

export interface FormatRow {
  id: string;
  name: string;
  size: string;
  applyName: string;
  starName: string;
  favourite: boolean;
  canStar: boolean;
  selected: boolean;
  canEdit: boolean;
  editName: string;
  deleteName: string;
  /** The trash's name while it is armed: "Press again to delete Key ring." */
  armedName: string;
  /** The rename field's name: "Name of Key ring". */
  renameName: string;
}

export interface FormatGroup {
  id: string;
  name: string;
  count: string;
  enabled: boolean;
  canToggle: boolean;
  canEdit: boolean;
  showName: string;
  foldName: string;
  deleteName: string;
  armedName: string;
  renameName: string;
  rows: FormatRow[];
}

/** What the "All formats" list shows (`curvyo-ui-core::format_list_view`). */
export interface FormatListView {
  total: string;
  canAdd: boolean;
  canExport: boolean;
  /** Why the user file could not be read; empty while it can. */
  broken: string;
  groups: FormatGroup[];
}

const EMPTY: FormatListView = { total: "", canAdd: false, canExport: false, broken: "", groups: [] };

function readList(session: WasmSession | null): FormatListView {
  if (!session) {
    return EMPTY;
  }
  const raw: FormatListRecord = session.format_list();
  const groupFlags = raw.group_flags;
  const rowFlags = raw.row_flags;
  const rowGroup = raw.row_group;
  const groups: FormatGroup[] = Array.from(raw.group_ids, (id, index) => ({
    id,
    name: raw.group_names[index],
    count: raw.group_counts[index],
    enabled: (groupFlags[index] & 1) !== 0,
    canToggle: (groupFlags[index] & 2) !== 0,
    canEdit: (groupFlags[index] & 4) !== 0,
    showName: raw.group_show_names[index],
    foldName: raw.group_fold_names[index],
    deleteName: raw.group_delete_names[index],
    armedName: raw.group_armed_names[index],
    renameName: raw.group_rename_names[index],
    rows: [],
  }));
  raw.row_ids.forEach((id, index) => {
    groups[rowGroup[index]].rows.push({
      id,
      name: raw.row_names[index],
      size: raw.row_sizes[index],
      applyName: raw.row_apply_names[index],
      starName: raw.row_star_names[index],
      favourite: (rowFlags[index] & 1) !== 0,
      canStar: (rowFlags[index] & 2) !== 0,
      selected: (rowFlags[index] & 4) !== 0,
      canEdit: (rowFlags[index] & 8) !== 0,
      editName: raw.row_edit_names[index],
      deleteName: raw.row_delete_names[index],
      armedName: raw.row_armed_names[index],
      renameName: raw.row_rename_names[index],
    });
  });
  const view: FormatListView = {
    total: raw.total_text,
    canAdd: raw.can_add,
    canExport: raw.can_export,
    broken: raw.broken,
    groups,
  };
  raw.free();
  return view;
}

export type FormUnit = "mm" | "cm" | "in" | "px";
export type FormField = "name" | "width" | "height" | "group-name";

/** The typed text of the add and edit form. The host holds it; Rust checks it. */
export interface FormValues {
  name: string;
  width: string;
  height: string;
  unit: FormUnit;
  /** The chosen group's id; empty for "New group...". */
  groupId: string;
  newGroupName: string;
  favourite: boolean;
}

export interface OpenForm {
  /** The id of the format being edited; `null` for Add. */
  editing: string | null;
  values: FormValues;
  /** The chip text of every refused field. */
  errors: Partial<Record<FormField, string>>;
  /** The field whose chip shows: the first refused, then the one focused. */
  chip: FormField | null;
  /** Counts up so a refused press moves focus even to the same field. */
  focusRequest: { field: FormField; n: number } | null;
}

/** The inline rename of a user format or group (`0045` criterion 43). */
export interface RenameState {
  kind: "format" | "group";
  id: string;
  /** The name when the field opened. */
  original: string;
  /** The typed text. */
  text: string;
  /** The chip text of a refused name; empty while none is shown. */
  error: string;
}

/** A failure notice 4 px under the button that was pressed. Never a success: the customer rule is
 * no notice after a successful operation; the screen reader hears the polite status instead. */
export interface FormatNotice {
  id: number;
  anchor: "add" | "import" | "export" | "broken";
  text: string;
}

const REFUSAL_MS = 8000;

export interface FormatsApi {
  list: FormatListView;
  expanded: boolean;
  toggleExpanded: () => void;
  folded: ReadonlySet<string>;
  toggleFold: (groupId: string) => void;
  pick: (id: string) => void;
  setFavourite: (id: string, on: boolean) => void;
  setGroupEnabled: (id: string, on: boolean) => void;
  form: OpenForm | null;
  openAdd: () => void;
  openEdit: (id: string) => void;
  changeForm: (change: Partial<FormValues>) => void;
  focusFormField: (field: FormField) => void;
  submitForm: () => void;
  /** `keyboard` is true when Escape or a key press closed it: the keyboard
   * goes to a sensible control instead of the canvas. */
  cancelForm: (keyboard: boolean) => void;
  /** The one armed trash and the polite status text (`0081` pattern). */
  arm: DeleteArm;
  /** A press on the trash of a user format or group: arms it, or deletes on the second press.
   * `next` is where the keyboard goes after a keyboard delete (`"disclosure"` or a row control). */
  pressDelete: (kind: "format" | "group", id: string, next: string | null) => void;
  renaming: RenameState | null;
  startRename: (kind: "format" | "group", id: string) => void;
  changeRename: (text: string) => void;
  /** Enter (`"enter"`) or blur (`"blur"`) in the rename field. */
  commitRename: (source: "enter" | "blur") => void;
  cancelRename: () => void;
  importFile: () => void;
  exportFile: () => void;
  setAside: (keyboard: boolean) => void;
  notice: FormatNotice | null;
  /** Where the keyboard goes back to after a form or confirm closed or the
   * file was set aside: `"add"`, `"disclosure"`, or `"<row key>:<control>"`
   * such as `"fmt:u-key-ring:apply"`. Consumed once by `takeFocusReturn`. */
  focusReturn: { target: string; n: number } | null;
  takeFocusReturn: (n: number) => void;
}

const FIELD_ORDER: FormField[] = ["name", "width", "height", "group-name"];

/**
 * The state of the formats library in the Document tab (`specs/0045-document-
 * formats-library/` criteria 12 to 25): whether the list is open, which groups
 * are folded, the open form with its typed text, the delete being confirmed and
 * the notice. Every rule, text and check is the session's; this holds typed
 * text and what is open. It lives above the tab body, so an open form's draft
 * survives a change of tab (`0043` criterion 17) and is dropped when a new
 * session replaces the old one (New, Open).
 */
export function useFormats(
  getSession: () => WasmSession | null,
  doc: DocumentPanelApi,
): FormatsApi {
  const session = getSession();
  const { refresh } = doc;
  // The list is re-read whenever the panel's view is (`doc.view` changes after every
  // sync and every command).
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const list = useMemo(() => readList(session), [session, doc.view]);
  const [expanded, setExpanded] = useState(false);
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const [form, setForm] = useState<OpenForm | null>(null);
  const arm = useDeleteArm();
  const { press: armPress, disarm } = arm;
  const [renaming, setRenaming] = useState<RenameState | null>(null);
  const renameRef = useRef<RenameState | null>(null);
  const [notice, setNotice] = useState<FormatNotice | null>(null);
  const [focusReturn, setFocusReturn] = useState<FormatsApi["focusReturn"]>(null);
  useEffect(() => {
    renameRef.current = renaming;
  }, [renaming]);
  const lastGroup = useRef("");
  const counter = useRef(0);

  // A new session (New, Open) drops the drafts, not the fold state.
  const [seen, setSeen] = useState(session);
  if (seen !== session) {
    setSeen(session);
    setForm(null);
    setRenaming(null);
    disarm();
    setNotice(null);
  }

  useEffect(() => {
    if (!notice) {
      return;
    }
    const handle = window.setTimeout(() => setNotice(null), REFUSAL_MS);
    return () => window.clearTimeout(handle);
  }, [notice]);

  const say = useCallback(
    (anchor: FormatNotice["anchor"], text: string) => {
      if (text === "") {
        return;
      }
      counter.current += 1;
      setNotice({ id: counter.current, anchor, text });
    },
    [],
  );

  /** Hands the new text of the user file to the host. */
  const write = useCallback(
    async (fileText: string, anchor: FormatNotice["anchor"]) => {
      const failure = await storeFormats(fileText);
      if (failure !== null) {
        say(anchor, `Could not save your formats: ${failure}.`);
      }
    },
    [say],
  );

  const toggleExpanded = useCallback(() => {
    disarm();
    setExpanded((open) => !open);
  }, [disarm]);
  const toggleFold = useCallback(
    (groupId: string) => {
      disarm();
      setFolded((now) => {
        const next = new Set(now);
        if (!next.delete(groupId)) {
          next.add(groupId);
        }
        return next;
      });
    },
    [disarm],
  );

  const pick = useCallback((id: string) => doc.pickPreset(id), [doc]);

  const setFavourite = useCallback(
    (id: string, on: boolean) => {
      const edit = getSession()?.set_format_favourite(id, on);
      if (edit?.ok) {
        void write(edit.file_text, "add");
      }
      refresh();
    },
    [getSession, refresh, write],
  );

  const setGroupEnabled = useCallback(
    (id: string, on: boolean) => {
      disarm();
      const edit = getSession()?.set_format_group_enabled(id, on);
      if (edit?.ok) {
        void write(edit.file_text, "add");
      }
      refresh();
    },
    [disarm, getSession, refresh, write],
  );

  const openWith = useCallback(
    (editing: string | null) => {
      const prefill = getSession()?.format_prefill(editing ?? "", lastGroup.current);
      if (!prefill) {
        return;
      }
      disarm();
      setForm({
        editing,
        values: {
          name: prefill.name,
          width: prefill.width,
          height: prefill.height,
          unit: prefill.unit as FormUnit,
          groupId: prefill.group_id,
          newGroupName: "",
          favourite: prefill.favourite,
        },
        errors: {},
        chip: null,
        focusRequest: { field: "name", n: ++counter.current },
      });
    },
    [disarm, getSession],
  );

  // Add while its form is open only moves the keyboard to Name (one form at a time).
  const openAdd = useCallback(() => {
    if (form && form.editing === null) {
      setForm({ ...form, focusRequest: { field: "name", n: ++counter.current } });
    } else {
      openWith(null);
    }
  }, [form, openWith]);

  const changeForm = useCallback((change: Partial<FormValues>) => {
    setForm((now) => {
      if (!now) {
        return now;
      }
      // The next keystroke clears the chip of the field that changed.
      const errors = { ...now.errors };
      if ("name" in change) delete errors.name;
      if ("width" in change || "unit" in change) delete errors.width;
      if ("height" in change || "unit" in change) delete errors.height;
      if ("newGroupName" in change) delete errors["group-name"];
      return { ...now, values: { ...now.values, ...change }, errors, chip: now.chip };
    });
  }, []);

  const focusFormField = useCallback((field: FormField) => {
    setForm((now) => (now ? { ...now, chip: now.errors[field] ? field : null } : now));
  }, []);

  const submitForm = useCallback(() => {
    const current = form;
    const active = getSession();
    if (!current || !active) {
      return;
    }
    const { values } = current;
    const fields = [
      values.name,
      values.width,
      values.height,
      values.unit,
      values.groupId,
      values.newGroupName,
    ];
    const edit =
      current.editing === null
        ? active.add_format(fields, values.favourite)
        : active.save_format(current.editing, fields, values.favourite);
    if (!edit.ok) {
      const errors: Partial<Record<FormField, string>> = {};
      edit.error_fields.forEach((field, index) => {
        errors[field as FormField] = edit.error_messages[index];
      });
      const first = FIELD_ORDER.find((field) => errors[field] !== undefined) ?? null;
      setForm({
        ...current,
        errors,
        chip: first,
        focusRequest: first ? { field: first, n: ++counter.current } : null,
      });
      return;
    }
    lastGroup.current = values.groupId;
    setForm(null);
    setFocusReturn({
      target: current.editing === null ? "add" : `fmt:${current.editing}:apply`,
      n: ++counter.current,
    });
    void write(edit.file_text, "add");
    refresh();
  }, [form, getSession, refresh, write]);

  const returnKeyboardTo = useCallback((target: string) => {
    setFocusReturn({ target, n: ++counter.current });
  }, []);

  const cancelForm = useCallback(
    (keyboard: boolean) => {
      if (keyboard && form) {
        returnKeyboardTo(form.editing === null ? "add" : `fmt:${form.editing}:edit`);
      }
      setForm(null);
    },
    [form, returnKeyboardTo],
  );

  const pressDelete = useCallback(
    (kind: "format" | "group", id: string, next: string | null) => {
      const active = getSession();
      if (!active) {
        return;
      }
      const group = kind === "group" ? list.groups.find((g) => g.id === id) : undefined;
      const row =
        kind === "format" ? list.groups.flatMap((g) => g.rows).find((r) => r.id === id) : undefined;
      const armedName = group?.armedName ?? row?.armedName;
      if (armedName === undefined) {
        return;
      }
      const name = group?.name ?? row?.name ?? "";
      armPress(`${kind === "format" ? "fmt" : "grp"}:${id}`, armedName, () => {
        const edit = kind === "format" ? active.delete_format(id) : active.delete_format_group(id);
        if (edit.ok) {
          void write(edit.file_text, "add");
        }
        if (next !== null) {
          setFocusReturn({ target: next, n: ++counter.current });
        }
        refresh();
        return kind === "format" ? `${name} deleted.` : `Group ${name} deleted.`;
      });
    },
    [armPress, getSession, list, refresh, write],
  );

  const startRename = useCallback(
    (kind: "format" | "group", id: string) => {
      const found =
        kind === "group"
          ? list.groups.find((g) => g.id === id && g.canEdit)
          : list.groups.flatMap((g) => g.rows).find((r) => r.id === id && r.canEdit);
      if (!found) {
        return;
      }
      disarm();
      const next: RenameState = { kind, id, original: found.name, text: found.name, error: "" };
      renameRef.current = next;
      setRenaming(next);
    },
    [disarm, list],
  );

  const changeRename = useCallback((text: string) => {
    setRenaming((now) => {
      const next = now ? { ...now, text, error: "" } : now;
      renameRef.current = next;
      return next;
    });
  }, []);

  const closeRename = useCallback((keyboardTarget: string | null) => {
    renameRef.current = null;
    setRenaming(null);
    if (keyboardTarget !== null) {
      setFocusReturn({ target: keyboardTarget, n: ++counter.current });
    }
  }, []);

  const cancelRename = useCallback(() => {
    const now = renameRef.current;
    if (now) {
      closeRename(`${now.kind === "format" ? "fmt" : "grp"}:${now.id}:${now.kind === "format" ? "apply" : "fold"}`);
    }
  }, [closeRename]);

  const commitRename = useCallback(
    (source: "enter" | "blur") => {
      const now = renameRef.current;
      const active = getSession();
      if (!now || !active) {
        return;
      }
      const back =
        source === "enter"
          ? `${now.kind === "format" ? "fmt" : "grp"}:${now.id}:${now.kind === "format" ? "apply" : "fold"}`
          : null;
      if (now.text.trim() === now.original) {
        closeRename(back);
        return;
      }
      const edit =
        now.kind === "format"
          ? active.rename_format(now.id, now.text)
          : active.rename_format_group(now.id, now.text);
      if (edit.ok) {
        void write(edit.file_text, "add");
        closeRename(back);
        refresh();
      } else if (source === "enter") {
        const error = edit.error_messages[0] ?? "";
        const next = { ...now, error };
        renameRef.current = next;
        setRenaming(next);
      } else {
        // Blur with an invalid name reverts without a notice.
        closeRename(null);
      }
    },
    [closeRename, getSession, refresh, write],
  );

  const importFile = useCallback(() => {
    void (async () => {
      let picked;
      try {
        picked = await pickFormatsFile();
      } catch (error) {
        say("import", `Could not import: ${error instanceof Error ? error.message : String(error)}.`);
        return;
      }
      const active = getSession();
      if (!picked || !active) {
        return;
      }
      const edit = active.import_formats(picked.text, picked.name);
      if (edit.ok) {
        await write(edit.file_text, "import");
      }
      if (!edit.ok) {
        say("import", edit.notice);
      }
      refresh();
    })();
  }, [getSession, refresh, say, write]);

  const exportFile = useCallback(() => {
    void (async () => {
      const text = getSession()?.export_formats() ?? "";
      if (text === "") {
        return;
      }
      try {
        await saveFormatsFile(text);
      } catch (error) {
        say("export", `Could not export: ${error instanceof Error ? error.message : String(error)}.`);
      }
    })();
  }, [getSession, say]);

  const setAside = useCallback(
    (keyboard: boolean) => {
    void (async () => {
      const failure = await setFormatsAside();
      if (failure !== null) {
        say("broken", `Could not move the file: ${failure}.`);
        return;
      }
      getSession()?.formats_file_set_aside();
      if (keyboard) {
        returnKeyboardTo("disclosure");
      }
      refresh();
    })();
    },
    [getSession, refresh, returnKeyboardTo, say],
  );

  const takeFocusReturn = useCallback((n: number) => {
    setFocusReturn((now) => (now && now.n === n ? null : now));
  }, []);

  return {
    list,
    expanded,
    toggleExpanded,
    folded,
    toggleFold,
    pick,
    setFavourite,
    setGroupEnabled,
    form,
    openAdd,
    openEdit: openWith,
    changeForm,
    focusFormField,
    submitForm,
    cancelForm,
    arm,
    pressDelete,
    renaming,
    startRename,
    changeRename,
    commitRename,
    cancelRename,
    importFile,
    exportFile,
    setAside,
    notice,
    focusReturn,
    takeFocusReturn,
  };
}
