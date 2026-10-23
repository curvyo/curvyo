// The Text tool in the frontend (`specs/0074-text-objects`; ADR 0020 §3 and §5): the fonts the
// host lists and hands in (once per face per session), the hidden textarea that is the input sink
// of edit mode, the Text bar's values and the Fonts tab's data. The session holds the string,
// caret and selection and draws them; nothing here edits text.
import { useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  EMPTY_FONTS,
  type FontFace,
  type FontsTabData,
  TEXT_LIMIT_NOTICE,
  TEXT_WRITE_FAILED_NOTICE,
  listsOnToolChange,
  sizeNotice,
  textRefusalText,
} from "@/lib/textText";
import { invoke, isTauri } from "@/platform/host";

/** What the Text bar (Text tool) or the Select bar's Text group shows (criterion 11). */
export interface TextBarState {
  shown: boolean;
  /** "Source Sans 3 Regular", or "" for "Mixed". */
  font: string;
  missing: boolean;
  /** The face a missing one is drawn in: Source Sans 3, or EMS Readability for a single-line
   * text (`specs/0076-single-line-fonts` criterion 22). */
  substitute: string;
  /** `null` for "Mixed". */
  sizeMm: number | null;
  /** The desktop build: Shift+Ctrl+T shows the Fonts tab (the browser has no key). */
  desktop: boolean;
}

const EMPTY_BAR: TextBarState = {
  shown: false,
  font: "",
  missing: false,
  substitute: "Source Sans 3",
  sizeMm: null,
  desktop: isTauri,
};

type ShowNotice = (
  text: string,
  ms?: number,
  source?: "canvas" | "panel",
  kind?: "status" | "alert",
) => void;

/** The caret blinks 530 ms on and off, steady while typing and for 500 ms after (UX notes §2). */
const BLINK_MS = 530;
/** How long a refusal notice stays (design system, "Action notice"). */
const REFUSAL_MS = 8000;
const STEADY_AFTER_KEY_MS = 500;

const MAC = /mac/i.test(navigator.platform);

interface Listed {
  /** `os2` (24 hex digits) and `stamp` come with the Curvyo and system lists
   * (`specs/0075-font-library` decisions 4 and 5). */
  faces: {
    id: string;
    family: string;
    style: string;
    index: number;
    os2?: string | null;
    stamp?: string;
    /** `"single_line"` for an SVG font face (`specs/0076-single-line-fonts`). */
    kind?: string;
  }[];
  skipped: string[];
}

/** The lists and bytes of this page, kept across sessions: a New or Open gets them again, and a
 * file is read from disk or the network once. */
const lists: Partial<Record<"curvyo" | "included" | "system", Listed>> = {};
const bytes = new Map<string, Uint8Array>();
const loading = new Set<string>();
const includedIds = new Set<string>();

/** The host's listing of a block as it arrived, with the class bytes and stamps the session does
 * not keep (the font library reads them). */
export function hostListing(block: "curvyo" | "included" | "system"): Listed["faces"] {
  return lists[block]?.faces ?? [];
}

/** The bytes of a listed file, read once per page (the cuttable check reads them too). */
export async function listedFileBytes(id: string): Promise<Uint8Array> {
  const cached = bytes.get(id);
  if (cached) {
    return cached;
  }
  const read = await readFile(id);
  bytes.set(id, read);
  return read;
}

/** Forgets the bytes of files that were removed or replaced (a Remove, an Add under a name). */
export function forgetFileBytes(ids: string[]): void {
  for (const id of ids) {
    bytes.delete(id);
  }
}

function fontsBase(): string {
  return `${import.meta.env.BASE_URL}fonts/`;
}

async function readFile(id: string): Promise<Uint8Array> {
  if (includedIds.has(id)) {
    const response = await fetch(`${fontsBase()}${id}`);
    if (!response.ok) {
      throw new Error(`${id}: ${response.status}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  }
  const buffer = await invoke<ArrayBuffer>("font_bytes", { id });
  return new Uint8Array(buffer);
}

export interface TextApi {
  editing: boolean;
  bar: TextBarState;
  fonts: FontsTabData;
  /** The "Font missing" tags, canvas CSS px (criterion 29). */
  missingMarks: { x: number; y: number }[];
  /** Where the sink sits: the caret's canvas point, so the IME window opens there. */
  sinkPoint: { x: number; y: number } | null;
  /** What the sink mirrors for screen readers and IMEs. */
  sinkValue: { text: string; start: number; end: number } | null;
  sinkRef: React.RefObject<HTMLTextAreaElement | null>;
  sinkHandlers: {
    onKeyDown: (event: React.KeyboardEvent<HTMLTextAreaElement>) => void;
    /** The native `beforeinput` (React's synthetic one has no `inputType`). */
    onBeforeInput: (event: InputEvent) => void;
    onCompositionUpdate: (event: React.CompositionEvent<HTMLTextAreaElement>) => void;
    onCompositionEnd: (event: React.CompositionEvent<HTMLTextAreaElement>) => void;
    onCopy: (event: React.ClipboardEvent<HTMLTextAreaElement>) => void;
    onCut: (event: React.ClipboardEvent<HTMLTextAreaElement>) => void;
    onPaste: (event: React.ClipboardEvent<HTMLTextAreaElement>) => void;
  };
  pickFont: (face: FontFace, byKeyboard: boolean) => void;
  /** A typed or stepped size; returns the notice to show under the field, if any. */
  setSize: (mm: number) => string | null;
  previewSize: (mm: number | null) => void;
  /** Object to path of the selection, with its refusal notice (criteria 22, 29). */
  objectToPath: () => void;
  /** The refusal of the last Object to path, shown under its button; `null` when none. */
  objectToPathNotice: string | null;
  /** The Fonts tab is shown, or the Text tool is used: the Curvyo folder is listed again and the
   * system fonts are read the first time (criteria 19, 20). */
  listHostFonts: () => Promise<void>;
  /** The texts of the open project using a face of `family` from the Curvyo fonts block
   * (`0075` criterion 25). */
  textsUsingFamily: (family: string) => number;
  /** Focus goes to the sink when edit mode is on (after a canvas press or Enter). */
  focusSinkIfEditing: () => void;
  /** The same, at once: the sink is mounted while the Text tool is active, so the release of a
   * press that started edit mode hands it the keyboard before the next key. */
  focusSinkNow: () => void;
  /** The face name of a path command's refusal of texts. */
  refusalFace: () => string;
}

export function useText(
  getSession: () => WasmSession | null,
  syncFromSession: () => void,
  syncRevision: number,
  showNotice: ShowNotice,
  containerRef: React.RefObject<HTMLDivElement | null>,
): TextApi {
  const [editing, setEditing] = useState(false);
  const [bar, setBar] = useState<TextBarState>(EMPTY_BAR);
  const [fonts, setFonts] = useState<FontsTabData>(EMPTY_FONTS);
  const [missingMarks, setMissingMarks] = useState<{ x: number; y: number }[]>([]);
  const [sinkPoint, setSinkPoint] = useState<{ x: number; y: number } | null>(null);
  const [sinkValue, setSinkValue] = useState<TextApi["sinkValue"]>(null);
  const [listsRevision, setListsRevision] = useState(0);
  const [objectToPathNotice, setObjectToPathNotice] = useState<string | null>(null);
  const sinkRef = useRef<HTMLTextAreaElement | null>(null);
  const lastKeyRef = useRef(0);
  const composingRef = useRef(false);
  const sessionSeenRef = useRef<WasmSession | null>(null);
  const systemAskedRef = useRef(false);
  const lastToolRef = useRef<string | null>(null);

  /** Sends every list read so far to `session`. */
  const sendLists = useCallback((session: WasmSession) => {
    for (const block of ["curvyo", "included", "system"] as const) {
      const listed = lists[block];
      if (listed) {
        session.set_font_list(block, JSON.stringify(listed));
      }
    }
  }, []);

  // The included fonts are listed at once (both targets); the Curvyo folder on the desktop too.
  useEffect(() => {
    let live = true;
    void fetch(`${fontsBase()}index.json`)
      .then((response) => response.json())
      .then((index: { faces: { family: string; style: string; file: string; kind?: string }[] }) => {
        for (const face of index.faces) {
          includedIds.add(face.file);
        }
        lists.included = {
          faces: index.faces.map((face) => ({
            id: face.file,
            family: face.family,
            style: face.style,
            index: 0,
            kind: face.kind,
          })),
          skipped: [],
        };
        if (live) {
          setListsRevision((r) => r + 1);
        }
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, []);

  const listHostFonts = useCallback(() => {
    // The browser's Curvyo fonts are the files added there (`0075` criterion 28).
    const folder = invoke<Listed>("font_folder_list")
      .then((listed) => {
        lists.curvyo = listed;
        setListsRevision((r) => r + 1);
      })
      .catch(() => {});
    if (!isTauri) {
      return folder;
    }
    if (!systemAskedRef.current) {
      systemAskedRef.current = true;
      void invoke<Listed["faces"]>("system_font_list")
        .then((faces) => {
          lists.system = { faces, skipped: [] };
          setListsRevision((r) => r + 1);
        })
        .catch(() => {});
    }
    return folder;
  }, []);

  // Every sync: a new session gets the lists, the faces it asks for are read and handed in, and
  // the bar, the tab and the tags are read.
  useEffect(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    if (sessionSeenRef.current !== session) {
      sessionSeenRef.current = session;
      sendLists(session);
    }
    for (const id of session.wanted_font_files()) {
      const cached = bytes.get(id);
      if (cached) {
        session.add_font_file(id, cached);
        continue;
      }
      if (loading.has(id)) {
        continue;
      }
      loading.add(id);
      void readFile(id)
        .then((read) => {
          bytes.set(id, read);
          getSession()?.add_font_file(id, read);
          syncFromSession();
        })
        .catch(() => {})
        .finally(() => loading.delete(id));
    }
    const raw = session.text_bar();
    let tab: FontsTabData = EMPTY_FONTS;
    try {
      tab = JSON.parse(session.fonts_tab_json()) as FontsTabData;
    } catch {
      tab = EMPTY_FONTS;
    }
    setBar({
      shown: raw.shown,
      font: raw.font,
      missing: raw.missing,
      substitute: tab.current?.substitute ?? EMPTY_BAR.substitute,
      sizeMm: Number.isFinite(raw.size_mm) ? raw.size_mm : null,
      desktop: isTauri,
    });
    raw.free();
    setFonts(tab);
    const marks = session.missing_font_marks();
    const points: { x: number; y: number }[] = [];
    for (let i = 0; i + 1 < marks.length; i += 2) {
      points.push({ x: marks[i], y: marks[i + 1] });
    }
    setMissingMarks(points);
    const nowEditing = session.text_editing();
    setEditing(nowEditing);
    // In edit mode a press on the canvas leaves focus on the sink, not on the canvas.
    if (nowEditing && document.activeElement === containerRef.current) {
      window.requestAnimationFrame(() => sinkRef.current?.focus({ preventScroll: true }));
    }
    if (!nowEditing && document.activeElement === sinkRef.current) {
      containerRef.current?.focus();
    }
    const caret = session.text_caret_point();
    setSinkPoint(caret.length === 2 ? { x: caret[0], y: caret[1] } : null);
    if (!composingRef.current) {
      setSinkValue(JSON.parse(session.text_sink_json()) as TextApi["sinkValue"]);
    }
    // The folder is listed when the Text tool is entered and when the Fonts tab is shown, never
    // on a sync: a list that arrives syncs again (criteria 19, 20).
    const tool = session.tool();
    if (listsOnToolChange(lastToolRef.current, tool)) {
      void listHostFonts();
    }
    lastToolRef.current = tool;
    if (session.take_text_write_failed()) {
      showNotice(TEXT_WRITE_FAILED_NOTICE, 8000, "canvas", "alert");
    }
  }, [
    syncRevision,
    listsRevision,
    getSession,
    sendLists,
    syncFromSession,
    listHostFonts,
    containerRef,
    showNotice,
  ]);

  // A list that arrives later goes to the live session.
  useEffect(() => {
    const session = getSession();
    if (session && listsRevision > 0) {
      sendLists(session);
      syncFromSession();
    }
  }, [listsRevision, getSession, sendLists, syncFromSession]);

  // The caret blinks while edit mode is on (UX notes §2): steady while typing and 500 ms after,
  // steady with reduced motion, hidden without window focus.
  useEffect(() => {
    if (!editing) {
      return undefined;
    }
    const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    const timer = window.setInterval(() => {
      const session = getSession();
      if (!session) {
        return;
      }
      const sinceKey = performance.now() - lastKeyRef.current;
      const phaseOn = Math.floor(performance.now() / BLINK_MS) % 2 === 0;
      session.set_text_caret_on(
        document.hasFocus() && (reduced || sinceKey < STEADY_AFTER_KEY_MS || phaseOn),
      );
    }, 100);
    return () => {
      window.clearInterval(timer);
      getSession()?.set_text_caret_on(true);
    };
  }, [editing, getSession]);

  const focusSinkNow = useCallback(() => {
    if (getSession()?.text_editing()) {
      sinkRef.current?.focus({ preventScroll: true });
    }
  }, [getSession]);

  const focusSinkIfEditing = useCallback(() => {
    if (getSession()?.text_editing()) {
      window.requestAnimationFrame(() => sinkRef.current?.focus({ preventScroll: true }));
    }
  }, [getSession]);

  /** After a message: the limit notice, the redraw and the sink's mirror. */
  const afterInput = useCallback(
    (session: WasmSession, limited = false) => {
      lastKeyRef.current = performance.now();
      if (limited || session.take_text_limit_notice()) {
        showNotice(TEXT_LIMIT_NOTICE, 3000, "canvas");
      }
      syncFromSession();
    },
    [showNotice, syncFromSession],
  );

  const returnToCanvas = useCallback(() => {
    containerRef.current?.focus();
  }, [containerRef]);

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const native = event.nativeEvent;
      if (native.isComposing || composingRef.current) {
        return;
      }
      const ctrl = event.ctrlKey || event.metaKey;
      const key = event.key;
      if (key === "Escape") {
        event.preventDefault();
        session.key_down("Escape", false, false, false, event.repeat, false, MAC, event.timeStamp);
        syncFromSession();
        returnToCanvas();
        return;
      }
      if (key === "Tab") {
        // Tab goes to the Text bar's Font button; edit mode stays (UX notes §2).
        event.preventDefault();
        document.querySelector<HTMLElement>("[data-text-font-button]")?.focus();
        return;
      }
      const moves: Record<string, string> = {
        ArrowLeft: ctrl ? "word-left" : "left",
        ArrowRight: ctrl ? "word-right" : "right",
        ArrowUp: "up",
        ArrowDown: "down",
        Home: "home",
        End: "end",
      };
      if (key in moves) {
        event.preventDefault();
        session.text_move(moves[key], event.shiftKey);
        afterInput(session);
        return;
      }
      if (ctrl && key.toLowerCase() === "a") {
        event.preventDefault();
        session.text_select_all();
        afterInput(session);
        return;
      }
      if (ctrl && (key.toLowerCase() === "z" || key.toLowerCase() === "y")) {
        event.preventDefault();
        session.text_undo(key.toLowerCase() === "y" || event.shiftKey);
        afterInput(session);
      }
    },
    [getSession, syncFromSession, returnToCanvas, afterInput],
  );

  const onBeforeInput = useCallback(
    (native: InputEvent) => {
      const session = getSession();
      if (!session || native.inputType === "insertCompositionText") {
        return;
      }
      native.preventDefault();
      switch (native.inputType) {
        case "insertText":
        case "insertReplacementText":
          afterInput(session, session.text_insert(native.data ?? ""));
          return;
        case "insertLineBreak":
        case "insertParagraph":
          afterInput(session, session.text_insert("\n"));
          return;
        case "deleteContentBackward":
          session.text_delete(true);
          break;
        case "deleteContentForward":
          session.text_delete(false);
          break;
        case "deleteWordBackward":
          session.text_move("word-left", true);
          session.text_delete(true);
          break;
        case "deleteWordForward":
          session.text_move("word-right", true);
          session.text_delete(false);
          break;
        case "historyUndo":
          session.text_undo(false);
          break;
        case "historyRedo":
          session.text_undo(true);
          break;
        default:
          return;
      }
      afterInput(session);
    },
    [getSession, afterInput],
  );

  const onCompositionUpdate = useCallback(
    (event: React.CompositionEvent<HTMLTextAreaElement>) => {
      const session = getSession();
      if (!session) {
        return;
      }
      composingRef.current = true;
      session.text_preedit(event.data, event.data.length);
      afterInput(session);
    },
    [getSession, afterInput],
  );

  const onCompositionEnd = useCallback(
    (event: React.CompositionEvent<HTMLTextAreaElement>) => {
      const session = getSession();
      composingRef.current = false;
      if (!session) {
        return;
      }
      session.text_preedit("", 0);
      afterInput(session, event.data ? session.text_insert(event.data) : false);
    },
    [getSession, afterInput],
  );

  const onCopy = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const text = getSession()?.text_selected() ?? "";
      event.preventDefault();
      event.clipboardData.setData("text/plain", text);
    },
    [getSession],
  );

  const onCut = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const text = session.text_selected();
      event.preventDefault();
      event.clipboardData.setData("text/plain", text);
      if (text) {
        session.text_delete(true);
        afterInput(session);
      }
    },
    [getSession, afterInput],
  );

  const onPaste = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      const session = getSession();
      if (!session) {
        return;
      }
      event.preventDefault();
      afterInput(session, session.text_insert(event.clipboardData.getData("text/plain")));
    },
    [getSession, afterInput],
  );

  const pickFont = useCallback(
    (face: FontFace, byKeyboard: boolean) => {
      getSession()?.pick_font(face.id, face.faceIndex);
      syncFromSession();
      // A mouse pick returns focus to the canvas, or to the sink in edit mode (UX notes §4).
      if (!byKeyboard) {
        if (getSession()?.text_editing()) {
          sinkRef.current?.focus({ preventScroll: true });
        } else {
          returnToCanvas();
        }
      }
    },
    [getSession, syncFromSession, returnToCanvas],
  );

  const setSize = useCallback(
    (mm: number): string | null => {
      const outcome = getSession()?.set_text_size(mm) ?? "invalid";
      syncFromSession();
      return sizeNotice(outcome);
    },
    [getSession, syncFromSession],
  );

  const previewSize = useCallback(
    (mm: number | null) => {
      getSession()?.preview_text_size(mm ?? Number.NaN);
    },
    [getSession],
  );

  const objectToPath = useCallback(() => {
    const raw = getSession()?.object_to_path();
    syncFromSession();
    if (!raw) {
      return;
    }
    const text = textRefusalText({ kind: raw.kind, count: raw.count, face: raw.face });
    raw.free();
    // The refusal is posted 4 px under the Object to path button (UX notes §4 and §6).
    setObjectToPathNotice(text);
  }, [getSession, syncFromSession]);

  // The refusal lasts 8 s (Action notice, refusal) or until the next sync's selection change.
  useEffect(() => {
    if (!objectToPathNotice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setObjectToPathNotice(null), REFUSAL_MS);
    return () => window.clearTimeout(timer);
  }, [objectToPathNotice]);

  const refusalFace = useCallback(() => getSession()?.text_refusal_face() ?? "", [getSession]);

  return {
    editing,
    bar,
    fonts,
    missingMarks,
    sinkPoint,
    sinkValue,
    sinkRef,
    sinkHandlers: {
      onKeyDown,
      onBeforeInput,
      onCompositionUpdate,
      onCompositionEnd,
      onCopy,
      onCut,
      onPaste,
    },
    pickFont,
    setSize,
    previewSize,
    objectToPath,
    objectToPathNotice,
    listHostFonts,
    textsUsingFamily: (family: string) => getSession()?.texts_using_family(family) ?? 0,
    focusSinkIfEditing,
    focusSinkNow,
    refusalFace,
  };
}
