//! Parity with the reference: identical stdout, stderr, status and store
//! bytes after identical command sequences.

mod common;

use common::{asked, Lcg, Pair};

/// Answer every compression a command asked for, on both sides.
fn settle(p: &Pair, mut out: String) {
    while let Some(id) = asked(&out) {
        let line = format!("summary of {} with é and ✓", id);
        out = p.same(&["nap", &id, &line]).stdout;
    }
}

fn note_n(p: &Pair, n: usize, settle_naps: bool) {
    for i in 0..n {
        let r = p.same(&[
            "note",
            &format!("memory number {} about topic-{}", i, i % 7),
        ]);
        if settle_naps {
            settle(p, r.stdout);
        }
    }
}

#[test]
fn usage_and_unknown_commands() {
    need_python!();
    let p = Pair::new("usage");
    assert_eq!(p.same(&[]).status, 0);
    assert_eq!(p.same(&["frobnicate"]).status, 1);
    for cmd in [
        "wake", "note", "nap", "recall", "zoom", "forget", "config", "import",
    ] {
        let r = p.same(&[cmd, "x"]);
        assert_eq!(r.status, 1, "{} without a store", cmd);
        assert!(r.stderr.starts_with("No memory at "));
    }
}

#[test]
fn init_is_idempotent() {
    need_python!();
    let p = Pair::new("init");
    p.same(&["init", "extra"]);
    p.same(&["init"]);
    p.same(&["init"]);
    p.same(&["note", "first"]);
    p.same(&["init"]);
    p.same_env(&["init"], &[("MEMORY_DIR", "~/other/mem")]);
    p.same_env(&["wake"], &[("MEMORY_DIR", "~/other/mem")]);
    p.same_env(&["wake"], &[("MEMORY_DIR", "")]);
    p.same_env(&["wake"], &[("MEMORY_DIR", "~/missing")]);
}

#[test]
fn note_validation() {
    need_python!();
    let p = Pair::new("note");
    p.same(&["init"]);
    let e = "é".repeat(140);
    let long = format!("{}a", e);
    let cases: Vec<&str> = vec![
        "",
        "   ",
        "\u{3000}\u{1c}",
        "a\nb",
        "a\rb",
        "a\r\nb\nc",
        &e,
        &long,
        "  padded \u{a0} ",
        "\u{1c}lead and trail\u{1f}",
        "tab\tinside",
        "x".repeat(281).leak(),
        "x".repeat(280).leak(),
    ];
    for c in cases {
        p.same(&["note", c]);
    }
    p.same(&["note"]);
    p.same(&["note", "a", "b"]);
    p.same(&["wake"]);
}

#[test]
fn growth_with_naps_and_wake_pages() {
    need_python!();
    let p = Pair::new("growth");
    p.same(&["init"]);
    p.same(&["wake"]);
    p.same(&["nap"]);
    note_n(&p, 40, true);
    p.same(&["config", "WAKE_LINES=12", "PART_LINES=5"]);
    let first = p.same(&["wake"]);
    assert!(first.stdout.contains("part 1 of"));
    for args in [
        ["wake", "2", "40"],
        ["wake", "3", "40"],
        ["wake", "9", "40"],
        ["wake", "1", "41"],
    ] {
        p.same(&args);
    }
    p.same(&["wake", "2"]);
    p.same(&["wake", "0"]);
    p.same(&["wake", "1", "0"]);
    p.same(&["wake", "x"]);
    p.same(&["wake", "1", "2", "3"]);
    p.same(&["wake", "1", "17"]);
    p.same(&["config", "PART_CHARS=300", "PART_LINES="]);
    p.same(&["wake"]);
    note_n(&p, 300, true);
    p.same(&["config", "WAKE_LINES=", "PART_CHARS=", "PART_LINES="]);
    p.same(&["wake"]);
}

#[test]
fn wake_refuses_until_naps_are_done() {
    need_python!();
    let p = Pair::new("refuse");
    p.same(&["init"]);
    p.same(&["config", "WAKE_LINES=4"]);
    note_n(&p, 37, false);
    let r = p.same(&["wake"]);
    assert_eq!(r.status, 1);
    assert!(r.stdout.starts_with("Cannot wake"));
    // Out-of-order and malformed nap ids.
    for id in [
        "4-5", "5-6", "0-0", "1-2", "x-1", "0-1x", "00-01", "0-3", "2-3", "-1-0",
    ] {
        p.same(&["nap", id, "some summary"]);
    }
    p.same(&["nap", "0-1"]);
    p.same(&["nap", "0-1", ""]);
    p.same(&["nap", "0-1", "two\nlines"]);
    settle(&p, p.same(&["nap"]).stdout);
    p.same(&["nap", "0-1", "again"]);
    p.same(&["nap", "0-1", "again"]);
    p.same(&["wake"]);
    note_n(&p, 3, false);
    p.same(&["nap", "0-1", "again"]);
    p.same(&["wake"]);
}

#[test]
fn zoom_and_forget() {
    need_python!();
    let p = Pair::new("zoom");
    p.same(&["init"]);
    note_n(&p, 70, true);
    for id in [
        "0-63", "0-31", "32-63", "64-127", "64-65", "68-69", "70-71", "0-1", "128-255", "3-4", "x",
        "0-0",
    ] {
        p.same(&["zoom", id]);
    }
    p.same(&["zoom"]);
    p.same(&["forget"]);
    p.same(&["forget", "68-69"]);
    p.same(&["forget", "8-15"]);
    p.same(&["zoom", "0-63"]);
    p.same(&["zoom", "8-15"]);
    p.same(&["forget", "8-15"]);
    p.same(&["wake"]);
    settle(&p, p.same(&["nap"]).stdout);
    p.same(&["forget", "0-1"]);
    p.same(&["nap", "4-5", "early"]);
    settle(&p, p.same(&["nap"]).stdout);
    p.same(&["wake"]);
}

#[test]
fn recall_matches_and_caps() {
    need_python!();
    let p = Pair::new("recall");
    p.same(&["init"]);
    p.same(&["recall", "anything"]);
    note_n(&p, 120, true);
    p.same(&["note", "Ünïcödé ÉLIXIR straße ΣΊΣΥΦΟΣ"]);
    for q in [
        "topic-3",
        "TOPIC-3",
        "absent",
        ".",
        "^#1[0-9] ",
        "number (1|22|99) ",
        "élixir",
        "ünïcödé",
        "\\d{3}",
        "^#5 ",
        "Σίσυφος",
        "\\bnumber\\b",
        "",
    ] {
        p.same(&["recall", q]);
    }
    p.same(&["recall"]);
    p.same(&["recall", "a", "b"]);
    p.same(&["config", "PART_CHARS=200"]);
    p.same(&["recall", "topic"]);
    p.same(&["config", "PART_CHARS=10"]);
    p.same(&["recall", "topic"]);
}

#[test]
fn config_files_and_arguments() {
    need_python!();
    let p = Pair::new("config");
    p.same(&["init"]);
    p.same(&["config"]);
    for a in [
        "WAKE_LINES=10",
        "wake_lines=11",
        " part_chars = 900 ",
        "ENTRY_CHARS=280",
        "ENTRY_CHARS=281",
        "ENTRY_CHARS=0",
        "ENTRY_CHARS=-1",
        "ENTRY_CHARS=1.5",
        "ENTRY_CHARS=",
        "NOPE=1",
        "WAKE_LINES",
        "=",
        "PART_LINES=007",
        "PART_LINES=x",
    ] {
        p.same(&["config", a]);
    }
    p.same(&["config", "WAKE_LINES=5", "PART_LINES=", "ENTRY_CHARS=20"]);
    p.same(&["note", "this note is longer than twenty bytes"]);
    p.same(&["note", "short"]);
    let files: [&[u8]; 7] = [
        b"WAKE_LINES = 3\r\nPART_LINES=2\rENTRY_CHARS=9 # comment\n",
        b"# only comments\n\n   \n",
        b"wake_lines=4\nbogus line without equals\n",
        b"\xef\xbb\xbfWAKE_LINES=4\n",
        b"NOPE=3\n",
        b"PART_CHARS=0\n",
        b"PART_CHARS=12#x=y\nENTRY_CHARS = 999\n",
    ];
    for f in files {
        p.file(".optmem/memory/config", f);
        p.same(&["config"]);
        p.same(&["wake"]);
    }
    p.file(".optmem/memory/config", b"");
    p.same(&["config", "WAKE_LINES=7"]);
}

#[test]
fn import_files() {
    need_python!();
    let p = Pair::new("import");
    p.same(&["init"]);
    let files: [(&str, &[u8]); 11] = [
        ("bad-shape.txt", b"2026-1-01 text\n"),
        ("bad-date.txt", b"2026-02-30 text\n"),
        ("year0.txt", b"0000-01-01 text\n"),
        ("empty-text.txt", b"2026-01-01   \n"),
        ("only-blank.txt", b"\n  \n\r\n"),
        ("latin1.txt", b"2026-01-01 caf\xe9\n"),
        ("too-long.txt", b"2026-01-01 xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n"),
        ("order.txt", b"2026-01-02 b\n2026-01-01 a\n"),
        ("good.txt", b"2020-01-01 first \xc3\xa9\r\n\r\n2020-01-01   second  \r2020-02-29 leap\n2021-05-05 \xe2\x9c\x93 last"),
        ("tab.txt", b"2021-05-05\ttab\n"),
        ("good2.txt", b"2021-06-01 more\n"),
    ];
    for (name, bytes) in files {
        p.file(name, bytes);
        p.same(&["import", name]);
    }
    p.file("older.txt", b"2019-01-01 old\n");
    p.same(&["import", "older.txt"]);
    p.same(&["import", "missing.txt"]);
    p.same(&["import", "."]);
    p.same(&["import"]);
    p.same(&["wake"]);
    settle(&p, p.same(&["nap"]).stdout);
    let many: String = (0..50)
        .map(|i| format!("2022-01-{:02} bulk {}\n", 1 + i % 28, i))
        .collect();
    let many = format!(
        "2022-01-01 start\n{}",
        many.lines()
            .map(|l| format!("2022-02-01{}\n", &l[10..]))
            .collect::<String>()
    );
    p.file("many.txt", many.as_bytes());
    p.same(&["import", "many.txt"]);
    p.same(&["wake"]);
}

#[test]
fn partial_trailing_log_record_is_repaired() {
    need_python!();
    let p = Pair::new("repair");
    p.same(&["init"]);
    note_n(&p, 5, true);
    for s in [&p.py, &p.rs] {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(s.mem().join("LOG.txt"))
            .unwrap();
        f.write_all(b"#5 2026-01-01 torn wri").unwrap();
    }
    p.same(&["wake"]);
    p.same(&["recall", "torn"]);
    p.same(&["note", "after the crash"]);
    p.same(&["wake"]);
}

#[test]
fn fuzzed_sequences() {
    need_python!();
    for seed in [1u64, 7, 42] {
        let p = Pair::new("fuzz");
        let mut g = Lcg(seed);
        p.same(&["init"]);
        let mut pending: Option<String> = None;
        for step in 0..150 {
            let r = match g.below(20) {
                0..=8 => p.same(&[
                    "note",
                    &format!("s{} step {} word{}", seed, step, g.below(9)),
                ]),
                9..=11 => match pending.clone() {
                    Some(id) if g.below(4) > 0 => {
                        p.same(&["nap", &id, &format!("sum {} at {}", id, step)])
                    }
                    _ => p.same(&["nap"]),
                },
                12 => p.same(&["wake"]),
                13 => {
                    let k = (1 + g.below(3)).to_string();
                    p.same(&["wake", &k])
                }
                14 => p.same(&["recall", &format!("word{}", g.below(9))]),
                15 => {
                    let s = 1u64 << (1 + g.below(4));
                    let lo = g.below(40) / s * s;
                    p.same(&["zoom", &format!("{}-{}", lo, lo + s - 1)])
                }
                16 => {
                    let s = 1u64 << (1 + g.below(3));
                    let lo = g.below(40) / s * s;
                    p.same(&["forget", &format!("{}-{}", lo, lo + s - 1)])
                }
                17 => {
                    let v = [
                        "WAKE_LINES=6",
                        "WAKE_LINES=",
                        "PART_LINES=4",
                        "PART_LINES=",
                        "PART_CHARS=150",
                    ][g.below(5) as usize];
                    p.same(&["config", v])
                }
                _ => p.same(&["nap", "0-1", "maybe settled"]),
            };
            if let Some(id) = asked(&r.stdout) {
                pending = Some(id);
            }
        }
    }
}
