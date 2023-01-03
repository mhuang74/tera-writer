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
use serde_json::{json, Map, Value};
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
use lazy_static::lazy_static;

lazy_static! {
    // global Singleton for single-thread use
    static ref OPENAI_CLIENT: openai_api::Client = {
        let api_token = std::env::var("OPENAI_API_KEY").expect("No openai api key found");
        let client = Client::new(&api_token);
        client
    };
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("none")).init();
    info!("Running {} v{}", crate_name!(), crate_version!());

    let opts: Opts = Opts::parse();
    debug!("opts:\n{:#?}", opts);

    let wrapped_context = wrapped_context::WrappedContext::new(&opts.context);
    let input_context: &Context = wrapped_context.context();
    trace!("input context:\n{:#?}", input_context);

    // assume context file has a "contexts" object holding a list of context dictionaries
    let contexts = input_context
        .get("contexts")
        .expect("Missing 'contexts' as list of dictionaries")
        .as_array()
        .unwrap();
    info!("Generating contents for {} contexts", contexts.len());

    let output_path = if let Some(out_path) = &opts.output_path {
        canonicalize(out_path).unwrap()
    } else {
        PathBuf::new()
    };

    // if template is provided, use Tera for processing, else, just process JSON context file
    if let Some(template_path) = &opts.template {
        info!("Rendering Tera Template: {:#?}", template_path);

        let template_string = Template::load(template_path).expect("Failed reading the template");

        debug!("template:\n{}", template_string);

        // use Tera to expand template
        let mut tera = setup_tera(template_path).expect("Failed to setup Tera");

        // setup rate limit
        let rate_limit = RateLimit::new(60, Duration::from_secs(60));
        let mut user_state = GcraState::default();

        let directory_key = &opts.directory_key.unwrap();

        let mut rendered: String;

        for (idx, context) in contexts.iter().enumerate() {
            // let topic = context
            //     .get("topic")
            //     .expect("Missing 'topic' element")
            //     .as_str()
            //     .unwrap();

            trace!("Processing context[{}]: {:#?}", idx, context);

            let tera_context: tera::Context = Context::from_value(context.to_owned())?;
            trace!("Tera context[{}]: {:#?}", idx, tera_context);

            // HACK: set cost=4 since currently calling openai via 4 prompts per template
            while user_state.check_and_modify(&rate_limit, 4).is_err() {
                trace!("Rate limited..sleeping");
                thread::sleep(Duration::from_millis(1000));
            }

            rendered = tera.render_str(&template_string, &tera_context).unwrap();
            trace!("Rendered: {}", rendered);

            let output_description = context.get(directory_key).unwrap().as_str().unwrap();
            let mut my_path = create_output_directory(&output_path, output_description);

            // use template filename as output filename
            let my_filename = Path::new(template_path.file_name().unwrap());
            my_path.push(my_filename);
            
            trace!("Writing output to {}", my_path.display());

            let mut file = File::create(my_path).expect("Failed opening output file");
            file.write_all(rendered.as_bytes())
                .map_err(|e| e.to_string())
                .unwrap();
        }
    } else {
        // expand input JSON via Completion API
        info!("Expanding JSON file: {:#?}", &opts.context);

        // if there are prompt templates, expand them via Tera then call OpenAI Completion API
        if let Some(prompt_template_map) = input_context.get("prompt_template_map") {
            // prepare output context with original context copied
            let mut output_context_list = Vec::<Map<String, Value>>::with_capacity(contexts.len());
            for context in contexts {
                let mut output_context_map = Map::<String, Value>::new();
                output_context_map.append(&mut context.as_object().unwrap().clone());
                output_context_list.push(output_context_map);
            }

            // setup rate limit
            let rate_limit = RateLimit::new(60, Duration::from_secs(60));
            let mut user_state = GcraState::default();

            for (key, prompt_template) in prompt_template_map.as_object().unwrap() {
                debug!("Prompt Template[{:#?}]: {:#?}", key, prompt_template);

                let prompt_str = prompt_template.get("prompt").unwrap().as_str().unwrap();
                trace!("Prompt[{}]: {}", key, prompt_str);

                let tokens = prompt_template.get("tokens").unwrap().as_u64().unwrap();
                trace!("Tokens[{}]: {}", key, tokens);

                let mut prompts = Vec::<String>::with_capacity(contexts.len());

                for (idx, context) in contexts.iter().enumerate() {
                    let tera_context: tera::Context = Context::from_value(context.to_owned())?;
                    trace!("Tera context[{}]: {:#?}", idx, tera_context);

                    let final_prompt = Tera::one_off(prompt_str, &tera_context, false).unwrap();

                    trace!("Prompt[{}] for context[{}]: {}", key, idx, final_prompt);

                    prompts.push(final_prompt);
                }

                // throttle api call
                while user_state.check_and_modify(&rate_limit, 1).is_err() {
                    trace!("Rate limited..sleeping");
                    thread::sleep(Duration::from_millis(1000));
                }

                // do actual openai api call in batches
                let completions = openai_completion_batch(prompts, tokens).expect("Failed to do Completion via OpenAI");

                for (idx, completion_str) in completions.iter().enumerate() {
                    let output_context_map = output_context_list.get_mut(idx).unwrap();
                    output_context_map.insert(key.to_owned(), completion_str.to_owned().into());
                }
            }

            trace!("Output context: {:#?}", output_context_list);

            // TODO: create output JSON and write to file

            let mut output_filename: PathBuf = opts.context.clone();
            output_filename.set_extension("content.json");
            info!("Writing output to: {:#?}", output_filename);
            let mut output_file =
                File::create(output_filename).expect("Unable to create output file");
            let map_objs: Vec<Value> = output_context_list
                .iter()
                .map(|m| Value::Object(m.to_owned()))
                .collect();
            let output_json = json!({ "contexts": Value::Array(map_objs) });
            let output_json_pretty =
                serde_json::to_string_pretty(&output_json).expect("prettify output json");
            output_file
                .write_all(output_json_pretty.as_bytes())
                .expect("Unable to write to output file");
        } else {
            info!("Missing 'prompt_template_map' in JSON. Nothing to do.");
        }
    };

    Ok(())
}

const OPANAI_BATCH_SIZE: usize = 20;

/// call openai completion sync api in batches of 20 prompts
fn openai_completion_batch(prompts: Vec<String>, tokens: u64) -> Result<Vec<String>> {

    let mut results: Vec<String> = Vec::<String>::with_capacity(prompts.len());

    for (batch_num, batch_prompts) in prompts.chunks(OPANAI_BATCH_SIZE).enumerate() {
        // construct CompletionArg with all prompts for current context
        let completion_args = CompletionArgs::builder()
                                                .model("text-curie-001")
                                                .max_tokens(tokens)
                                                .temperature(0.7)
                                                .prompt(batch_prompts)
                                                .build()
                                                .expect("Invalid Completion Prompt");

                                                   // call Completion API
        let completion = OPENAI_CLIENT.complete_prompt_sync(completion_args)?;
        trace!("Completion[{}]: {:#?}", batch_num, completion);

        for idx in 0..batch_prompts.len() {
            let completion_str = completion.choices[idx].text.trim();
            results.push(completion_str.to_owned());
        }
    }

    Ok(results)
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

    // register custom function to call openai completion api
    tera.register_function("openai_completion", openai_completion_tera_function);

    Ok(tera)
}

/// Call OpenAI Completion via Sync endpoint
fn openai_completion_tera_function(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    let tokens = args
        .get("tokens")
        .expect("No token count given")
        .as_u64()
        .unwrap();

    let prompt = args
        .get("prompt")
        .expect("No prompt given")
        .as_str()
        .unwrap();

    trace!("Prompt: {}", prompt);

    let completion_args = CompletionArgs::builder()
        .model("text-curie-001")
        .max_tokens(tokens)
        .temperature(0.7)
        .prompt(vec![prompt.to_owned()])
        .build()?;



    let completion = OPENAI_CLIENT.complete_prompt_sync(completion_args).unwrap();

    trace!("Completion: {:#?}", completion);

    Ok(completion.to_string().trim().into())
}

fn create_output_directory(output_path: &Path, output_description: &str) -> PathBuf {
    let re_non_alpha: Regex = Regex::new(r"\P{alpha}").unwrap();
    let re_spaces: Regex = Regex::new(r"[ ]+").unwrap();

    let stripped = re_non_alpha.replace_all(output_description, " ");
    let directory = re_spaces.replace_all(stripped.as_ref(), "_");

    trace!("Output Dir: '{}' -> '{}' -> '{}'", output_description, stripped, directory);

    let mut my_path: PathBuf = PathBuf::from(output_path);
    my_path.push(directory.as_ref());

    // create directory if not exist
    match fs::create_dir(&my_path) {
        Err(e) => {
            trace!("Unable to create directory {:#?}: {}", my_path, e);
        }
        Ok(()) => {
            trace!("Created Dir: {:#?}", my_path);
        }
    };

    my_path
}
