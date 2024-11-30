use std::path::PathBuf;

use clap::{builder::ValueHint, ArgAction, Parser};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use globset::Glob;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(index = 1, help = "The path to the directory containing the codebase.", value_hint = ValueHint::DirPath, required = true)]
    pub path: PathBuf,
    #[arg(short, long, help = "The path of the output file.", value_hint = ValueHint::FilePath, required = false, default_value = "output.txt")]
    pub output: Option<PathBuf>,
    #[arg(short, long, help = "Exclude files or directories matching the specified pattern.", value_hint = ValueHint::Other, required = false, num_args = 0.., action = ArgAction::Append)]
    pub exclude: Option<Vec<Glob>>,
    #[arg(
        long,
        help = "Exit on non-UTF-8 content.",
        required = false,
        default_value = "false"
    )]
    pub exit_on_non_utf8: bool,
    #[arg(
        long,
        help = "Do not consider the ignore files (.gitignore, .hgignore, .ignore, .git/info/exclude and core.excludesFile in .git/config).",
        required = false,
        default_value = "false"
    )]
    pub do_not_consider_ignore_files: bool,
    #[arg(
        long,
        help = "Include .git directory in the search.",
        required = false,
        default_value = "false"
    )]
    pub dangerously_allow_dot_git_traversal: bool,
    #[arg(short, long, help = "Maximum depth to walk into the directory tree.", value_hint = ValueHint::Other, required = false)]
    pub max_depth: Option<usize>,
    #[arg(
        short,
        long,
        help = "Follow symbolic links.",
        required = false,
        default_value = "false"
    )]
    pub follow_symbolic_links: bool,
    #[command(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_args() {
        let args = Args::parse_from(&["cunw", "/path/to/codebase"]);
        assert_eq!(args.path.to_str().unwrap(), "/path/to/codebase");
        assert_eq!(args.output, Some(std::path::PathBuf::from("output.txt")));
        assert_eq!(args.exclude, None);
        assert_eq!(args.exit_on_non_utf8, false);
        assert_eq!(args.do_not_consider_ignore_files, false);
        assert_eq!(args.dangerously_allow_dot_git_traversal, false);
        assert_eq!(args.max_depth, None);
        assert_eq!(args.follow_symbolic_links, false);
    }

    #[test]
    fn test_custom_args() {
        let args = Args::parse_from(&[
            "cunw",
            "/path/to/codebase",
            "-o",
            "custom_output.md",
            "-e",
            "*.txt",
            "--exit-on-non-utf8",
            "--do-not-consider-ignore-files",
            "--dangerously-allow-dot-git-traversal",
            "-m",
            "3",
            "-f",
            "-v",
        ]);
        assert_eq!(args.path.to_str().unwrap(), "/path/to/codebase");
        assert_eq!(
            args.output,
            Some(std::path::PathBuf::from("custom_output.md"))
        );
        assert_eq!(args.exclude.unwrap()[0].glob(), "*.txt");
        assert_eq!(args.exit_on_non_utf8, true);
        assert_eq!(args.do_not_consider_ignore_files, true);
        assert_eq!(args.dangerously_allow_dot_git_traversal, true);
        assert_eq!(args.max_depth, Some(3));
        assert_eq!(args.follow_symbolic_links, true);
        assert_eq!(args.verbosity.log_level_filter(), log::LevelFilter::Debug);
    }
}
