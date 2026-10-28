//! Store-local sizes, validated overrides, and the stable configuration format.
//!
//! Named fields keep byte and line budgets distinct. Overrides retain whether a
//! value was explicitly set, so resetting a setting follows future tool defaults
//! and config output can distinguish an override from an inherited value.

use std::path::Path;

use crate::fail::{die, IoAt, R};
use crate::output::text_mode;
use crate::{paths, pystr, store, text};

/// Effective configuration; CHARS in the external format means UTF-8 bytes.
#[derive(Clone, Copy)]
pub struct Config {
    /// Desired number of aligned context lines before pagination.
    pub wake_lines: u64,
    /// Maximum UTF-8 bytes in one note or submitted summary.
    pub entry_bytes: u64,
    /// Maximum memory-text bytes per page or retained recall window, including LF.
    pub part_bytes: u64,
    /// Maximum context lines per wake page, independently of the byte limit.
    pub part_lines: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wake_lines: 96,
            entry_bytes: 280,
            part_bytes: 20_000,
            part_lines: 500,
        }
    }
}

/// A setting's identity, independent of its presentation order or array index.
#[derive(Clone, Copy)]
pub enum Knob {
    /// Context coverage budget, named WAKE_LINES in config files.
    WakeLines,
    /// Per-entry UTF-8 byte ceiling, named ENTRY_CHARS externally.
    EntryBytes,
    /// Page/recall UTF-8 byte budget, named PART_CHARS externally.
    PartBytes,
    /// Context lines per wake page, named PART_LINES externally.
    PartLines,
}

impl Knob {
    /// The reference's stable display order, shared by config files and stdout.
    pub const ALL: [Self; 4] = [
        Self::WakeLines,
        Self::EntryBytes,
        Self::PartBytes,
        Self::PartLines,
    ];

    /// Preserve the public configuration names while using precise internal names.
    pub fn name(self) -> &'static str {
        match self {
            Self::WakeLines => "WAKE_LINES",
            Self::EntryBytes => "ENTRY_CHARS",
            Self::PartBytes => "PART_CHARS",
            Self::PartLines => "PART_LINES",
        }
    }

    /// Resolve an already stripped, uppercased key from a file or command argument.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|knob| knob.name() == name)
    }

    /// Read the default from the same named configuration used at startup.
    pub fn default_value(self) -> u64 {
        let defaults = Config::default();

        match self {
            Self::WakeLines => defaults.wake_lines,
            Self::EntryBytes => defaults.entry_bytes,
            Self::PartBytes => defaults.part_bytes,
            Self::PartLines => defaults.part_lines,
        }
    }

    /// Agent-facing explanation preserved in config files and command output.
    pub fn description(self) -> &'static str {
        match self {
            Self::WakeLines => "the memory context: how many lines wake prints",
            Self::EntryBytes => "the longest one memory may be, in bytes",
            Self::PartBytes => "output paging: largest part, in bytes",
            Self::PartLines => "output paging: largest part, in lines",
        }
    }

    /// Validate a positive size, including the fixed-record ceiling for entries.
    pub fn parse_size(self, value: &str, location: &str) -> R<u64> {
        let Some(number) = pystr::uint(value).filter(|number| *number >= 1) else {
            return die(format!(
                "{}{} must be a positive whole number, not '{}'.",
                location,
                self.name(),
                value
            ));
        };

        let entry_ceiling = (store::TREE_REC - 8).min(store::LOG_REC - 40);

        if matches!(self, Self::EntryBytes) && number > entry_ceiling {
            return die(format!(
                "{}ENTRY_CHARS is at most {}: a memory has to fit the fixed-width records.",
                location, entry_ceiling
            ));
        }

        Ok(number)
    }
}

/// None follows the default, even if an explicit value would equal that default.
#[derive(Default)]
pub struct Overrides {
    wake_lines: Option<u64>,
    entry_bytes: Option<u64>,
    part_bytes: Option<u64>,
    part_lines: Option<u64>,
}

impl Overrides {
    /// Inspect the presence of an override without losing its default/reset state.
    pub fn get(&self, knob: Knob) -> Option<u64> {
        match knob {
            Knob::WakeLines => self.wake_lines,
            Knob::EntryBytes => self.entry_bytes,
            Knob::PartBytes => self.part_bytes,
            Knob::PartLines => self.part_lines,
        }
    }

    /// Store a validated override, or reset it when a CLI value is empty.
    pub fn set(&mut self, knob: Knob, value: Option<u64>) {
        let field = match knob {
            Knob::WakeLines => &mut self.wake_lines,
            Knob::EntryBytes => &mut self.entry_bytes,
            Knob::PartBytes => &mut self.part_bytes,
            Knob::PartLines => &mut self.part_lines,
        };

        *field = value;
    }

    /// Apply overrides without changing their persistent representation.
    fn effective(&self) -> Config {
        let defaults = Config::default();

        Config {
            wake_lines: self.wake_lines.unwrap_or(defaults.wake_lines),
            entry_bytes: self.entry_bytes.unwrap_or(defaults.entry_bytes),
            part_bytes: self.part_bytes.unwrap_or(defaults.part_bytes),
            part_lines: self.part_lines.unwrap_or(defaults.part_lines),
        }
    }
}

/// List accepted keys in the same order as the reference's size dictionary.
pub fn names() -> String {
    Knob::ALL.map(Knob::name).join(", ")
}

/// Load effective settings; a missing file follows all tool defaults.
pub fn load(directory: &Path) -> R<Config> {
    Ok(read_overrides(directory)?.effective())
}

/// Parse universal newlines and inline comments, retaining the last repeated key.
pub fn read_overrides(directory: &Path) -> R<Overrides> {
    let mut overrides = Overrides::default();
    let path = directory.join("config");

    if !path.exists() {
        return Ok(overrides);
    }

    let display_path = paths::pretty(&path.to_string_lossy());
    let Some(contents) = text::read_utf8(&path)? else {
        return die(format!("{} is not UTF-8 text.", display_path));
    };

    for (index, line) in pystr::lines(&contents).iter().enumerate() {
        let line = pystr::strip(line.split('#').next().unwrap_or(""));
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };

        let name = pystr::strip(name).to_uppercase();
        let location = format!("{} line {}: ", display_path, index + 1);
        let Some(knob) = Knob::named(&name) else {
            return die(format!(
                "{}{} is not a size. Delete the line, or name one of: {}.",
                location,
                name,
                names()
            ));
        };

        let number = knob.parse_size(pystr::strip(value), &location)?;
        overrides.set(knob, Some(number));
    }

    Ok(overrides)
}

/// Rewrite the canonical config, commenting out settings that follow defaults.
pub fn write(directory: &Path, overrides: &Overrides, executable: &str) -> R<()> {
    let mut lines = vec![
        "# OptMem sizes for this memory. A commented line means: follow the".to_owned(),
        format!(
            "# tool's default. Edit with `{} config NAME=VALUE`.",
            executable
        ),
        String::new(),
    ];

    for knob in Knob::ALL {
        let value = overrides.get(knob);
        let mark = if value.is_some() { "" } else { "# " };

        lines.push(format!(
            "{:<2}{:<12} = {:<6} # {}",
            mark,
            knob.name(),
            value.unwrap_or(knob.default_value()),
            knob.description()
        ));
    }

    let path = directory.join("config");
    let contents = text_mode(&(lines.join("\n") + "\n"));

    std::fs::write(&path, contents).at(&path)
}
