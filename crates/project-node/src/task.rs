use lazy_static::lazy_static;
use moon_task::{Task, TaskError, TaskID, TaskType};
use moon_utils::{process, regex};
use std::collections::HashMap;

lazy_static! {
    pub static ref WIN_DRIVE: regex::Regex = regex::create_regex(r#"^[A-Z]:"#).unwrap();

    pub static ref ARG_ENV_VAR: regex::Regex = regex::create_regex(r#"^[A-Z0-9_]+="#).unwrap();

    pub static ref ARG_OUTPUT_FLAG: regex::Regex =
        regex::create_regex(r#"^(-o|--(out|output|dist)(-{0,1}(?i:dir|file))?)$"#).unwrap();

    pub static ref INFO_OPTIONS: regex::Regex =
        regex::create_regex(r#"--(help|version)"#)
            .unwrap();

    // This isn't exhaustive but captures very popular tools
    pub static ref DEV_COMMAND: regex::Regex =
        regex::create_regex(r#"(gatsby (new|dev|develop|serve|repl))|(next (dev|start))|(parcel [^build])|(react-scripts start)|(snowpack dev)|(vite (dev|preview|serve))|(vue-cli-service serve)|(webpack (s|serve|server|w|watch|-))"#)
            .unwrap();

    pub static ref DEV_COMMAND_SOLO: regex::Regex =
            regex::create_regex(r#"^(npx |yarn dlx |pnpm dlx )?(parcel|vite|webpack)$"#)
                .unwrap();
}

fn is_bash_script(arg: &str) -> bool {
    arg.ends_with(".sh")
}

fn is_node_script(arg: &str) -> bool {
    arg.ends_with(".js") || arg.ends_with(".cjs") || arg.ends_with(".mjs")
}

pub fn should_run_in_ci(script: &str) -> bool {
    if INFO_OPTIONS.is_match(script) {
        return true;
    }

    if script.contains("--watch")
        || DEV_COMMAND.is_match(script)
        || DEV_COMMAND_SOLO.is_match(script)
    {
        return false;
    }

    true
}

fn clean_env_var(pair: &str) -> (String, String) {
    let mut parts = pair.split('=');
    let key = parts.next().unwrap();
    let mut val = parts.next().unwrap_or_default();

    if val.ends_with(';') {
        val = &val[0..(val.len() - 1)];
    }

    (key.to_owned(), val.to_owned())
}

fn clean_output_path(target_id: &str, output: &str) -> Result<String, TaskError> {
    if output.starts_with("..") {
        Err(TaskError::NoParentOutput(
            output.to_owned(),
            target_id.to_owned(),
        ))
    } else if output.starts_with('/') || WIN_DRIVE.is_match(output) {
        Err(TaskError::NoAbsoluteOutput(
            output.to_owned(),
            target_id.to_owned(),
        ))
    } else if output.starts_with("./") || output.starts_with(".\\") {
        Ok(output[2..].to_owned())
    } else {
        Ok(output.to_owned())
    }
}

fn detect_task_type(command: &str) -> TaskType {
    if command == "bash" || command == "noop" {
        return TaskType::System;
    }

    TaskType::Node
}

#[track_caller]
pub fn convert_script_to_task(target_id: &str, script: &str) -> Result<Task, TaskError> {
    let script_args = process::split_args(script)?;
    let mut task = Task::new(target_id.to_owned());
    let mut args = vec![];

    for (index, arg) in script_args.iter().enumerate() {
        // Extract nvironment variables
        if ARG_ENV_VAR.is_match(arg) {
            let (key, val) = clean_env_var(arg);

            task.env.insert(key, val);

            continue;
        }

        // Detect possible outputs
        if ARG_OUTPUT_FLAG.is_match(arg) {
            if let Some(output) = script_args.get(index + 1) {
                task.outputs.push(clean_output_path(target_id, output)?);
            }
        }

        args.push(arg.to_owned());
    }

    if let Some(command) = args.get(0) {
        if is_bash_script(command) {
            task.command = "bash".to_owned();
        } else if is_node_script(command) {
            task.command = "node".to_owned();
        } else {
            task.command = args.remove(0);
        }
    } else {
        task.command = "noop".to_owned();
    }

    task.args = args;
    task.type_of = detect_task_type(&task.command);
    task.options.run_in_ci = should_run_in_ci(script);

    Ok(task)
}

pub fn create_tasks_from_scripts() -> HashMap<TaskID, Task> {
    let mut tasks = HashMap::new();

    tasks
}
