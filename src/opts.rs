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

	/// This flag tells the command to parse all templates found in the same
	/// path where the given template is located.
	#[clap(short, long, visible_alias = "inherit")]
	pub include: bool,

	/// Option to define a different path from which search and parse templates.
	#[clap(long, visible_alias = "inherit-path")]
	pub include_path: Option<PathBuf>,

	/// Location of the input data in JSON format.
	#[clap(index = 1)]
	pub context: Option<PathBuf>,

	/// Optional output file. If not passed, using stdout.
	#[clap(short, long)]
	pub out: Option<PathBuf>,
}
