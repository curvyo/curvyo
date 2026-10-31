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
import {
  EMPTY_STYLE,
  type StyleOutcome,
  type StyleState,
  type WeightCode,
  boldKeyTarget,
  parseStyle,
  styleNotice,
} from "@/lib/styleText";
import type { NudgeReadout } from "@/hooks/useNudgeFeedback";
import { NUDGE_READOUT_MS } from "@/lib/nudgeText";
import { type AlignCode, kernKey, parseAlign } from "@/lib/textLayoutText";
import { parseLayerNotice } from "@/lib/layerRows";
import { EMPTY_TEXT_PATH, type TextPathBar, parseTextPath } from "@/lib/textPathText";
import {
  type OrientationCode,
  type WritingModeCode,
  parseOrientation,
  parseWritingMode,
} from "@/lib/textWritingText";
import { createFontListing } from "@/lib/fontListing";
import { invoke, isTauri } from "@/platform/host";

/** What the Text bar (Text tool) or the Select bar's Text group shows (criterion 11). */
export interface TextBarState {
  shown: boolean;
  /** "Source Sans 3 Regular", or "" for "Mixed". */
  font: string;
  missing: boolean;
  /** How many different faces the subject texts use, and how many of them are missing (UX notes
   * §0.4 of `specs/0084-text-bar-second-row`). */
  faces: number;
  missingFaces: number;
  /** The face a missing one is drawn in: Source Sans 3, or EMS Readability for a single-line
   * text (`specs/0076-single-line-fonts` criterion 22). */
  substitute: string;
  /** `null` for "Mixed". */
  sizeMm: number | null;
  /** The desktop build: Shift+Ctrl+T shows the Fonts tab (the browser has no key). */
  desktop: boolean;
  /** The one edited or selected text the font preview follows, empty for none
   * (`specs/0077-font-preview` criteria 8 and 9). */
  sampleNode: string;
  /** That text's first line, at most 200 characters. */
  sample: string;
  /** The Style group's six buttons (`specs/0078-font-style-buttons`). */
  style: StyleState;
  /** The pressed alignment, `null` when the subject texts differ
   * (`specs/0079-text-layout-controls` criterion 1). */
  align: AlignCode | null;
  /** The pressed writing mode and glyph orientation, `null` when the subject texts differ
   * (criterion 20). */
  writingMode: WritingModeCode | null;
  orientation: OrientationCode | null;
  /** The On path button and groups (`specs/0080-text-on-path`). */
  path: TextPathBar;
}

const EMPTY_BAR: TextBarState = {
  shown: false,
  font: "",
  missing: false,
  faces: 1,
  missingFaces: 0,
  substitute: "Source Sans 3",
  sizeMm: null,
  desktop: isTauri,
  sampleNode: "",
  sample: "",
  style: EMPTY_STYLE,
  align: "left",
  writingMode: "horizontal",
  orientation: "auto",
  path: EMPTY_TEXT_PATH,
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

/** A press's outcome from the session's JSON; nothing set for an empty answer (greyed). */
function outcomeOf(json: string): StyleOutcome {
  if (!json) {
    return { set: 0, left: 0, upright: false };
  }
  return JSON.parse(json) as StyleOutcome;
}

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
    /** The file's name and size, its folder path for display (desktop) and its font folder
     * (`specs/0083-font-info-and-zip` criteria 2 and 10). */
    file?: string;
    size?: number;
    path?: string;
    folder?: string | null;
    /** An included font's licence id and licence file (`font.toml`, ADR 0020 §4). */
    licence?: string;
    licence_file?: string;
  }[];
  skipped: string[];
  /** The Curvyo folder's font folders and their documents (`0083` criterion 4). */
  folders?: { name: string; documents: { name: string; size: number }[] }[];
}

/** The lists and bytes of this page, kept across sessions: a New or Open gets them again, and a
 * file is read from disk or the network once. */
const lists: Partial<Record<"curvyo" | "included" | "system", Listed>> = {};
const bytes = new Map<string, Uint8Array>();
const loading = new Set<string>();
const includedIds = new Set<string>();

/** The included fonts' index, as a list (both targets). */
async function listIncluded(): Promise<Listed> {
  const response = await fetch(`${fontsBase()}index.json`);
  const index = (await response.json()) as {
    faces: { family: string; style: string; file: string; kind?: string; licence?: string; licence_file?: string }[];
  };
  for (const face of index.faces) {
    includedIds.add(face.file);
  }
  return {
    faces: index.faces.map((face) => ({
      id: face.file,
      family: face.family,
      style: face.style,
      index: 0,
      kind: face.kind,
      file: face.file.slice(face.file.lastIndexOf("/") + 1),
      licence: face.licence,
      licence_file: face.licence_file,
    })),
    skipped: [],
  };
}

/** The mounted text hooks, told when a block is listed. */
const listeners = new Set<() => void>();

/** Lists each block once: the included index at start, the system fonts (desktop) when the Fonts
 * tab or the Text tool asks, and every block before an SVG import or an Open matches faces. */
const ensureListing = createFontListing<Listed>(
  {
    included: listIncluded,
    curvyo: () => invoke<Listed>("font_folder_list"),
    ...(isTauri
      ? {
          system: () =>
            invoke<Listed["faces"]>("system_font_list").then((faces) => ({ faces, skipped: [] })),
        }
      : {}),
  },
  lists,
  () => {
    for (const listed of listeners) {
      listed();
    }
  },
);

/** Sends every list read so far to `session`. */
export function sendFontLists(session: WasmSession): void {
  for (const block of ["curvyo", "included", "system"] as const) {
    const listed = lists[block];
    if (listed) {
      session.set_font_list(block, JSON.stringify(listed));
    }
  }
}

/** Resolves once every font block is listed (or could not be): an SVG import or an Open waits for
 * it, then hands the lists to its session with [`sendFontLists`], so phase 1 matches the faces
 * against the full listing and never takes a listed face for missing
 * (`specs/0024-svg-import-export` criterion 106). */
export function fontListingReady(): Promise<void> {
  return ensureListing();
}

/** The host's listing of a block as it arrived, with the class bytes and stamps the session does
 * not keep (the font library reads them). */
export function hostListing(block: "curvyo" | "included" | "system"): Listed["faces"] {
  return lists[block]?.faces ?? [];
}

/** The Curvyo block's font folders and their documents (`0083` criterion 4). */
export function hostFolders(): NonNullable<Listed["folders"]> {
  return lists.curvyo?.folders ?? [];
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

/** The face the glyph table shows (`specs/0082-glyph-table` "Terms", Frage 39 A). */
export interface GlyphSubject {
  /** "Great Vibes Regular". */
  name: string;
  family: string;
  missing: boolean;
  /** The listed face, `null` when missing. */
  face: FontFace | null;
  /** The characters Recognise text guessed in the subject text while the session holds them
   * (`specs/0088-trace-recognition` criterion 7), each once; empty otherwise. */
  guessed: string;
}

export interface TextApi {
  editing: boolean;
  /** The readout of an Alt+arrow kern at the caret ("Kern +0.3, 0.0 mm"), canvas CSS px; gone
   * 800 ms after the last key (`0079` UX notes §10.3). */
  kernReadout: NudgeReadout | null;
  /** The glyph table's subject face; `null` before any font is listed. */
  glyphSubject: GlyphSubject | null;
  /** A cell's character inserted (`0082` criteria 12 to 14); unless `keepFocus`, the focus goes
   * to the edit sink. */
  insertGlyph: (character: string, keepFocus: boolean) => void;
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
  /** A press on a family row or a preview tile: the subject texts keep their style where the
   * family has it (`specs/0078-font-style-buttons` criterion 22). */
  pickFamily: (face: FontFace, byKeyboard: boolean) => void;
  /** The face a preview tile of `face`'s family shows and applies (`0078` criterion 24). */
  tileFace: (face: FontFace) => { id: string; faceIndex: number };
  /** A weight or Italic press; returns the notice to show under the button, if any. */
  pressWeight: (code: WeightCode) => string | null;
  pressItalic: () => string | null;
  /** A press in the Alignment group (`specs/0079-text-layout-controls` criterion 1). */
  setAlign: (align: AlignCode) => void;
  /** A press in the Writing mode or Glyph orientation group (criterion 20). */
  setWritingMode: (mode: WritingModeCode) => void;
  setOrientation: (orientation: OrientationCode) => void;
  /** The last "Put on path" for the polite region (`specs/0080-text-on-path` UX notes §2). */
  pathAnnouncement: { text: string; seq: number };
  /** Where the "1" badge of the pick set sits, canvas px (`0080` UX notes §9); `null` without. */
  pickBadge: { x: number; y: number } | null;
  /** The On path button and groups' presses. */
  path: {
    pressOnPath: () => void;
    release: () => void;
  };
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
  /** The last "Put on path" for the polite region, with a sequence so a repeat is spoken. */
  const [pathAnnouncement, setPathAnnouncement] = useState<{ text: string; seq: number }>({ text: "", seq: 0 });
  const [fonts, setFonts] = useState<FontsTabData>(EMPTY_FONTS);
  const [missingMarks, setMissingMarks] = useState<{ x: number; y: number }[]>([]);
  const [sinkPoint, setSinkPoint] = useState<{ x: number; y: number } | null>(null);
  const [pickBadge, setPickBadge] = useState<{ x: number; y: number } | null>(null);
  const [sinkValue, setSinkValue] = useState<TextApi["sinkValue"]>(null);
  const [listsRevision, setListsRevision] = useState(0);
  const [objectToPathNotice, setObjectToPathNotice] = useState<string | null>(null);
  const [glyphSubject, setGlyphSubject] = useState<GlyphSubject | null>(null);
  const [kernReadout, setKernReadout] = useState<NudgeReadout | null>(null);
  const kernTimer = useRef<number | undefined>(undefined);
  const sinkRef = useRef<HTMLTextAreaElement | null>(null);
  const lastKeyRef = useRef(0);
  const composingRef = useRef(false);
  const sessionSeenRef = useRef<WasmSession | null>(null);
  const lastToolRef = useRef<string | null>(null);

  // The included fonts are listed at once (both targets); a list that arrives syncs again.
  useEffect(() => {
    const listed = () => setListsRevision((r) => r + 1);
    listeners.add(listed);
    void ensureListing(["included"]);
    return () => {
      listeners.delete(listed);
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
    void ensureListing(["system"]);
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
      sendFontLists(session);
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
      faces: raw.faces,
      missingFaces: raw.missing_faces,
      substitute: tab.current?.substitute ?? EMPTY_BAR.substitute,
      sizeMm: Number.isFinite(raw.size_mm) ? raw.size_mm : null,
      desktop: isTauri,
      sampleNode: raw.sample_node,
      sample: raw.sample,
      style: parseStyle(raw.style_json),
      align: parseAlign(raw.align),
      writingMode: parseWritingMode(raw.writing_mode),
      orientation: parseOrientation(raw.orientation),
      path: parseTextPath(session.text_path_view_json()),
    });
    raw.free();
    // "Put on path", spoken once by the polite region (`0080` UX notes §2).
    const said = session.take_text_path_announcement();
    if (said) {
      setPathAnnouncement({ text: said, seq: Date.now() });
    }
    setFonts(tab);
    const subject = JSON.parse(session.glyph_subject()) as {
      family: string;
      style: string;
      missing: boolean;
      index: number | null;
    } | null;
    const guessed = session.recognise_guessed_chars();
    setGlyphSubject((before) => {
      const next: GlyphSubject | null = subject
        ? {
            name: `${subject.family} ${subject.style}`,
            family: subject.family,
            missing: subject.missing,
            face: subject.index === null ? null : (tab.faces[subject.index] ?? null),
            guessed,
          }
        : null;
      const same =
        before !== null &&
        next !== null &&
        before.name === next.name &&
        before.missing === next.missing &&
        before.guessed === next.guessed &&
        before.face?.id === next.face?.id &&
        before.face?.faceIndex === next.face?.faceIndex;
      return same ? before : next;
    });
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
    // "Put text on 3 paths." in the canvas notice slot (`0080` criterion 37).
    const pathNotice = session.take_text_path_notice();
    if (pathNotice) {
      showNotice(pathNotice, undefined, "canvas", "status");
    }
    const badge = session.text_path_pick_badge();
    setPickBadge((previous) => {
      const next = badge.length === 2 ? { x: badge[0], y: badge[1] } : null;
      return previous?.x === next?.x && previous?.y === next?.y ? previous : next;
    });
  }, [
    syncRevision,
    listsRevision,
    getSession,
    syncFromSession,
    listHostFonts,
    containerRef,
    showNotice,
  ]);

  // A list that arrives later goes to the live session.
  useEffect(() => {
    const session = getSession();
    if (session && listsRevision > 0) {
      sendFontLists(session);
      syncFromSession();
    }
  }, [listsRevision, getSession, syncFromSession]);

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
      const kern = kernKey(key, event.altKey, event.shiftKey, ctrl);
      if (kern) {
        event.preventDefault();
        const readout = session.text_kern(kern[0], kern[1], kern[2]);
        afterInput(session);
        if (readout) {
          const caret = session.text_caret_point();
          if (caret.length === 2) {
            setKernReadout({ text: readout, x: caret[0], y: caret[1] });
            window.clearTimeout(kernTimer.current);
            kernTimer.current = window.setTimeout(() => setKernReadout(null), NUDGE_READOUT_MS);
          }
        }
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
      // Ctrl+B and Ctrl+I press Bold (Regular on a Bold text) and Italic, before the browser
      // takes them (`specs/0078-font-style-buttons` criterion 19).
      if (ctrl && !event.altKey && (key.toLowerCase() === "b" || key.toLowerCase() === "i")) {
        event.preventDefault();
        const raw = session.text_bar();
        const style = parseStyle(raw.style_json);
        raw.free();
        let notice: string | null = null;
        if (key.toLowerCase() === "b") {
          const target = boldKeyTarget(style);
          if (target) {
            notice = styleNotice(target, outcomeOf(session.press_text_weight(target)));
          }
        } else if (!style.italic.greyed) {
          notice = styleNotice("italic", outcomeOf(session.press_text_italic()));
        }
        if (notice) {
          showNotice(notice, 3000, "canvas");
        }
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
    [getSession, syncFromSession, returnToCanvas, afterInput, showNotice],
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

  const pickFamily = useCallback(
    (face: FontFace, byKeyboard: boolean) => {
      getSession()?.pick_font_family(face.id, face.faceIndex);
      syncFromSession();
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

  const tileFace = useCallback(
    (face: FontFace) => {
      const json = getSession()?.family_tile_face(face.id, face.faceIndex) ?? "null";
      const shown = JSON.parse(json) as { id: string; faceIndex: number } | null;
      return shown ?? { id: face.id, faceIndex: face.faceIndex };
    },
    [getSession],
  );

  const pressWeight = useCallback(
    (code: WeightCode) => {
      const outcome = outcomeOf(getSession()?.press_text_weight(code) ?? "");
      syncFromSession();
      return styleNotice(code, outcome);
    },
    [getSession, syncFromSession],
  );

  const pressItalic = useCallback(() => {
    const outcome = outcomeOf(getSession()?.press_text_italic() ?? "");
    syncFromSession();
    return styleNotice("italic", outcome);
  }, [getSession, syncFromSession]);

  const setAlign = useCallback(
    (align: AlignCode) => {
      getSession()?.set_text_alignment(align);
      syncFromSession();
    },
    [getSession, syncFromSession],
  );

  const setWritingMode = useCallback(
    (mode: WritingModeCode) => {
      getSession()?.set_text_writing_mode(mode);
      syncFromSession();
    },
    [getSession, syncFromSession],
  );

  // The On path button and groups (`specs/0080-text-on-path`): each press, then a re-read.
  const pathCommand = useCallback(
    (run: (session: NonNullable<ReturnType<typeof getSession>>) => void) => {
      const session = getSession();
      if (!session) {
        return;
      }
      run(session);
      syncFromSession();
    },
    [getSession, syncFromSession],
  );

  const pressOnPath = useCallback(() => {
    pathCommand((session) => {
      // A locked text is refused with the Pen's lock notice (criterion 7).
      if (session.text_path_press() === "locked") {
        const notice = parseLayerNotice(session.take_layer_notice());
        if (notice) {
          showNotice(notice.text, undefined, "canvas", notice.kind);
        }
      }
    });
  }, [pathCommand, showNotice]);

  const setOrientation = useCallback(
    (orientation: OrientationCode) => {
      getSession()?.set_text_glyph_orientation(orientation);
      syncFromSession();
    },
    [getSession, syncFromSession],
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

  const insertGlyph = useCallback(
    (character: string, keepFocus: boolean) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const outcome = JSON.parse(session.insert_from_glyph_table(character)) as { editing: boolean; limited: boolean };
      afterInput(session, outcome.limited);
      if (!keepFocus && outcome.editing) {
        focusSinkIfEditing();
      }
    },
    [getSession, afterInput, focusSinkIfEditing],
  );

  const refusalFace = useCallback(() => getSession()?.text_refusal_face() ?? "", [getSession]);

  return {
    editing,
    kernReadout,
    glyphSubject,
    insertGlyph,
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
    pickFamily,
    tileFace,
    pressWeight,
    pressItalic,
    setAlign,
    setWritingMode,
    setOrientation,
    pathAnnouncement,
    pickBadge,
    path: {
      pressOnPath,
      release: () => pathCommand((session) => session.release_text_path()),
    },
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
