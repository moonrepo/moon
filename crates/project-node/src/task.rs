use lazy_static::lazy_static;
use moon_task::{Task, TaskID};
use moon_utils::{process, regex, string_vec};
use std::collections::HashMap;

lazy_static! {
    pub static ref ENV_VAR: regex::Regex = regex::create_regex(r#"^[A-Z0-9_]+="#).unwrap();
    pub static ref OUTPUT_FLAG: regex::Regex =
        regex::create_regex(r#"^(-o|--out-?(dir|file))$"#).unwrap();

    // This isn't exhaustive but captures very popular tools
    pub static ref DEV_COMMAND: regex::Regex =
        regex::create_regex(r#"(parcel (serve|watch))|(snowpack dev)|(vite (dev|preview|serve))|(webpack (s|serve|server|w|watch|-))"#)
            .unwrap();
    pub static ref DEV_COMMAND_SOLO: regex::Regex =
            regex::create_regex(r#"^vite$"#)
                .unwrap();
}

fn should_run_in_ci(script: &str) -> bool {
    if script.contains("--watch")
        || DEV_COMMAND.is_match(script)
        || DEV_COMMAND_SOLO.is_match(script)
    {
        return false;
    }

    true
}

fn clean_output_path(output: &str) -> String {
    if output.starts_with("..") {
        panic!("Cannot traverse upwards");
    } else if output.starts_with("./") || output.starts_with(".\\") {
        output[2..].to_owned()
    } else {
        output.to_owned()
    }
}

#[track_caller]
fn convert_script_to_task(script: &str) -> Result<Task, Box<dyn std::error::Error>> {
    let script_args = process::split_args(script)?;
    let mut task = Task::default();
    let mut args = vec![];

    for (index, arg) in script_args.iter().enumerate() {
        // Environment variables
        if ENV_VAR.is_match(arg) {
            let mut env_parts = arg.split('=');

            task.env.insert(
                env_parts.next().unwrap().to_string(),
                env_parts.next().unwrap().to_string(),
            );

            continue;
        }

        // Outputs
        if OUTPUT_FLAG.is_match(arg) {
            if let Some(output) = script_args.get(index) {
                task.outputs.push(clean_output_path(output));
            }
        }

        args.push(arg.to_owned());
    }

    task.command = args.pop().unwrap().to_owned();
    task.args = args;
    task.inputs = string_vec!["**/*"];
    task.options.run_in_ci = should_run_in_ci(script);

    Ok(task)
}

pub fn create_tasks_from_scripts() -> HashMap<TaskID, Task> {
    let mut tasks = HashMap::new();

    tasks
}
