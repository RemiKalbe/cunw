use std::{cell::RefCell, rc::Rc, time::Duration};

use clap::Parser;
use crossterm::{
    execute,
    style::{PrintStyledContent, Stylize},
};
use indicatif::ProgressBar;

pub mod args;
pub mod error;
pub mod file;
pub mod logger;
pub mod walk;

fn main() {
    // Record the start time of the program
    let start = std::time::Instant::now();

    let args = args::Args::parse();
    log::set_max_level(args.verbosity.log_level_filter());

    let ignore_dot_git = globset::Glob::new("/.git").unwrap();

    let exclude = args
        .exclude
        .unwrap_or_default()
        .into_iter()
        .chain(if args.dangerously_allow_dot_git_traversal {
            vec![]
        } else {
            vec![ignore_dot_git]
        })
        .collect::<Vec<_>>();

    let walker = walk::ArgWalker::try_from(
        &args.path,
        !args.do_not_consider_ignore_files,
        Some(exclude),
        args.include,
        args.max_depth,
        args.follow_symbolic_links,
    )
    .unwrap();
    let session = walker.try_sprint().unwrap();
    let session = session.into_iter().fold(Vec::new(), |mut acc, dir| {
        if let Ok(dir) = dir {
            acc.push(dir);
        }
        acc
    });
    let paths = session
        .iter()
        .map(|dir| dir.path().to_path_buf())
        .collect::<Vec<_>>();
    let paths = Rc::new(RefCell::new(paths));

    let tree_pg = ProgressBar::new(paths.borrow().len() as u64);
    tree_pg.enable_steady_tick(Duration::from_millis(100));
    tree_pg.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.94} {pos}/{len} {msg}") // 94: 	Orange4	#875f00
            .unwrap()
            .tick_strings(&["◜", "◠", "◝", "◞", "◡", "◟", "◯"]),
    );
    tree_pg.set_message("Building directory tree...");

    let tree = file::DirectoryTree::from(
        args.path.clone(),
        args.path.clone(),
        args.path.clone(),
        Some(paths),
        &tree_pg,
    );

    tree_pg.finish_with_message("Directory tree built!");

    let files_paths = session
        .iter()
        .filter(|dir| {
            let metadata = dir.metadata().unwrap();
            metadata.is_file()
        })
        .map(|dir| dir.path().to_path_buf())
        .collect::<Vec<_>>();

    let mut file_collector = file::FileCollector::new(args.path.clone());
    file_collector.collect_files(files_paths).unwrap();

    let output_generator = file::OutputGenerator::new(file_collector.files, tree);
    output_generator
        .write_file(&args.output.unwrap_or(std::path::PathBuf::from("output.md")))
        .unwrap();

    // Record the end time of the program
    let end = std::time::Instant::now();
    // Calculate the time taken by the program
    let time_taken = end - start;
    let time_taken = time_taken.as_secs_f64();
    // Print the time taken by the program
    execute!(
        std::io::stdout(),
        PrintStyledContent(format!("Done in: {:.4} seconds\r\n", time_taken).dim())
    )
    .unwrap();
}
