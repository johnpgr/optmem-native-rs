//! Parallel writers, native and Python mixed on ONE store: every
//! acknowledged note gets a unique id, records stay aligned, and exactly one
//! of several racing naps lands.

mod common;

use common::{Pair, Side};
use std::collections::BTreeSet;
use std::fs;
use std::process::Child;

fn spawn(side: &Side, mem: &std::path::Path, args: &[&str]) -> Child {
    side.command(args, &[("MEMORY_DIR", mem.to_str().unwrap())])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

fn sides(p: &Pair) -> Vec<&Side> {
    if common::python_available() {
        vec![&p.rs, &p.py]
    } else {
        vec![&p.rs]
    }
}

#[test]
fn mixed_writers_get_unique_ids() {
    let p = Pair::new("race");
    let mem = p.root.join("shared");
    let init =
        p.rs.command(&["init"], &[("MEMORY_DIR", mem.to_str().unwrap())])
            .output()
            .unwrap();
    assert!(init.status.success());
    let sides = sides(&p);
    let n = 48;
    let kids: Vec<(String, Child)> = (0..n)
        .map(|i| {
            let text = format!("writer {} of {}", i, n);
            let child = spawn(sides[i % sides.len()], &mem, &["note", &text]);
            (text, child)
        })
        .collect();
    let mut ids = BTreeSet::new();
    for (_, k) in kids {
        let out = k.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let s = String::from_utf8(out.stdout).unwrap();
        let id: u64 = s
            .strip_prefix("Saved as #")
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(ids.insert(id), "id {} handed out twice", id);
    }
    assert_eq!(ids, (0..n as u64).collect());
    let log = fs::read(mem.join("LOG.txt")).unwrap();
    assert_eq!(log.len(), n * 320);
    let mut texts = BTreeSet::new();
    for (i, rec) in log.chunks(320).enumerate() {
        assert_eq!(rec[319], b'\n');
        let line = std::str::from_utf8(rec).unwrap().trim_end();
        let rest = line
            .strip_prefix(&format!("#{} ", i))
            .expect("ids in file order");
        texts.insert(rest.split_once(' ').unwrap().1.to_string());
    }
    assert_eq!(texts.len(), n);
}

#[test]
fn racing_naps_land_once() {
    let p = Pair::new("naprace");
    let mem = p.root.join("shared");
    let env = [("MEMORY_DIR", mem.to_str().unwrap())];
    assert!(p
        .rs
        .command(&["init"], &env)
        .output()
        .unwrap()
        .status
        .success());
    for i in 0..2 {
        assert!(p
            .rs
            .command(&["note", &format!("n{}", i)], &env)
            .output()
            .unwrap()
            .status
            .success());
    }
    let sides = sides(&p);
    let kids: Vec<Child> = (0..12)
        .map(|i| {
            spawn(
                sides[i % sides.len()],
                &mem,
                &["nap", "0-1", &format!("summary {}", i)],
            )
        })
        .collect();
    let mut saved = 0;
    for k in kids {
        let out = k.wait_with_output().unwrap();
        let s = String::from_utf8(out.stdout).unwrap();
        if s.starts_with("0-1 saved.") {
            saved += 1;
        } else {
            assert!(
                s.starts_with("0-1 is already settled.")
                    || s.starts_with("0-1 was settled or forgotten meanwhile.")
                    || s.starts_with("Nothing left to compress."),
                "{}",
                s
            );
        }
    }
    assert_eq!(saved, 1);
    assert_eq!(fs::metadata(mem.join("TREE/2")).unwrap().len(), 288);
}
