mod opts;
mod template;
mod wrapped_context;

use crate::template::Template;
use clap::{crate_name, crate_version, Parser};
use env_logger::Env;
use log::{debug, info, trace};
use opts::*;
use std::{fs::{canonicalize, self}, fs::File, io::Write, string::String, path::{Path, PathBuf}};
use tera::{Context, Tera};
use regex::Regex;

fn main() -> Result<(), String> {
	env_logger::Builder::from_env(Env::default().default_filter_or("none")).init();
	info!("Running {} v{}", crate_name!(), crate_version!());

	let opts: Opts = Opts::parse();
	debug!("opts:\n{:#?}", opts);

	let template = Template::load(&opts.template).expect("Failed reading the template");
	trace!("template:\n{}", template);

	let mut path = canonicalize(&opts.template).unwrap();
	let mut output_path = if let Some(out_path) = &opts.output_path {
		canonicalize(out_path).unwrap()
	} else {
		PathBuf::new()
	};


	let mut wrapped_context = wrapped_context::WrappedContext::new(opts);
	wrapped_context.create_context();

	let context: &Context = wrapped_context.context();
	trace!("context:\n{:#?}", context);

	let mut rendered : String = String::default();

	let re_non_alpha : Regex = Regex::new(r"[[:^alpha:]]").unwrap();
	let re_spaces : Regex = Regex::new(r"[ ]+").unwrap();

	let topics = context.get("topics").unwrap().as_array().unwrap();

	info!("Generating posts for {} topics", topics.len());

	for topic in topics {
		trace!("topic: {:#?}", topic);

		let stripped = re_non_alpha.replace_all(topic.as_str().unwrap(), " ");
		let directory = re_spaces.replace_all(stripped.as_ref(), "_");

		trace!("directory: {:#?}", directory);

		let mut my_path = output_path.clone();
		my_path.push(directory.as_ref());
		fs::create_dir(&my_path);
		my_path.push("index.md");

		debug!("Saving to {}", my_path.display());

		let mut ctx : Context = Context::new();
		ctx.insert("topic", topic);

		rendered = Tera::one_off(&template, &ctx, false).unwrap();

		trace!("{}", rendered);

		let mut file = File::create(my_path).expect("Failed opening output file");
		file.write_all(rendered.as_bytes()).map_err(|e| e.to_string());

	}





	Ok(())
}
