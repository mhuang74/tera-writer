use clap::{crate_authors, crate_version, Parser};
use std::path::PathBuf;
 
/// Command line ai writer using Tera templating engine.
/// Input topic list provided in JSON format.
#[derive(Debug, Parser)]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
	/// Location of the template.
	#[clap(short, long)]
	pub template: PathBuf,

	/// Location of post topic list in a single JSON list).
	#[clap(index = 1)]
	pub context: Option<PathBuf>,

	/// Optional output path. If not passed, using current directory.
	#[clap(short, long)]
	pub output_path: Option<PathBuf>,
}
