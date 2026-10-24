# ADR 0021: Per-character values on a text

**Status:** Proposed — needs-customer; default A per Frage 42 KERN-MODEL, asked 2026-10-24,
unanswered. Proposed by the architect 2026-10-24. `needs-customer` (document model: a value per
character; file format: `format_version` bump). Extends
[ADR 0020](0020-text-nodes-text-engine-and-font-sources.md) §1 (the text node: its string gains
per-character values) and [ADR 0009](0009-concurrent-editing-semantics.md) §3 (they merge per
character with the string). Touches [ADR 0014](0014-history-undo-and-branches.md) §1 (the restore
engine learns them). Asked by `specs/0079-text-layout-controls` slice 4 (criteria 26 to 31, 37,
38).

## Context

Kerning (Frage 30 A) is a value on one character: it stays with that character when text is typed
or deleted before it, dies with it, is not given to a character typed next to it, and merges per
character between peers (criteria 30, 31). Every value on a text so far is a register on the node
(ADR 0020 §1, `0079` decision 1); none follows a character. The same need will come back for
glyph alternates (`0082`, Frage 40 B), so the mechanism is decided once.

What the code has today (format 20, Loro `=1.16.2`): `text_string` is already a `LoroText`
container (`text_codec.rs`), edited by `apply_string_diff` (common prefix and suffix kept, the
middle deleted and inserted), so **no migration of the string is needed**. Three places read it
as a plain string and would drop anything attached to it: the copy of a node (`objects.rs`
`copy_map`), the restore engine's node value (`history/node_value.rs`, `Node::Text(String)`) and
`history/merge.rs`.

Checked in a scratch crate against `loro =1.16.2` (outside the workspace, deleted):

- `LoroText::mark(range, key, value)` and `unmark(range, key)` work on Unicode scalar ranges; the
  expand behaviour of a key comes from `LoroDoc::config_text_style` and is **stored in the op**
  (`StyleStart { info }`), so a peer or a reopened file without the config honours it. A local
  `mark` on an unconfigured key fails with `StyleConfigMissing` (loud, not silent).
- With `ExpandType::None`, a character inserted directly before or after a marked range gets no
  value; a character inserted **inside** a multi-character marked range takes the value
  (Peritext). Deleting a marked character and typing it again leaves no value.
- Concurrent marks on different characters both survive; on the same character both peers
  converge on one value (op order), as any register.
- Snapshot size, 500-character text: +28 bytes per single-character mark (500 marks: 297 → 11,384
  bytes), +55 bytes for one range mark; unmarking keeps the history (+5.5 bytes per mark).

Options (Frage 42):

| Option | Why it won or lost |
|---|---|
| **A. Loro text marks on `text_string`, `ExpandType::None` (chosen)** | The CRDT moves the value with its character through local edits and merges (criteria 30, 31) with no code of ours; deletion removes it; nothing to collect. Costs: the three plain-string readers above learn marks, and the config must be set before any local mark. |
| B. A list keyed by character offset, remapped on every edit | Our code remaps on local edits, but two peers' offsets disagree after a merge: a kern lands on the wrong character or two. Fails criterion 31. |
| C. A map of records holding Loro stable cursors (`get_cursor`) | Merges per record, but a deleted character leaves an orphan record every reader must skip and someone must collect; a character typed at the cursor's side can be taken for the kerned one; two containers to keep in step. More code for what A gives free. |

## Decision

### 1. What a per-character value is

A value attached to one Unicode scalar value of `text_string`, stored as a Loro text mark on
exactly that character, absent = default, a default never written. Slice 4 defines three keys:

```text
mark key   value                              absent   meaning (criteria 26, 27, 27b)
kern_x     f64, factor of text_size, [-1, 1]   0        extra advance before the character,
                                                        in reading direction (along a column
                                                        in vertical text)
kern_y     f64, factor of text_size, [-2, 2]   0        shift of the character and the rest of
                                                        its line, positive up (across the line:
                                                        to the right of a column); cumulative
                                                        like kern_x and SVG `dy`
rot        f64, degrees, (-180, 180]          0        the character's own turn about its
                                                        cluster's centre, clockwise on screen,
                                                        added to the text's Character rotation
```

Relative to Size for the reason of Frage 32 A; `rot` is an angle and does not scale. All keys are
configured `ExpandType::None` and never with another type (Loro's rule: one expand type per key).
A later key (a glyph alternate, `0082` Frage 40 B, value a string) uses the same mechanism and
takes its own format number; this ADR does not add it. Read rules: a non-finite or non-number
value reads as 0, an out-of-range factor is clamped, a rotation is wrapped into (-180, 180]
(merge residue, never refused).

**Customer decisions 2026-10-24 (feedback on PR #189):** `kern_y` moves the rest of the line too
(it was "the character alone"; criterion 27), and `rot` is added (criterion 27b). Both land in
format 21, which this branch introduces and which no merged build has written, so no further
number is taken. An "All characters" button (criterion 27c) writes one of these values on every
character through §2's write; with 28 bytes per mark that is the cost §2 accepts. The layout applies the value of a **cluster's
first character** (`0074` decision 3); a value on a later character of a cluster, or on `\n`, is
kept and ignored.

### 2. Writing

- **One mark per character.** A value set on a range is written as one `mark(i..i+1)` per
  character whose stored value differs, so a character a peer inserts between two kerned
  characters is at a boundary and gets nothing (a range mark would hand it the value). Clearing
  is one `unmark` over the cleared range. Cost: 28 bytes per kerned character; kerning is a few
  pairs per text, and setting a value on every character of a long text is letter spacing's job.
- **The config is set where the `LoroDoc` is built**, in `Document`'s two constructors
  (`document.rs`, new and from snapshot), with the keys of §1, before any edit. One test per
  constructor marks a character.
- **Local edits reconcile.** The edit session holds the string and its per-character values
  together (ui-core); the commit runs `apply_string_diff` (the CRDT carries existing marks along)
  and then writes the difference between the session's values and the stored ones. So a
  character the session typed has no value even where the diff would have kept a mark, and an
  Alt+arrow change lands on the character the maker saw.

### 3. Merging

Per character and per key: kerns on different characters by two peers both survive; on the same
character and key the later op wins on every peer. A character deleted by one peer while another
kerns it stays deleted (the mark covers nothing). This is ADR 0009 §3's per-character merge,
extended to the values.

### 4. Copy, paste, conversion, split

- **Copying a text object** (duplicate, paste of objects, `copy_map`) copies the string **with**
  its values: the copy writes the marks of the source runs (`to_delta`).
- **Copying characters** in edit mode puts plain text on the clipboard; pasting plain text
  carries no value (criterion 30).
- **Object to path, Weld, Split, Break apart** read the laid-out outlines, which already include
  the kerns (§6), so the result matches the drawing within 0.01 mm and nothing is kept: a path has
  no characters. Split and Break apart convert texts first (`ui-core/src/split.rs`), so they follow
  the same rule.
- **Splitting a text into texts** does not exist today. If a story adds it, each part takes the
  values of its characters (a slice of §5's list), no new decision.
- **Linked clones** read the original's `TextSnapshot` (ADR 0015) and show its kerns.

### 5. Undo and history

`history/node_value.rs` `Node::Text` becomes `Text { string, chars }` (`chars` as in §6). The
restore keeps ADR 0014 §1's three-way rule: the string as today; then, where the live string
equals the restored one, each character's value goes back to its old value only where it is still
the value the step wrote. `history/merge.rs` writes `Node::Text` with its marks. A preview of an
old version reads marks from the live document like everything else.

### 6. Core API shape

- `curvyo-document-core`, new module `text_chars.rs` ("the per-character values of a text and
  their read and write as Loro marks"; `text_codec.rs` is 556 lines and does not grow):
  `pub struct CharValues { pub kern_x: f64, pub kern_y: f64, pub rotation: f64 }` with `DEFAULT`,
  and
  `pub struct TextChars(Vec<(usize, CharValues)>)`: sorted by character index, non-default
  entries only, empty for almost every text. `TextSnapshot` gains `chars: TextChars`
  (`skip_serializing_if` empty, so `document.json` mirrors it and a text without kerns mirrors
  byte for byte as before).
- Document methods: `edit_text_string` becomes `edit_text(id, string, &TextChars)` (the session
  commit, §2), and `set_text_chars(&[(NodeId, TextChars)], label)` writes only differences in one
  commit ("Change kerning", "Clear kerning", "Change per-character rotation", "Clear
  per-character rotation"; each clear leaves the other keys). No method per key.
- `curvyo-text-core`: `LayoutOptions` gains `char_offsets: Vec<CharOffset>` with
  `CharOffset { index: usize, along: Mm, across: Mm, rotation_rad: f64 }` (any order, character
  index into the laid-out string). One mapping in `ui-core::text_layout::layout_options` (factor ×
  size, as `0079` decision 4; degrees to radians). The engine adds `along` to the pen before the
  cluster (before alignment and justify, so a justified line keeps its width) and adds `across` to
  the line's running rise, which moves that cluster and every later one of the line (the next
  line starts at 0); `PlacedGlyph` gains `rotation_rad`, the cluster's own turn, which
  `glyph_paths` adds to the text's turn about the same pivot; caret boundaries sit in the middle
  of the gap, as letter spacing (`0079` decision 5), and the caret stays on the line's baseline. Text-core
  works in `Mm` and knows nothing of marks or factors; a new module `char_offsets.rs` keeps
  `layout.rs` (482 lines) under 500.

### 7. File format

The next free `format_version` at merge (21 if nothing else takes it first): a build of format 20
reads `text_string` with `to_string`, would draw a kerned text unkerned without a word, so it must
refuse the file (ADR 0004 §9). Migration: none; a file without marks reads as before. Golden file
`text_layout_v<N>.curvyo`: one text with `kern_x` on one character, one with `kern_y`, one with
both on several characters, one with `rot` on two characters (one also kerned), one without; test that a format-20 fixture opens, saves and has no
mark.

## Before the build

No separate spike: the scratch check above answered the Loro questions. The plan's first task
is a test-first round of the four risks in document-core: the config on both constructors, marks
surviving `apply_string_diff` on prefix and suffix, undo restoring a kern (§5), and `copy_map`
copying marks.

## Consequences

- Kerns follow their characters through edits, merges and undo with no orphan bookkeeping.
- The text is now rich text in Loro's sense; every reader of `text_string` that needs the values
  must use `to_delta`, not `to_string`. The three places today are named in Context; a review
  checks new ones.
- One more Loro feature (Peritext marks) under the pinned `=1.16.2`; a Loro upgrade must keep the
  golden file and the four tests green.
- A remote insert can never inherit a kern (one mark per character); the price is 28 bytes per
  kerned character in the file.
- Not solved here: glyph alternates (Frage 40 B) need only a new key and a format number; any
  other per-character value (font, size, colour) stays out of scope (`0079` "Out of scope").
