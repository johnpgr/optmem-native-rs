//! Native OptMem CLI. Command behavior, storage, and process I/O have separate owners.
//!
//! See the README architecture map and compatibility notes before changing the
//! fixed-record format, output contract, or mixed Python/native lock behavior.

mod app;
mod cli;
mod commands;
mod compression;
mod config;
mod cover;
mod fail;
mod output;
mod pagination;
mod paths;
mod pystr;
mod store;
mod text;
mod today;
mod validation;

/// Let command-local files and lock guards drop before terminating the process.
fn main() {
    std::process::exit(cli::run());
}
