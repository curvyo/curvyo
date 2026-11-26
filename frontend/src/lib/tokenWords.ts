// The fixed words of the access token field, the Lock and the token form (`specs/0156-library-git-token`
// UX notes 2 to 4), worded in Rust and read when drawn (the wasm is loaded then).
import { library_token_words } from "@/wasm-bindings/curvyo_editor_wasm.js";

export interface TokenWords {
  add_label: string;
  statement: string;
  private_tooltip: string;
  new_label: string;
  label: string;
  change: string;
  add: string;
  remove: string;
  remove_armed: string;
  bad_text: string;
}

let words: TokenWords | null = null;

/** The words, parsed once. */
export function tokenWords(): TokenWords {
  words ??= JSON.parse(library_token_words()) as TokenWords;
  return words;
}
