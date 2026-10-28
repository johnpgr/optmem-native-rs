//! Run the reference Python `memo` and the native one side by side.
//!
//! Each implementation gets its own disposable HOME holding its executable at
//! `~/.optmem/memo` and its store at `~/.optmem/memory`, so both print the
//! same paths and no output has to be rewritten except the HOME itself.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const REFERENCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/reference/memo.py");
pub const NATIVE: &str = env!("CARGO_BIN_EXE_memo");

static SEQ: AtomicUsize = AtomicUsize::new(0);

/// The interpreter for the reference: $OPTMEM_PYTHON, else `python3`.
pub fn python() -> String {
    std::env::var("OPTMEM_PYTHON").unwrap_or_else(|_| "python3".into())
}

pub fn python_available() -> bool {
    Command::new(python())
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
        && Path::new(REFERENCE).exists()
}

/// Skip a differential test where the reference cannot run.
#[macro_export]
macro_rules! need_python {
    () => {
        if !common::python_available() {
            eprintln!("skipped: python3 or reference/memo.py unavailable");
            return;
        }
    };
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Run {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct Side {
    pub home: PathBuf,
    pub exe: PathBuf,
}

impl Side {
    fn new(root: &Path, name: &str, src: &str) -> Side {
        let home = root.join(name);
        fs::create_dir_all(home.join(".optmem")).unwrap();
        let native = !src.ends_with(".py");
        let name = if native && cfg!(windows) {
            "memo.exe"
        } else {
            "memo"
        };
        let exe = home.join(".optmem").join(name);
        fs::copy(src, &exe).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Side { home, exe }
    }

    pub fn mem(&self) -> PathBuf {
        self.home.join(".optmem").join("memory")
    }

    pub fn command(&self, args: &[&str], env: &[(&str, &str)]) -> Command {
        let mut cmd = if cfg!(windows) && is_python(&self.exe) {
            let mut c = Command::new(python());
            c.arg(&self.exe);
            c
        } else {
            Command::new(&self.exe)
        };
        cmd.args(args)
            .current_dir(&self.home)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove("MEMORY_DIR")
            .envs(env.iter().copied());
        cmd
    }

    pub fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Run {
        let out: Output = self.command(args, env).output().unwrap();
        let norm = |b: &[u8]| {
            // Windows: CRLF text mode, and the executable is memo.exe.
            let s = String::from_utf8_lossy(b).replace("\r\n", "\n");
            let s = s.replace("memo.exe", "memo");
            let h = self.home.to_string_lossy().to_string();
            let s = s.replace(&format!("/private{}", h), "<HOME>");
            s.replace(&h, "<HOME>")
        };
        Run {
            status: out.status.code().unwrap_or(-1),
            stdout: norm(&out.stdout),
            stderr: norm(&out.stderr),
        }
    }
}

fn is_python(p: &Path) -> bool {
    fs::read(p).is_ok_and(|b| b.starts_with(b"#!"))
}

pub struct Pair {
    pub root: PathBuf,
    pub py: Side,
    pub rs: Side,
}

impl Pair {
    pub fn new(name: &str) -> Pair {
        let root = std::env::temp_dir().join(format!(
            "optmem-diff-{}-{}-{}",
            std::process::id(),
            name,
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Pair {
            py: Side::new(&root, "py", REFERENCE),
            rs: Side::new(&root, "rs", NATIVE),
            root,
        }
    }

    /// Run on both; require identical results; return them.
    pub fn same(&self, args: &[&str]) -> Run {
        self.same_env(args, &[])
    }

    pub fn same_env(&self, args: &[&str], env: &[(&str, &str)]) -> Run {
        let a = self.py.run(args, env);
        let b = self.rs.run(args, env);
        assert_eq!(
            a, b,
            "\nargs: {:?}\n--- python ---\n{}{}\n--- native ---\n{}{}",
            args, a.stdout, a.stderr, b.stdout, b.stderr
        );
        self.same_store();
        b
    }

    /// Write the same file into both homes.
    pub fn file(&self, rel: &str, bytes: &[u8]) {
        for s in [&self.py, &self.rs] {
            let p = s.home.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        }
    }

    /// Both stores hold byte-identical files (the lock file aside).
    pub fn same_store(&self) {
        let a = snapshot(&self.py.mem());
        let b = snapshot(&self.rs.mem());
        let names = |v: &Vec<(String, Vec<u8>)>| v.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
        assert_eq!(names(&a), names(&b), "store file sets differ");
        for ((n, x), (_, y)) in a.iter().zip(&b) {
            assert!(
                x == y,
                "{} differs:\n{:?}\n{:?}",
                n,
                String::from_utf8_lossy(x),
                String::from_utf8_lossy(y)
            );
        }
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

pub fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel == ".lock" {
                continue;
            }
            if p.is_dir() {
                out.push((rel + "/", Vec::new()));
                walk(base, &p, out);
            } else {
                out.push((rel, fs::read(&p).unwrap()));
            }
        }
    }
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// The block id a nap prompt asks for, parsed from its `Run:` line.
pub fn asked(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .filter(|l| l.starts_with("Run: "))
        .filter_map(|l| l.split_once(" nap ").map(|x| x.1))
        .filter_map(|r| r.split(' ').next())
        .next_back()
        .map(str::to_string)
}

/// A tiny deterministic PRNG, so fuzz runs reproduce from their seed.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}
