use clap::Parser;

pub mod args;
pub mod error;
pub mod logger;
pub mod walk;

fn main() {
    let args = args::Args::try_parse().unwrap();
    log::set_max_level(args.verbosity.log_level_filter());
}
