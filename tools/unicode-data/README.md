# Unicode data for the glyph table

`generate.py` writes the two generated files of `specs/0082-glyph-table` (`adrs.md` decision 2):

| Output | What |
|---|---|
| `frontend/public/unicode/names-17.0.0.bin` | The character names and abbreviation aliases, fetched by the frontend on the first switch to Glyphs mode and decoded by `curvyo-text-core/src/unicode_names.rs` |
| `curvyo-text-core/src/unicode_blocks.rs` | The block table (`block_of`) |

Inputs: Unicode 17.0.0 from <https://www.unicode.org/Public/17.0.0/ucd/>, licence
[Unicode-3.0](../../frontend/public/unicode/LICENSE) (shipped next to the names file).

| File | SHA-256 |
|---|---|
| `UnicodeData.txt` | `2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c` |
| `Blocks.txt` | `c0edefaf1a19771e830a82735472716af6bf3c3975f6c2a23ffbe2580fbbcb15` |
| `NameAliases.txt` | `793f6f1e4d15fd90f05ae66460191dc4d75d1fea90136a25f30dd6a4cb950eac` |

The script refuses an input with another checksum, so a re-run gives the committed bytes.

```text
mkdir ucd && cd ucd
for f in UnicodeData.txt Blocks.txt NameAliases.txt; do
  curl -sSfLO https://www.unicode.org/Public/17.0.0/ucd/$f
done
python3 -I ../tools/unicode-data/generate.py .
```

## Names file format 1

All integers are unsigned LEB128 varints; strings are a varint length and ASCII bytes.

1. `CVYN`, the format byte `1`, the Unicode version as three bytes (17, 0, 0).
2. The word dictionary: a count, then each word (most frequent first).
3. Prefix ranges (Unicode name rule NR2): a count, then per range its first code point, its
   length less one and the prefix; the name is the prefix and the code point in hex.
4. Explicit names: a count, then per name the code point's distance to the previous one (to 0
   for the first), the word count and the word indices; words are joined by one space.
5. Abbreviation aliases (`NameAliases.txt`, type `abbreviation`, the last one per code point):
   a count, then per alias the code point distance and the alias.

Hangul syllables (name rule NR1) are computed by the decoder and not stored.
