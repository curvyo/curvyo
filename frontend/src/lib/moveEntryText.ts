/**
 * The text the typed move chip hands to Rust (`specs/0047-transform-polish/` criteria 38 and
 * 44): a field the maker has not edited holds the prefill of one reference point, and Rust
 * reads a field equal to that prefill as "no change on that axis". The text that is *sent* must
 * therefore come from the point Rust compares against, not from the point the field happens to
 * show while the pointer rests on a mark.
 */

export interface MoveEntryTexts {
  /** Per reference point (0 to 8), the X and Y an untouched field shows in Absolute mode. */
  absolute: ReadonlyArray<readonly [string, string]>;
  /** The X and Y an untouched field shows in Relative mode. */
  relative: readonly [string, string];
}

/** What the maker typed: per axis, whether the field was edited and its text. */
export interface TypedFields {
  edited: readonly [boolean, boolean];
  typed: readonly [string, string];
}

/** The text of one field for the reference point `reference`. */
export function fieldText(
  entry: MoveEntryTexts,
  fields: TypedFields,
  axis: 0 | 1,
  absolute: boolean,
  reference: number,
): string {
  if (fields.edited[axis]) {
    return fields.typed[axis];
  }
  return absolute ? (entry.absolute[reference]?.[axis] ?? "") : entry.relative[axis];
}

/**
 * The two texts an Enter sends: the prefill of `chosen`, the reference point the maker chose,
 * for an untouched field. A mark under the pointer changes what the fields *show* (criterion 44)
 * but never what Enter sends.
 */
export function submittedTexts(
  entry: MoveEntryTexts,
  fields: TypedFields,
  absolute: boolean,
  chosen: number,
): [string, string] {
  return [
    fieldText(entry, fields, 0, absolute, chosen),
    fieldText(entry, fields, 1, absolute, chosen),
  ];
}
