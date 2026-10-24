NotoSansJP-Subset.otf: a 3.5 KB subset of Noto Sans JP Regular
(https://github.com/notofonts/noto-cjk, Sans/SubsetOTF/JP), SIL Open Font License 1.1 (OFL.txt).
Made with fontTools:
  pyftsubset NotoSansJP-Regular.otf --text="ABHIxy 日本語、。「」ー（）" \
    --layout-features='vert,vrt2,kern,liga,calt' --no-hinting --desubroutinize
It has vhea, vmtx, VORG and the vert/vrt2 features; used by the vertical layout tests
(specs/0079-text-layout-controls slice 3).
