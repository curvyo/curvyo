// Which sample the font preview's field shows (`specs/0077-font-preview` criteria 8 to 10,
// `adrs.md` decision 5): the one selected or edited text's first line when it is new, else what
// the maker typed.

/** What the Text bar state says the sample follows: a node id (empty for none) and its line. */
export interface SampleSource {
  node: string;
  sample: string;
}

/** The sample when the preview opens (criterion 8): the selected text's line, else the kept
 * sample. */
export function openingSample(source: SampleSource, kept: string): string {
  return source.node ? source.sample : kept;
}

/** The sample the field takes after a change of the Text bar state, or `null` to keep the field
 * (criteria 9 and 10): only when a single text is followed and it, or its line, differs from the
 * last state seen. */
export function followedSample(last: SampleSource, now: SampleSource): string | null {
  if (!now.node) {
    return null;
  }
  return now.node !== last.node || now.sample !== last.sample ? now.sample : null;
}
