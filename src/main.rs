use clap::Parser;
use globset::{Glob, GlobSetBuilder};

pub mod args;
pub mod codebase;
pub mod error;
pub mod gitignore;
pub mod logger;
pub mod os;
pub mod tree;

use codebase::CodebaseBuilder;
use error::{CunwError, Result};
use logger::Logger;

/// Git related globs to ignore, I don't see a reason
/// why we should consider these files but if you want
/// to include them you can use `--dangerously-allow-dot-git-traversal` flag.
const GIT_RELATED_IGNORE_PATTERNS: [&str; 2] = ["**/.git", "./**/.git"];

#[tokio::main]
async fn main() -> Result<()> {
    // Record the start time of the program
    // This is used to calculate the total time taken by the program
    let start = std::time::Instant::now();

    // Parse the command line arguments
    let args = args::Args::parse();

    // Set the log level based on the verbosity flag
    logger::Logger::init(Some(args.verbosity.log_level_filter()));

    // Build the excluded paths
    let mut excluded_paths = GlobSetBuilder::new();
    if let Some(exclude) = args.exclude {
        // We normalize the path so that the glob pattern
        // begins with the base path.
        // We also ensure that we normalize only if needed.
        let base = args.path.clone();
        let base = base.to_str().unwrap();
        let base = {
            if base.ends_with('/') {
                base.to_string()
            } else {
                format!("{}/", base)
            }
        };
        for glob in exclude {
            let excluded_paths_with_base = {
                let original_glob = glob.glob();
                if !original_glob.starts_with(&base) {
                    if original_glob.starts_with('/') {
                        let glob = original_glob.strip_prefix('/').unwrap().to_string();
                        format!("{}{}", base, glob)
                    } else {
                        let glob = original_glob.to_string();
                        format!("{}{}", base, glob)
                    }
                } else {
                    original_glob.to_string()
                }
            };
            let glob = Glob::new(&excluded_paths_with_base).unwrap();
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
        .build(args.path)
        .await?;

    // Create and write to output file
    let output_str = codebase.try_to_string()?;

    let output = args
        .output
        .unwrap_or(std::path::PathBuf::from("output.txt"));
    std::fs::write(output.clone(), output_str)
        .map_err(|err| CunwError::new(err.into()).with_file(output))?;

    // Record the end time of the program
    let end = std::time::Instant::now();
    // Calculate the time taken by the program
    let time_taken = end - start;
    let time_taken = time_taken.as_secs_f64();
    // Print the time taken by the program
    Logger::info(format!("Done in: {:.4} seconds\r\n", time_taken).as_str());

    Ok(())
}
