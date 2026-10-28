//! Store creation and the exact agent onboarding instructions.

use crate::app::App;
use crate::config::{self, Overrides};
use crate::fail::{die, R};
use crate::pystr::plural;
use crate::{paths, pystr, store};

/// The one command that may create the memory directory, and the whole
/// setup. Re-running it only ever creates what is missing.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    if !args.is_empty() {
        return die(format!("usage: {} init", app.executable));
    }

    let d = app.path().to_path_buf();
    let fresh = !d.is_dir();
    store::makedirs(&d.join("TREE"))?;
    store::touch(&store::log_path(&d))?;

    if !d.join("config").exists() {
        config::write(app.path(), &Overrides::default(), &app.executable)?;
    }

    app.config = config::load(app.path())?;
    let pd = paths::pretty(&app.directory);

    if fresh {
        app.output.line(format_args!(
            "Created {}: this machine's memory, one identity, forever.",
            pd
        ));
    } else {
        let n = store::log_len(&d)?;
        app.output
            .line(format_args!("Found {}: {}.", pd, plural(n, "memory")));
    }

    app.output.line(format_args!(
        "Sizes live in {}/config; the defaults are fine.",
        pd
    ));
    app.output.blank_line();
    app.output.line(format_args!(
        "Paste this at the top of your agent's AGENTS.md (or CLAUDE.md), done:"
    ));
    app.output.blank_line();
    let block = include_str!("../setup.txt")
        .replace("{memo}", &app.executable)
        .replace("{data}", &pd)
        .replace("{chars}", &app.config.entry_bytes.to_string());
    app.output.line(format_args!("{}", pystr::rstrip(&block)));
    Ok(())
}
