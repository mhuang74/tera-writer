mod opts;
mod template;
mod wrapped_context;

use crate::template::Template;
use anyhow::Result;
use clap::{crate_name, crate_version, Parser};
use env_logger::Env;
use gcra::{GcraState, RateLimit};
use log::{debug, info, trace};
use openai_api::{api::CompletionArgs, Client};
use opts::*;
use regex::Regex;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    fs::{self, canonicalize},
    io::Write,
    path::{Path, PathBuf},
    string::String,
    thread,
    time::Duration,
};
use tera::{Context, Tera};

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("none")).init();
    info!("Running {} v{}", crate_name!(), crate_version!());

    let opts: Opts = Opts::parse();
    debug!("opts:\n{:#?}", opts);

    let wrapped = wrapped_context::WrappedContext::new(&opts.context);
    let context: &Context = wrapped.context();
    trace!("context:\n{:#?}", context);

    let output_path = if let Some(out_path) = &opts.output_path {
        canonicalize(out_path).unwrap()
    } else {
        PathBuf::new()
    };

    // if template is provided, use Tera for processing, else, just process JSON context file
    if let Some(template_path) = &opts.template {
        info!("Rendering Tera Template: {:#?}", template_path);

        let template_string = Template::load(template_path).expect("Failed reading the template");

        trace!("template:\n{}", template_string);

        // use Tera to expand template
        let mut tera = setup_tera(template_path).expect("Failed to setup Tera");

        // render template using list of topics
        let topics = context.get("topics").unwrap().as_array().unwrap();
        info!("Generating posts for {} topics", topics.len());

        // setup rate limit
        let rate_limit = RateLimit::new(60, Duration::from_secs(60));
        let mut user_state = GcraState::default();

        let mut rendered: String;

        for topic in topics {
            info!("topic: {:#?}", topic);

            let mut ctx: Context = Context::new();
            ctx.insert("topic", topic);

            while user_state.check_and_modify(&rate_limit, 4).is_err() {
                info!("Rate limited...sleeping...");
                thread::sleep(Duration::from_millis(1000));
                info!("Rate limited...waking up...");
            }

            rendered = tera.render_str(&template_string, &ctx).unwrap();
            debug!("Rendered: {}", rendered);

            let mut my_path = create_topic_directory(&output_path, topic.as_str().unwrap());
            my_path.push("index.md");
            debug!("Saving to {}", my_path.display());

            let mut file = File::create(my_path).expect("Failed opening output file");
            file.write_all(rendered.as_bytes())
                .map_err(|e| e.to_string())
                .unwrap();
        }
    } else {
        // expand input JSON via Completion API
        info!("Expanding JSON file: {:#?}", &opts.context);
    };

    Ok(())
}

/// instantiate tera and register openai custom function
fn setup_tera(template_path: &Path) -> Result<Tera> {
    // construct template path for Tera

    let path = canonicalize(template_path).unwrap();
    let mut dir = path.to_str().unwrap();

    if path.is_file() {
        dir = path.parent().unwrap().to_str().unwrap();
    }

    let glob = dir.to_owned() + "/**/*";

    // instantiate Tera
    let mut tera = match Tera::new(&glob) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // define and register custom tera function
    fn openai_completion(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
        let api_token = std::env::var("OPENAI_API_KEY").expect("No openai api key found");
        let client = Client::new(&api_token);

        let prompt = args
            .get("prompt")
            .expect("No prompt given")
            .as_str()
            .unwrap();
        debug!("Prompt: {}", prompt);
        let tokens = args
            .get("tokens")
            .expect("No token count given")
            .as_u64()
            .unwrap();

        let completion_args = CompletionArgs::builder()
            .model("text-curie-001")
            .max_tokens(tokens)
            .temperature(0.7);

        let completion = client
            .complete_prompt_sync(completion_args.prompt(prompt).build()?)
            .unwrap();

        debug!("Completion: {:#?}", completion);

        Ok(completion.to_string().trim().into())
    }

    tera.register_function("openai_completion", openai_completion);

    Ok(tera)
}

fn create_topic_directory(output_path: &Path, topic: &str) -> PathBuf {
    let re_non_alpha: Regex = Regex::new(r"[[:^alpha:]]").unwrap();
    let re_spaces: Regex = Regex::new(r"[ ]+").unwrap();

    let stripped = re_non_alpha.replace_all(topic, " ");
    let directory = re_spaces.replace_all(stripped.as_ref(), "_");

    debug!("directory: {:#?}", directory);

    let mut my_path: PathBuf = PathBuf::from(output_path);
    my_path.push(directory.as_ref());

    // create directory if not exist
    match fs::create_dir(&my_path) {
        Err(e) => {
            trace!("Unable to create directory {}: {}", directory, e);
        }
        Ok(()) => {
            trace!("Created Dir: {}", directory);
        }
    };

    my_path
}
