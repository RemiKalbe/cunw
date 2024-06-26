use std::path::PathBuf;

use clap::{builder::ValueHint, ArgAction, Parser};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use globset::Glob;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(index = 1, short, long, help = "The path to the directory containing the codebase.", value_hint = ValueHint::DirPath, required = true)]
    pub path: PathBuf,
    #[arg(short, long, help = "The path of the output file.", value_hint = ValueHint::FilePath, required = false, default_value = "output.txt")]
    pub output: Option<PathBuf>,
    #[arg(short, long, help = "Exclude files or directories matching the specified pattern.", value_hint = ValueHint::Other, required = false, num_args = 0.., action = ArgAction::Append)]
    pub exclude: Option<Vec<Glob>>,
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
