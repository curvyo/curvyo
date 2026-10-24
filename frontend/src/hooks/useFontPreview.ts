// The font preview's state for the editor (`specs/0077-font-preview` criteria 5 to 10, `adrs.md`
// decisions 4 and 5): open or closed (the page store), the sample with the rule that follows the
// selected text, the size, the split or full choice and the dragged split, these and the typed
// sample kept per user.
import { useCallback, useEffect, useRef, useState } from "react";

import { useFontPreviewState } from "@/hooks/useFontPreviewState";
import { type PreviewState, setPreviewOpen } from "@/lib/fontPreviewStore";
import { type SampleSource, followedSample, openingSample } from "@/lib/fontPreviewSample";
import {
  type PreviewMode,
  type PreviewPrefs,
  readPreviewPrefs,
  withRecent,
  writePreviewPrefs,
} from "@/lib/fontPreviewState";
import { prerenderTiles } from "@/lib/previewFaces";

export interface FontPreviewApi {
  store: PreviewState;
  sample: string;
  /** The maker typed into the sample field: shown and kept. */
  typeSample: (sample: string) => void;
  size: number;
  setSize: (size: number) => void;
  /** Full view chosen (the forced full view of a narrow area is not stored). */
  wantFull: boolean;
  toggleFull: () => void;
  /** The dragged split's share of the canvas area; undefined for the default (criterion 27). */
  split: number | undefined;
  /** Keeps a dragged split, or forgets it (undefined) for the default. */
  setSplit: (split: number | undefined) => void;
  close: () => void;
  /** Fonts or Glyphs, kept (`specs/0082-glyph-table` criterion 3). */
  mode: PreviewMode;
  setMode: (mode: PreviewMode) => void;
  /** The glyph search field's text, kept (`0082` criterion 16). */
  glyphSearch: string;
  setGlyphSearch: (text: string) => void;
  /** The characters last inserted from the glyph table, most recent first (`0082` criterion 18). */
  recent: string[];
  addRecent: (character: string) => void;
}

export function useFontPreview(source: SampleSource): FontPreviewApi {
  const store = useFontPreviewState();
  const [prefs, setPrefs] = useState<PreviewPrefs>(() => readPreviewPrefs());
  const [sample, setSample] = useState(prefs.sample);
  const last = useRef<SampleSource>(source);
  const prefsRef = useRef(prefs);
  useEffect(() => {
    prefsRef.current = prefs;
  }, [prefs]);

  const keep = useCallback((change: (before: PreviewPrefs) => Partial<PreviewPrefs>) => {
    setPrefs((before) => {
      const next = { ...before, ...change(before) };
      writePreviewPrefs(next);
      return next;
    });
  }, []);

  // Opening: the selected text's first line, else the kept sample (criterion 8).
  useEffect(() => {
    if (store.open) {
      setSample(openingSample(last.current, prefsRef.current.sample));
    }
  }, [store.open]);

  // A new single text, or its line edited, replaces the field (criteria 9 and 10).
  useEffect(() => {
    const now = { node: source.node, sample: source.sample };
    const next = followedSample(last.current, now);
    last.current = now;
    if (next !== null) {
      setSample(next);
    }
  }, [source.node, source.sample]);

  // The Fonts tab lists families: their first tiles are prepared in the background, so the
  // preview opens with them drawn (customer report 2026-10-23). The open grid prepares its own.
  const { open, listing } = store;
  useEffect(() => {
    if (!open) {
      prerenderTiles(
        listing.families.map((entry) => entry.family.faces[0]),
        (face) => sample || face.family,
        null,
        () => {},
      );
    }
  }, [open, listing, sample]);

  const typeSample = useCallback(
    (text: string) => {
      setSample(text);
      keep(() => ({ sample: text }));
    },
    [keep],
  );

  return {
    store,
    sample,
    typeSample,
    size: prefs.size,
    setSize: useCallback((size: number) => keep(() => ({ size })), [keep]),
    wantFull: prefs.full,
    toggleFull: useCallback(() => keep((before) => ({ full: !before.full })), [keep]),
    split: prefs.split,
    setSplit: useCallback(
      (split: number | undefined) =>
        setPrefs((before) => {
          const { split: _old, ...rest } = before;
          const next: PreviewPrefs = split === undefined ? rest : { ...rest, split };
          writePreviewPrefs(next);
          return next;
        }),
      [],
    ),
    close: useCallback(() => setPreviewOpen(false), []),
    mode: prefs.mode ?? "fonts",
    setMode: useCallback((mode: PreviewMode) => keep(() => ({ mode })), [keep]),
    glyphSearch: prefs.glyphSearch ?? "",
    setGlyphSearch: useCallback((glyphSearch: string) => keep(() => ({ glyphSearch })), [keep]),
    recent: prefs.recent ?? [],
    addRecent: useCallback(
      (character: string) => keep((before) => ({ recent: withRecent(before.recent ?? [], character) })),
      [keep],
    ),
  };
}
