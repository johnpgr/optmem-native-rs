//! Per-invocation state shared by command handlers.
//!
//! Configuration is loaded once before ordinary commands run. Opening a store
//! repairs missing layout files but never creates a missing identity; only
//! `init` may create the memory directory. Paths retain their invoked spelling.

use std::ffi::OsStr;
use std::path::Path;

use crate::config::{self, Config};
use crate::fail::{die, R};
use crate::output::Output;
use crate::{paths, store};

/// All mutable state belongs to this invocation rather than global variables.
pub struct App {
    /// Home-abbreviated executable path used in agent-facing instructions.
    pub executable: String,
    /// Expanded store path, retaining the spelling used in missing-store errors.
    pub directory: String,
    pub config: Config,
    pub output: Output,
}

impl App {
    /// Resolve environment defaults without opening or creating a memory store.
    pub fn new(invocation: Option<&OsStr>) -> Self {
        Self {
            executable: paths::me(invocation),
            directory: memory_directory(),
            config: Config::default(),
            output: Output::default(),
        }
    }

    /// Borrow a filesystem path without changing its spelling or resolving symlinks.
    pub fn path(&self) -> &Path {
        Path::new(&self.directory)
    }

    /// Substitute the exact help template without adding a second trailing newline.
    pub fn usage(&self) -> String {
        include_str!("usage.txt").replace("{memo}", &self.executable)
    }

    /// Require an existing identity, restore missing files, and validate its config.
    pub fn open(&mut self) -> R<()> {
        if !self.path().is_dir() {
            return die(format!(
                "No memory at {}.\nTo create one, run: {} init\nTo use an existing one, point MEMORY_DIR at it.",
                self.directory, self.executable
            ));
        }

        store::makedirs(&self.path().join("TREE"))?;

        let log = store::log_path(self.path());

        if !log.exists() {
            store::touch(&log)?;
        }

        self.config = config::load(self.path())?;

        Ok(())
    }
}

/// An empty override follows the default store, matching the reference CLI.
pub fn memory_directory() -> String {
    match std::env::var("MEMORY_DIR") {
        Ok(path) if !path.is_empty() => paths::expanduser(&path),
        _ => paths::expanduser("~/.optmem/memory"),
    }
}
