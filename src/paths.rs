//! Paths as the reference prints them: `os.path.expanduser`,
//! `os.path.abspath`, and `pretty` (keep symlinks, fold $HOME to ~).

use std::env;
use std::ffi::OsStr;
#[cfg(windows)]
use std::path::Path;

pub const SEP: char = if cfg!(windows) { '\\' } else { '/' };

/// A path as the user would type it: keep symlinks, fold $HOME to ~.
pub fn pretty(p: &str) -> String {
    let p = abspath(p);
    let home = expanduser("~");

    match p.strip_prefix(&format!("{}{}", home, SEP)) {
        Some(rest) => format!("~{}{}", SEP, rest),
        None => p,
    }
}

/// The path the agent runs this tool by. Python's `__file__` is the script
/// path as invoked, symlinks kept; a native executable reconstructs it from
/// `argv[0]`, searching PATH as the shell did when `argv[0]` is a bare name.
pub fn me(argv0: Option<&OsStr>) -> String {
    let exe = || {
        env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    };

    let found = if cfg!(windows) {
        exe()
    } else {
        argv0.and_then(|a| a.to_str()).and_then(|a| {
            if a.contains('/') {
                Some(a.to_string())
            } else {
                search_path(a)
            }
        })
    };
    pretty(&found.or_else(exe).unwrap_or_else(|| "memo".into()))
}

#[cfg(unix)]
/// Find an executable PATH entry without resolving the symlink spelling.
fn search_path(name: &str) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;

    if name.is_empty() {
        return None;
    }

    let path = env::var("PATH").ok()?;
    path.split(':').find_map(|dir| {
        let dir = if dir.is_empty() { "." } else { dir };
        let cand = format!("{}/{}", dir.trim_end_matches('/'), name);
        let m = std::fs::metadata(&cand).ok()?;
        (m.is_file() && m.permissions().mode() & 0o111 != 0).then_some(cand)
    })
}

#[cfg(not(unix))]
/// Windows uses the module path instead of POSIX shell PATH resolution.
fn search_path(_: &str) -> Option<String> {
    None
}

#[cfg(unix)]
/// Expand tilde prefixes, falling back to the account database when HOME is absent.
pub fn expanduser(path: &str) -> String {
    if !path.starts_with('~') {
        return path.to_string();
    }

    let i = path[1..].find('/').map_or(path.len(), |j| j + 1);
    let home = if i == 1 {
        match env::var("HOME") {
            Ok(h) => Some(h),
            Err(_) => passwd_home(None),
        }
    } else {
        passwd_home(Some(&path[1..i]))
    };

    let Some(home) = home else {
        return path.to_string();
    };

    let r = format!("{}{}", home.trim_end_matches('/'), &path[i..]);

    if r.is_empty() {
        "/".into()
    } else {
        r
    }
}

#[cfg(unix)]
/// Copy a reentrant password lookup's home path before releasing its scratch buffer.
fn passwd_home(name: Option<&str>) -> Option<String> {
    use std::ffi::{CStr, CString};
    let cname = match name {
        Some(n) => Some(CString::new(n).ok()?),
        None => None,
    };

    let mut buf = vec![0 as libc::c_char; 1 << 14];
    // SAFETY: the buffers outlive the calls, and pw_dir is read only when
    // the lookup reported success.
    unsafe {
        let mut pwd: libc::passwd = std::mem::zeroed();
        let mut res: *mut libc::passwd = std::ptr::null_mut();
        let rc = match &cname {
            Some(c) => {
                libc::getpwnam_r(c.as_ptr(), &mut pwd, buf.as_mut_ptr(), buf.len(), &mut res)
            }
            None => libc::getpwuid_r(
                libc::getuid(),
                &mut pwd,
                buf.as_mut_ptr(),
                buf.len(),
                &mut res,
            ),
        };

        if rc != 0 || res.is_null() || pwd.pw_dir.is_null() {
            return None;
        }
        Some(CStr::from_ptr(pwd.pw_dir).to_string_lossy().into_owned())
    }
}

#[cfg(windows)]
/// Reproduce ntpath's current/named-user expansion from the Windows home variables.
pub fn expanduser(path: &str) -> String {
    if !path.starts_with('~') {
        return path.to_string();
    }

    let i = path[1..].find(['/', '\\']).map_or(path.len(), |j| j + 1);
    let home = if let Ok(h) = env::var("USERPROFILE") {
        h
    } else if let Ok(hp) = env::var("HOMEPATH") {
        let drive = env::var("HOMEDRIVE").unwrap_or_default();
        Path::new(&drive).join(hp).to_string_lossy().into_owned()
    } else {
        return path.to_string();
    };

    let home = if i != 1 {
        let target = &path[1..i];

        if env::var("USERNAME").ok().as_deref() == Some(target) {
            home
        } else {
            let hp = Path::new(&home);

            if hp.file_name().and_then(|f| f.to_str()) != env::var("USERNAME").ok().as_deref() {
                return path.to_string();
            }

            match hp.parent() {
                Some(parent) => parent.join(target).to_string_lossy().into_owned(),
                None => return path.to_string(),
            }
        }
    } else {
        home
    };
    format!("{}{}", home, &path[i..])
}

#[cfg(unix)]
/// Make a lexical absolute path while retaining symlinks and POSIX double slashes.
pub fn abspath(p: &str) -> String {
    if p.starts_with('/') {
        return normpath(p);
    }

    match env::current_dir() {
        Ok(cwd) => {
            let cwd = cwd.to_string_lossy();
            let joined = if cwd.ends_with('/') || p.is_empty() {
                format!("{}{}", cwd, p)
            } else {
                format!("{}/{}", cwd, p)
            };
            normpath(&joined)
        }
        Err(_) => normpath(p),
    }
}

#[cfg(windows)]
/// Delegate drive-relative path handling to the platform without requiring existence.
pub fn abspath(p: &str) -> String {
    let p = if p.is_empty() { "." } else { p };
    std::path::absolute(Path::new(p))
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string())
}

/// posixpath.normpath.
#[cfg(unix)]
pub fn normpath(path: &str) -> String {
    if path.is_empty() {
        return ".".into();
    }

    let initial = if path.starts_with("//") && !path.starts_with("///") {
        2
    } else if path.starts_with('/') {
        1
    } else {
        0
    };

    let mut comps: Vec<&str> = Vec::new();

    for c in path.split('/') {
        if c.is_empty() || c == "." {
            continue;
        }

        if c != ".." || (initial == 0 && comps.is_empty()) || comps.last() == Some(&"..") {
            comps.push(c);
        } else if !comps.is_empty() {
            comps.pop();
        }
    }

    let s = format!("{}{}", "/".repeat(initial), comps.join("/"));

    if s.is_empty() {
        ".".into()
    } else {
        s
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(normpath("/a/./b/../c//"), "/a/c");
        assert_eq!(normpath("//a"), "//a");
        assert_eq!(normpath("///a/.."), "/");
        assert_eq!(normpath("a/../.."), "..");
        assert_eq!(normpath(""), ".");
    }

    #[test]
    fn folds_home() {
        let home = expanduser("~");
        assert_eq!(pretty(&format!("{}/x/../y", home)), "~/y");
        assert_eq!(pretty(&home), home);
    }
}
