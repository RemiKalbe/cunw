use std::{backtrace::Backtrace, fmt::Write, io::stdout};

use crossterm::{
    execute,
    style::{PrintStyledContent, StyledContent, Stylize},
    Command,
};
use log::Level as LogLevel;

use crate::error::Error;

pub struct Logger;

impl Logger {
    pub fn format_level<'a>(log_level: &LogLevel) -> StyledContent<String> {
        match log_level {
            LogLevel::Error => "😱  ERROR  ".to_string().red().bold(),
            LogLevel::Warn => "😳  WARN   ".to_string().yellow().bold(),
            LogLevel::Info => "🤓☝️  INFO ".to_string().green().bold(),
            LogLevel::Debug => "🧐  DEBUG  ".to_string().blue().bold(),
            LogLevel::Trace => "🔬  TRACE  ".to_string().magenta().bold(),
        }
    }
    pub fn format_message_title<'a>(
        message_title: &'a str,
        message_is_some: bool,
    ) -> StyledContent<String> {
        let m = match message_is_some {
            true => format!("{}: ", message_title).bold(),
            false => format!("{}\r\n", message_title).bold(),
        };
        m
    }
    pub fn format_message<'a>(message: &'a str) -> StyledContent<String> {
        message.to_string().dim()
    }
    pub fn format_helper_message<'a>(message: &'a str) -> Vec<StyledContent<String>> {
        let t = format!("\nHelper:").cyan();
        let m = format!("{}", message).dim();
        vec![t.stylize(), m.stylize()]
    }
    pub fn format_error_message<'a>(err: &'a str) -> Vec<StyledContent<String>> {
        let t = format!("\nOriginal Error:\n").red();
        let m = format!("{}", err).red().dim();
        vec![t.stylize(), m.stylize()]
    }
    pub fn format_backtrace_message(backtrace: Backtrace) -> Vec<StyledContent<String>> {
        let t = format!("\nBacktrace:\r\n").dim();
        let m = format!("{}\r\n", backtrace);
        vec![t.stylize(), m.stylize()]
    }
    pub fn break_line(buffer: &[StyledContent<String>]) -> Vec<StyledContent<String>> {
        [buffer, ["\r\n".to_string().stylize()].as_slice()].concat()
    }
    pub fn concat_with_space(
        buffer1: &[StyledContent<String>],
        buffer2: &[StyledContent<String>],
    ) -> Vec<StyledContent<String>> {
        [buffer1, [" ".to_string().stylize()].as_slice(), buffer2].concat()
    }
    pub fn format_log_err(err: Error) -> (LogLevel, Vec<StyledContent<String>>) {
        let (context, inner, backtrace) = match err {
            Error::ArgError {
                inner,
                context,
                backtrace,
            } => (context, inner.to_string(), backtrace),
            Error::IOError {
                inner,
                context,
                backtrace,
            } => (context, inner.to_string(), backtrace),
            Error::RuntimeError {
                inner,
                context,
                backtrace,
            } => (context, inner.to_string(), backtrace),
            Error::ArgPatternError {
                inner,
                backtrace,
                context,
            } => (context, inner.to_string(), backtrace),
            Error::IgnoreFilePatternError {
                inner,
                backtrace,
                context,
            } => (context, inner.to_string(), backtrace),
            Error::UnknownError {
                inner,
                backtrace,
                context,
            } => (context, inner, backtrace),
            Error::WalkError {
                inner,
                backtrace,
                context,
            } => (context, inner.to_string(), backtrace),
        };

        let (c, log_level) = match context {
            Some(ref c) => {
                let message_title = match c.message_title.as_ref() {
                    Some(m) => Self::format_message_title(m, c.message.is_some()),
                    None => String::default().stylize(),
                };
                let message = match c.message.as_ref() {
                    Some(m) => Self::format_message(m),
                    None => String::default().stylize(),
                };
                let helper = match c.helper.as_ref() {
                    Some(h) => Self::format_helper_message(h),
                    None => vec![],
                };
                // If the log level is Debug or Trace, we want to show which crate the log message is coming from
                let from_crate = match (c.from_crate.as_ref(), c.level) {
                    (Some(c), LogLevel::Debug) | (Some(c), LogLevel::Trace) => {
                        let c = c.to_string();
                        Self::format_message(c.as_str())
                    }
                    _ => String::default().stylize(),
                };

                (
                    vec![
                        [message_title, message, from_crate].as_slice(),
                        helper.as_slice(),
                    ]
                    .concat(),
                    c.level,
                )
            }
            None => (vec![], LogLevel::Error),
        };

        // If the log level is Debug or Trace, we want to show the original error message
        let e = Self::format_error_message(&inner);
        // If the log level is Trace, we want to show the backtrace
        let b = Self::format_backtrace_message(backtrace);

        let mut log_message = c;
        if log_level <= LogLevel::Debug {
            log_message = Self::break_line(&log_message);
            log_message = Self::concat_with_space(&log_message, &e);
        }
        if log_level <= LogLevel::Trace {
            log_message = Self::break_line(&log_message);
            log_message = Self::concat_with_space(&log_message, &b);
        }

        let log_level_str = Self::format_level(&log_level);

        (
            log_level,
            Self::concat_with_space(&[log_level_str], &log_message),
        )
    }
    pub fn should_log(log_level: LogLevel) -> bool {
        let mut env_log_level = log::max_level();
        // Set the log level to Trace if we are running a test
        if cfg!(test) {
            env_log_level = log::LevelFilter::Trace;
        }

        log_level <= env_log_level
    }
    pub fn log_err(err: Error) {
        let (message_level, buffer) = Self::format_log_err(err);
        if Self::should_log(message_level) {
            let buffer = Self::break_line(&buffer);
            for b in buffer {
                execute!(stdout(), PrintStyledContent(b)).unwrap();
            }
        }
    }
    pub fn log_err_with(err: Error, w: &mut impl Write) -> std::fmt::Result {
        let (message_level, buffer) = Self::format_log_err(err);
        if Self::should_log(message_level) {
            let buffer = Self::break_line(&buffer);
            for b in buffer {
                PrintStyledContent(b).write_ansi(w)?;
            }
        }
        Ok(())
    }
    pub fn log_message(
        log_level: LogLevel,
        title: &str,
        message: Option<&str>,
    ) -> std::fmt::Result {
        if Self::should_log(log_level) {
            let message_title = Self::format_message_title(title, message.is_some());
            let message = match message {
                Some(m) => Self::format_message(m),
                None => String::default().stylize(),
            };
            let buffer = Self::concat_with_space(&[message_title], &[message]);
            let log_level_str = Self::format_level(&log_level);
            let buffer = Self::concat_with_space(&[log_level_str], &buffer);
            let buffer = Self::break_line(&buffer);
            for b in buffer {
                execute!(stdout(), PrintStyledContent(b)).unwrap();
            }
        }
        Ok(())
    }
}
