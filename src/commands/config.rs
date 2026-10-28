//! Inspect or update validated overrides while preserving reference display order.

use crate::app::App;
use crate::config::{self, Knob};
use crate::fail::{die, R};
use crate::pystr;

/// An empty CLI value resets a setting; no arguments leave the config file alone.
pub(super) fn execute(app: &mut App, args: &[String]) -> R<()> {
    let mut overrides = config::read_overrides(app.path())?;

    for argument in args {
        let Some((name, value)) = argument.split_once('=') else {
            return usage(app);
        };

        let name = pystr::strip(name).to_uppercase();
        let Some(knob) = Knob::named(&name) else {
            return usage(app);
        };

        let value = pystr::strip(value);
        let override_value = if value.is_empty() {
            None
        } else {
            Some(knob.parse_size(value, "")?)
        };

        overrides.set(knob, override_value);
    }

    if !args.is_empty() {
        config::write(app.path(), &overrides, &app.executable)?;
    }

    for knob in Knob::ALL {
        let value = overrides.get(knob);
        let default_note = match value {
            Some(_) => format!(" (default {})", knob.default_value()),
            None => String::new(),
        };

        app.output.line(format_args!(
            "{:<12} {:<7} {}{}",
            knob.name(),
            value.unwrap_or(knob.default_value()),
            knob.description(),
            default_note
        ));
    }

    Ok(())
}

/// File and argv validation share knob names, but have different error contexts.
fn usage(app: &App) -> R<()> {
    die(format!(
        "usage: {} config [NAME=VALUE ...]   # NAME one of {}",
        app.executable,
        config::names()
    ))
}
