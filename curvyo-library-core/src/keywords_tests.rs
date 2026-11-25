use std::io::Write;

use super::*;

fn leb(out: &mut Vec<u8>, mut n: u64) {
    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn text(out: &mut Vec<u8>, s: &str) {
    leb(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// Words: 0 "find", 1 "magnifier", 2 "system", 3 "Bin" (re-folded), 4 "trash".
/// Tabler: `search` (filled too), `trash`; MDI: `magnify`. Related: magnifier → find.
fn sample() -> Vec<u8> {
    let mut out = b"CVYK\x01".to_vec();
    leb(&mut out, 2);
    for (prefix, variants) in [("tabler", vec!["filled"]), ("mdi", vec![])] {
        for field in [
            prefix,
            "Project",
            "package",
            "1.0.0",
            "MIT",
            "https://example.org",
        ] {
            text(&mut out, field);
        }
        leb(&mut out, variants.len() as u64);
        for v in variants {
            text(&mut out, v);
        }
    }
    leb(&mut out, 5);
    for w in ["find", "magnifier", "system", "Bin", "trash"] {
        text(&mut out, w);
    }
    // tabler: "search" (mask 1, keywords 0 1, category 2), "trash" (shares "", keywords 3)
    leb(&mut out, 2);
    for (shared, rest, mask, kws, cats) in [
        (0, "search", 1, vec![0, 1], vec![2]),
        (0, "trash", 0, vec![3], vec![]),
    ] {
        leb(&mut out, shared);
        text(&mut out, rest);
        leb(&mut out, mask);
        leb(&mut out, kws.len() as u64);
        for k in &kws {
            leb(&mut out, *k);
        }
        leb(&mut out, cats.len() as u64);
        for k in &cats {
            leb(&mut out, *k);
        }
    }
    leb(&mut out, 1);
    leb(&mut out, 0);
    text(&mut out, "magnify");
    leb(&mut out, 0);
    leb(&mut out, 1);
    leb(&mut out, 0);
    leb(&mut out, 0);
    leb(&mut out, 1);
    leb(&mut out, 1);
    leb(&mut out, 1);
    leb(&mut out, 0);
    out
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gz.write_all(data).unwrap();
    gz.finish().unwrap()
}

#[test]
fn plain_and_gzip_decode_to_the_same_words() {
    for bytes in [sample(), gzip(&sample())] {
        let k = Keywords::decode(&bytes).unwrap();
        assert_eq!(k.sources().len(), 2);
        assert_eq!(k.sources()[0].variants, ["filled"]);
        let search = k.of("tabler", "search").unwrap();
        assert_eq!(search.keywords().collect::<Vec<_>>(), ["find", "magnifier"]);
        assert_eq!(search.categories().collect::<Vec<_>>(), ["system"]);
        assert_eq!(
            k.of("tabler", "trash")
                .unwrap()
                .keywords()
                .collect::<Vec<_>>(),
            ["bin"],
            "re-folded"
        );
        let related: Vec<(&str, Vec<&str>)> = k.related().map(|(h, r)| (h, r.collect())).collect();
        assert_eq!(related, [("magnifier", vec!["find"])]);
    }
}

#[test]
fn variant_names_share_the_words_and_unknown_names_have_none() {
    let k = Keywords::decode(&sample()).unwrap();
    assert!(
        k.of("tabler", "search-filled").is_some(),
        "the filled variant"
    );
    assert!(k.of("tabler", "trash-filled").is_none(), "no filled trash");
    assert!(k.of("tabler", "searc").is_none());
    assert!(k.of("mdi", "magnify").is_some());
    assert!(k.of("mdi", "search").is_none(), "keywords never cross sets");
    assert!(
        k.of("lucide", "search").is_none(),
        "a source the file lacks"
    );
}

#[test]
fn damaged_files_are_errors_never_panics() {
    let good = sample();
    for cut in 0..good.len() {
        assert!(Keywords::decode(&good[..cut]).is_err(), "cut at {cut}");
    }
    let mut extra = good.clone();
    extra.push(0);
    assert_eq!(
        Keywords::decode(&extra).unwrap_err(),
        KeywordsError::Damaged
    );
    let mut magic = good.clone();
    magic[0] = b'X';
    assert_eq!(
        Keywords::decode(&magic).unwrap_err(),
        KeywordsError::NotKeywords
    );
    let mut format = good.clone();
    format[4] = 2;
    assert_eq!(
        Keywords::decode(&format).unwrap_err(),
        KeywordsError::NotKeywords
    );
    // Every single byte flipped: an error or a file, never a panic.
    for at in 5..good.len() {
        let mut bad = good.clone();
        bad[at] ^= 0xff;
        let _ = Keywords::decode(&bad);
    }
    let broken_gzip = [0x1f, 0x8b, 8, 0, 0, 0];
    assert_eq!(
        Keywords::decode(&broken_gzip).unwrap_err(),
        KeywordsError::Inflate
    );
}

#[test]
fn an_unsorted_name_list_or_a_word_index_past_the_end_is_damaged() {
    let good = sample();
    // "search" then "trash": swap the first letters to make "trash" < "search" false-ordered.
    let at = good.windows(6).position(|w| w == b"search").unwrap();
    let mut unsorted = good.clone();
    unsorted[at] = b'z';
    assert_eq!(
        Keywords::decode(&unsorted).unwrap_err(),
        KeywordsError::Damaged
    );
    let mut past = good.clone();
    // The last related index (1, "magnifier"'s "find" is 0): point it at word 9.
    let last = past.len() - 1;
    past[last] = 9;
    assert_eq!(Keywords::decode(&past).unwrap_err(), KeywordsError::Damaged);
}

#[test]
fn more_than_4_mib_inflated_is_refused() {
    let big = vec![0u8; MAX_KEYWORDS_BYTES + 1];
    assert_eq!(
        Keywords::decode(&gzip(&big)).unwrap_err(),
        KeywordsError::TooLarge
    );
    assert_eq!(Keywords::decode(&big).unwrap_err(), KeywordsError::TooLarge);
}

#[test]
fn a_varint_past_64_bits_is_damaged() {
    let mut r = Reader {
        data: &[0xff; 10],
        at: 0,
    };
    assert_eq!(r.num().unwrap_err(), KeywordsError::Damaged);
    let mut max = Reader {
        data: &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
        at: 0,
    };
    assert_eq!(max.num().unwrap(), u64::MAX);
    let mut bad = b"CVYK\x01".to_vec();
    bad.extend([0xff; 10]);
    assert_eq!(Keywords::decode(&bad).unwrap_err(), KeywordsError::Damaged);
}

#[test]
fn a_huge_icon_count_reserves_nothing_it_cannot_read() {
    // One source without variants, no words, then an icon count near the data's length.
    let mut out = b"CVYK\x01".to_vec();
    leb(&mut out, 1);
    for field in ["p", "P", "p", "1", "MIT", "u"] {
        text(&mut out, field);
    }
    leb(&mut out, 0);
    leb(&mut out, 0);
    leb(&mut out, 40);
    out.extend([0u8; 40]);
    assert_eq!(Keywords::decode(&out).unwrap_err(), KeywordsError::Damaged);
}
