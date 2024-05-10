use std::path::PathBuf;

use clap::{builder::ValueHint, Parser};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use globset::Glob;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, help = "The path to the directory containing the codebase.", value_hint = ValueHint::DirPath, required = true, default_value = ".")]
    pub path: PathBuf,
    #[arg(short, long, help = "The path of the output file.", value_hint = ValueHint::FilePath, required = false, default_value = "output.md")]
    pub output: Option<PathBuf>,
    #[arg(short, long, help = "Exclude files or directories matching the specified pattern.", value_hint = ValueHint::Other, required = false)]
    pub exclude: Option<Vec<Glob>>,
    #[arg(short, long, help = "Include only files or directories matching the specified pattern.", value_hint = ValueHint::Other, required = false)]
    pub include: Option<Vec<Glob>>,
    #[arg(short, long, help = "Exclude paths as defined in the first ignore file (.gitignore, .hgignore, .ignore, .git/info/exclude and core.excludesFile in .git/config) found in the parent of the current directory being processed (not just the cwd).", value_hint = ValueHint::Other, required = false, default_value = "true")]
    pub consider_ignore_files: Option<bool>,
    #[arg(short, long, help = "Maximum depth to walk into the directory tree.", value_hint = ValueHint::Other, required = false)]
    pub max_depth: Option<usize>,
    #[arg(short, long, help = "Follow symbolic links.", value_hint = ValueHint::Other, required = false, default_value = "false")]
    pub follow_symbolic_links: Option<bool>,
    #[command(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}
