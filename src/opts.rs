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
    #[arg(
        short,
        long,
        requires = "category_subdirectory_key",
        requires = "title_subdirectory_key"
    )]
    pub template: Option<PathBuf>,

    /// Output path [Default: current directory]
    #[arg(short, long)]
    pub output_path: Option<PathBuf>,

    /// Context key for category subdirectory in output path
    #[arg(long = "cat_sub")]
    pub category_subdirectory_key: Option<String>,

    /// Context key for title subdirectory in output path
    #[arg(long = "title_sub")]
    pub title_subdirectory_key: Option<String>,
}
