use clap::Parser;
use globset::{Glob, GlobSetBuilder};
use indicatif::ProgressBar;
use miette::{Context, IntoDiagnostic, Result};

pub mod args;
pub mod codebase;
pub mod gitignore;
pub mod logger;
pub mod os;
pub mod tree;

use codebase::CodebaseBuilder;
use logger::Logger;

/// Git related globs to ignore, I don't see a reason
/// why we should consider these files but if you want
/// to include them you can use `--dangerously-allow-dot-git-traversal` flag.
const GIT_RELATED_IGNORE_PATTERNS: [&str; 2] = ["**/.git", "./**/.git"];

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[nuclei::main]
async fn main() -> Result<()> {
    // Record the start time of the program
    // This is used to calculate the total time taken by the program
    let start = std::time::Instant::now();

    // Parse the command line arguments
    let args = args::Args::parse();

    // Set the log level based on the verbosity flag
    env_logger::builder()
        .format_timestamp(None)
        .format_level(false)
        .format_target(false)
        .format_module_path(false)
        .format_indent(Some(logger::LEVEL_WIDTH + logger::LOCATION_WIDTH))
        .filter_module(CRATE_NAME, args.verbosity.log_level_filter())
        .init();

    // Build the excluded paths
    let mut excluded_paths = GlobSetBuilder::new();
    if let Some(exclude) = args.exclude {
        for glob in exclude {
            excluded_paths.add(glob);
        }
    }
    if !args.do_not_consider_ignore_files {
        for pattern in GIT_RELATED_IGNORE_PATTERNS.iter() {
            excluded_paths.add(Glob::new(pattern).unwrap());
        }
    }
    let excluded_paths = excluded_paths.build().unwrap();

    // Build Codebase
    let codebase = CodebaseBuilder::new()
        .excluded_paths(excluded_paths)
        .consider_gitignores(!args.do_not_consider_ignore_files)
        .max_depth(args.max_depth.unwrap_or(std::usize::MAX))
        .follow_symlinks(args.follow_symbolic_links)
        .build(args.path)?;

    // Create and write to output file
    let formated_tree = codebase.get_formated_tree();
    let formated_files_representation = codebase.get_formated_files_representation()?;
    let mut output_str = String::new();
    output_str.push_str(&formated_tree);
    output_str.push_str("\n\n");
    output_str.push_str(&formated_files_representation);

    let output = args
        .output
        .unwrap_or(std::path::PathBuf::from("output.txt"));
    std::fs::write(output, output_str)
        .into_diagnostic()
        .wrap_err("Failed to write to output file 🫥")?;

    // Record the end time of the program
    let end = std::time::Instant::now();
    // Calculate the time taken by the program
    let time_taken = end - start;
    let time_taken = time_taken.as_secs_f64();
    // Print the time taken by the program
    Logger::info(format!("Done in: {:.4} seconds\r\n", time_taken).as_str());

    Ok(())
}
