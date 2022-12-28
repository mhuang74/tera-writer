use clap::Parser;
use std::path::PathBuf;

/// CLI that couples Tera templates with OpenAI Completion API
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// JSON file with topics and prompts for Completion API, or used as data source when Tera template is specified.
    #[arg(index = 1)]
    pub context: PathBuf,

    /// Tera template to inject JSON data into.
    #[arg(short, long, requires = "directory_key")]
    pub template: Option<PathBuf>,

    /// Output path [Default: current directory]
    #[arg(short, long)]
    pub output_path: Option<PathBuf>,

    /// create output directories based on value of this context key
    #[arg(short, long)]
    pub directory_key: Option<String>,
}
