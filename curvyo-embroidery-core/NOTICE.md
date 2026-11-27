# Notice: curvyo-embroidery-core

This crate is Curvyo's own code under AGPL-3.0-or-later, with two functions transliterated from
**pyembroidery** (https://github.com/EmbroidePy/pyembroidery), pinned at commit
`c16d1b46390a1632ee36ff98eb8b3ec01934a8e9`, read on 2026-11-27:

- `src/dst_write.rs`: `encode_step` and the trim sequence from `pyembroidery/DstWriter.py`
  (`encode_record`, `write`); changed: the stitch list is Y up, so the Y flip is left out, and a
  step past 121 tenths is an error instead of an exception.
- `src/dst_read.rs`: `decode` and the record classes from `pyembroidery/DstReader.py`
  (`decode_dx`, `decode_dy`, `dst_read_stitches`); changed: Y up, unflipped, and exactly the
  writer's trim sequence is read as a trim instead of pyembroidery's trim interpolation.

pyembroidery's licence:

```text
MIT License

Copyright (c) 2018

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
