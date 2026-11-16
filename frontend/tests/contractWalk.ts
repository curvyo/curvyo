// A helper of the view contract tests: every way to break one field of a Rust fixture, so a
// decoder is shown to reject a missing field and a field of the wrong type at every depth
// (`specs/0113-fixture-sets-and-pins` plan D1).

/** One broken copy of a fixture: where, how, and the value. */
export interface Break {
  path: string;
  how: "missing" | "mistyped";
  value: unknown;
}

function copy<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** A value of a different type than `value`: text for numbers and booleans, an array for null (no
 * nullable field takes one), a number for the rest. */
function wrongType(value: unknown): unknown {
  if (value === null) return ["wrong"];
  return typeof value === "number" || typeof value === "boolean" ? "wrong" : 7;
}

function visit(root: unknown, node: unknown, trail: Array<string | number>, out: Break[]): void {
  if (Array.isArray(node)) {
    node.forEach((item, index) => visit(root, item, [...trail, index], out));
    return;
  }
  if (typeof node !== "object" || node === null) return;
  for (const key of Object.keys(node)) {
    const here = [...trail, key];
    const path = here.join(".");
    out.push({ path, how: "missing", value: replace(root, here, undefined) });
    out.push({ path, how: "mistyped", value: replace(root, here, wrongType((node as Record<string, unknown>)[key])) });
    visit(root, (node as Record<string, unknown>)[key], here, out);
  }
}

function replace(root: unknown, trail: Array<string | number>, value: unknown): unknown {
  const next = copy(root);
  let at = next as Record<string | number, unknown>;
  for (const step of trail.slice(0, -1)) at = at[step] as Record<string | number, unknown>;
  const last = trail[trail.length - 1];
  if (value === undefined) delete at[last];
  else at[last] = value;
  return next;
}

/** Every missing and mistyped variant of `fixture`, one field at a time, array elements included. */
export function breakEveryField(fixture: unknown): Break[] {
  const out: Break[] = [];
  visit(fixture, fixture, [], out);
  return out;
}
