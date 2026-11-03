// The other arguments of a host command whose file bytes travel as a raw binary body
// (`curvyo-app`'s `raw_ipc`): JSON in one header, every character outside printable ASCII
// escaped, since a header carries ASCII only.

/** The header the host reads the arguments from. */
export const ARGS_HEADER = "curvyo-args";

/** `args` as JSON that holds printable ASCII only (`\uXXXX` escapes for the rest). */
export function asciiJson(args: Record<string, unknown>): string {
  return JSON.stringify(args).replace(
    /[^\x20-\x7e]/g,
    (char) => `\\u${char.charCodeAt(0).toString(16).padStart(4, "0")}`,
  );
}

/** The bytes of a raw host answer (an `ArrayBuffer`, or a number array from a stand-in). */
export function answerBytes(answer: ArrayBuffer | ArrayLike<number>): Uint8Array<ArrayBuffer> {
  if (answer instanceof ArrayBuffer) {
    return new Uint8Array(answer);
  }
  return Uint8Array.from(answer);
}
