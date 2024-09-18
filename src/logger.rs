use std::panic::Location;

use colored::{ColoredString, Colorize};
use log::{debug, error, info, trace, warn, Level};
use terminal_emoji::Emoji;

pub struct Logger;

pub const LOCATION_WIDTH: usize = 40;
pub const LEVEL_WIDTH: usize = 3;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

impl Logger {
    pub fn init(verbosity: Option<log::LevelFilter>) {
        let mut builder = env_logger::builder();
        builder
            .format_timestamp(None)
            .format_level(false)
            .format_target(false)
            .format_module_path(false)
            .format_indent(Some(LEVEL_WIDTH + LOCATION_WIDTH));

        if let Some(verbosity) = verbosity {
            builder.filter_module(CRATE_NAME, verbosity);
        }
        let _ = builder.try_init();
    }

    fn format_location(l: &Location<'static>) -> ColoredString {
        let file = l.file();
        let line = l.line();
        let column = l.column();
        let l = format!("in {} {}:{}", file.underline(), line, column);
        let padding = " ".repeat(LOCATION_WIDTH - l.chars().count());
        format!("{}{}", l, padding).dimmed()
    }
    fn format_level(level: Level) -> ColoredString {
        let padding = " ".repeat(LEVEL_WIDTH - 1);
        match level {
            Level::Error => format!("{}{}", Emoji::new("🚨", "E"), padding).red(),
            Level::Warn => format!("{}{}", Emoji::new("😳", "W"), padding).yellow(),
            Level::Info => format!("{}{}", Emoji::new("🤓", "I"), padding).green(),
            Level::Debug => format!("{}{}", Emoji::new("🐛", "D"), padding).blue(),
            Level::Trace => format!("{}{}", Emoji::new("🔬", "T"), padding).purple(),
        }
    }
    #[track_caller]
    pub fn trace(message: &str) {
        let location = Location::caller();
        trace!(
            "{} {} {}",
            Self::format_location(&location),
            Self::format_level(Level::Trace),
            message.purple(),
        );
    }
    #[track_caller]
    pub fn debug(message: &str) {
        let location = Location::caller();
        debug!(
            "{} {} {}",
            Self::format_location(&location),
            Self::format_level(Level::Debug),
            message.blue(),
        );
    }
    #[track_caller]
    pub fn info(message: &str) {
        let location = Location::caller();
        info!(
            "{} {} {}",
            Self::format_location(&location),
            Self::format_level(Level::Info),
            message.green(),
        );
    }
    #[track_caller]
    pub fn warn(message: &str) {
        let location = Location::caller();
        warn!(
            "{} {} {}",
            Self::format_location(&location),
            Self::format_level(Level::Warn),
            message.yellow(),
        );
    }
    #[track_caller]
    pub fn error(message: &str) {
        let location = Location::caller();
        error!(
            "{} {} {}",
            Self::format_location(&location),
            Self::format_level(Level::Error),
            message.red(),
        );
    }
}
