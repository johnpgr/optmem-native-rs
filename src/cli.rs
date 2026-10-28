//! OS argument decoding, dispatch, final output, and exit-status translation.
//!
//! Unknown commands are rejected before opening a store. `init` is the only
//! handler allowed to create one. Buffered stdout is emitted even on failure,
//! preserving wake's compression refusal and errors after partial rendering.

use std::io::Write;

use crate::app::{memory_directory, App};
use crate::fail::{self, die, Fail, R};
use crate::output::text_mode;
use crate::{commands, paths};

/// Run one invocation without installing a panic hook or changing global state.
pub fn run() -> i32 {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let mut app = App::new(arguments.first().map(|argument| argument.as_os_str()));

    let Some(name) = arguments.get(1) else {
        emit(&(app.usage() + "\n"), false);
        return 0;
    };

    let name = name.to_string_lossy();
    let Some(handler) = commands::resolve(&name) else {
        emit(
            &format!("No such command: {}\n\n{}\n", name, app.usage()),
            true,
        );
        return 1;
    };

    let arguments: Option<Vec<String>> = arguments[2..]
        .iter()
        .map(|argument| argument.to_str().map(String::from))
        .collect();

    let result = match arguments {
        Some(arguments) => execute(&mut app, &name, handler, &arguments),
        None => die("Arguments must be UTF-8 text."),
    };

    emit(app.output.as_str(), false);

    exit_status(result)
}

/// Only ordinary commands require an existing store and loaded configuration.
fn execute(app: &mut App, name: &str, handler: commands::Handler, args: &[String]) -> R<()> {
    if name != "init" {
        app.open()?;
    }

    handler(app, args)
}

/// Report errors once; open-file errors use the store path as Python does.
fn exit_status(result: R<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(Fail::Exit(status)) => status,
        Err(Fail::Die(message)) => {
            emit(&(message + "\n"), true);
            1
        }
        Err(Fail::Io(path, error)) => {
            let path = path.unwrap_or_else(memory_directory);
            emit(
                &format!("{}: {}.\n", paths::pretty(&path), fail::strerror(&error)),
                true,
            );
            1
        }
    }
}

/// Keep the reference's best-effort output policy and platform text newlines.
fn emit(text: &str, error: bool) {
    let text = text_mode(text);

    if error {
        let _ = std::io::stderr().lock().write_all(text.as_bytes());
        return;
    }

    let mut stdout = std::io::stdout().lock();
    let _ = stdout
        .write_all(text.as_bytes())
        .and_then(|_| stdout.flush());
}
