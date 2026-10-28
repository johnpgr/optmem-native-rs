//! The deliberate differences from the reference, each pinned down. README
//! lists them; none of these cases is a parity test.

mod common;

use common::Pair;
use std::fs;

fn store_with_notes(name: &str, n: usize) -> Pair {
    let p = Pair::new(name);
    p.same(&["init"]);
    for i in 0..n {
        p.same(&["note", &format!("note {} aaa bbb", i)]);
    }
    p
}

#[test]
fn regex_without_lookaround_or_backreferences() {
    need_python!();
    let p = store_with_notes("regex", 3);
    for q in ["note (?=1)", "(a)\\1", "(?<!x)aaa", "(?<=note )1"] {
        let py = p.py.run(&["recall", q], &[]);
        let rs = p.rs.run(&["recall", q], &[]);
        assert_eq!(py.status, 0, "python accepts {}", q);
        assert_eq!(rs.status, 1, "native rejects {}", q);
        assert!(rs.stderr.starts_with("bad regex: "), "{}", rs.stderr);
        assert!(rs.stdout.is_empty());
    }
}

#[test]
fn regex_syntax_that_means_something_else() {
    need_python!();
    let p = store_with_notes("posix", 2);
    // Rust reads a POSIX class; Python reads a set followed by `]`.
    let rs = p.rs.run(&["recall", "[[:digit:]] aaa"], &[]);
    assert_eq!(
        rs.stdout,
        "#0 DATE note 0 aaa bbb\n#1 DATE note 1 aaa bbb\n2 matches.\n".replace("DATE", &today(&p))
    );
    let py = p.py.run(&["recall", "[[:digit:]] aaa"], &[]);
    assert_ne!(py, rs);
}

fn today(p: &Pair) -> String {
    let log = fs::read_to_string(p.rs.mem().join("LOG.txt")).unwrap();
    log.split(' ').nth(1).unwrap().to_string()
}

#[test]
fn non_utf8_config_is_an_error_not_a_traceback() {
    need_python!();
    let p = store_with_notes("cfg", 0);
    p.file(".optmem/memory/config", b"\xff\xfe");
    let rs = p.rs.run(&["wake"], &[]);
    assert_eq!(
        (rs.status, rs.stderr.as_str()),
        (1, "~/.optmem/memory/config is not UTF-8 text.\n")
    );
    assert!(p
        .py
        .run(&["wake"], &[])
        .stderr
        .contains("UnicodeDecodeError"));
}

#[test]
fn corrupt_log_record_is_an_error_not_a_traceback() {
    need_python!();
    let p = store_with_notes("corrupt", 3);
    for s in [&p.py, &p.rs] {
        let path = s.mem().join("LOG.txt");
        let mut b = fs::read(&path).unwrap();
        b[320 + 5] = 0xff;
        fs::write(&path, b).unwrap();
    }
    let rs = p.rs.run(&["recall", "x"], &[]);
    assert_eq!(
        (rs.status, rs.stderr.as_str()),
        (1, "Memory #1 in LOG.txt is corrupt.\n")
    );
    assert!(p
        .py
        .run(&["recall", "x"], &[])
        .stderr
        .contains("UnicodeDecodeError"));
}

#[test]
fn torn_summary_record_is_not_a_summary() {
    need_python!();
    let p = store_with_notes("torn", 4);
    // The reference prints whatever bytes a crash left; native treats an
    // incomplete record as not built, so nap repairs and rebuilds it.
    p.same(&["nap", "0-1", "first pair"]);
    for s in [&p.py, &p.rs] {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .append(true)
            .open(s.mem().join("TREE/2"))
            .unwrap();
        f.write_all(b"half a summ").unwrap();
    }
    let rs = p.rs.run(&["zoom", "0-3"], &[]);
    assert!(
        rs.stdout.ends_with("#2-3 not compressed yet\n"),
        "{}",
        rs.stdout
    );
    assert!(p
        .py
        .run(&["zoom", "0-3"], &[])
        .stdout
        .ends_with("#2-3 half a summ\n"));
    let rs = p.rs.run(&["nap", "2-3", "second pair"], &[]);
    assert!(rs.stdout.starts_with("2-3 saved."), "{}", rs.stdout);
    assert_eq!(
        fs::metadata(p.rs.mem().join("TREE/2")).unwrap().len(),
        2 * 288
    );
}

#[test]
fn only_ascii_digits_in_numbers() {
    need_python!();
    let p = store_with_notes("digits", 4);
    let rs = p.rs.run(&["zoom", "\u{660}-\u{663}"], &[]);
    assert_eq!(
        rs.stderr,
        "'\u{660}-\u{663}' is not a block id. Copy it from the prompt.\n"
    );
    assert_eq!(p.py.run(&["zoom", "\u{660}-\u{663}"], &[]).status, 0);
}
