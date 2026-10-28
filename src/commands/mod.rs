//! Command routing only; handlers own validation, work, and buffered responses.
//!
//! Shared compression planning, text validation, and physical storage live
//! outside this module so commands do not depend on one another's internals.

mod config;
mod import;
mod init;
mod nap;
mod note;
mod recall;
mod tree;
mod wake;

use crate::app::App;
use crate::fail::R;

/// Uniform handler boundary used after CLI argument decoding and store setup.
pub type Handler = fn(&mut App, &[String]) -> R<()>;

/// Resolve exactly the nine reference commands without touching the filesystem.
pub fn resolve(name: &str) -> Option<Handler> {
    match name {
        "init" => Some(init::execute),
        "wake" => Some(wake::execute),
        "note" => Some(note::execute),
        "nap" => Some(nap::execute),
        "recall" => Some(recall::execute),
        "zoom" => Some(tree::zoom),
        "forget" => Some(tree::forget),
        "config" => Some(config::execute),
        "import" => Some(import::execute),
        _ => None,
    }
}
