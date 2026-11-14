use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::fmt::Write;
use std::fs;
use std::ops::RangeInclusive;
use std::path::PathBuf;

/// The test toolchains (built from `wasm/` with `just build-wasm`, and only
/// located in debug builds) that generated workspaces configure. Together they
/// cover tiers 1 to 3, requirements, and environment setup.
pub const GENERATED_TOOLCHAINS: [&str; 7] = [
    "tc-tier1",
    "tc-tier2",
    "tc-tier2-reqs",
    "tc-tier2-setup-env",
    "tc-tier3",
    "tc-tier3-reqs",
    "tc-tier3-setup-env",
];

/// A small seeded pseudo-random number generator (SplitMix64), so that
/// generated workspaces can be reproduced from their seed alone.
#[derive(Clone, Debug)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);

        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// A number in `0..max`, or 0 when `max` is 0.
    pub fn below(&mut self, max: usize) -> usize {
        if max == 0 {
            0
        } else {
            (self.next_u64() % max as u64) as usize
        }
    }

    /// A number in the inclusive range.
    pub fn between(&mut self, range: RangeInclusive<usize>) -> usize {
        let (min, max) = range.into_inner();

        min + self.below(max.saturating_sub(min) + 1)
    }

    /// True `percent` percent of the time.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Where the generated workspace is ran from. The tier 2 test toolchains
/// locate the dependencies root from the working directory, so this decides
/// which projects own an `InstallDependencies` (and `SetupEnvironment`) action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedWorkingDir {
    /// The workspace root, which is located as a lone package that no
    /// project is in, so every project falls back to toolchain setup.
    WorkspaceRoot,
    /// A project's root, which is located as a lone package owned by
    /// that project.
    Project,
    /// The root of a project named `in`, which is located as a dependencies
    /// workspace at the workspace root, with that project as its only member.
    DependenciesWorkspace,
}

#[derive(Clone, Debug)]
pub struct GeneratorOptions {
    /// How many projects to generate.
    pub projects: RangeInclusive<usize>,
    /// How many tasks to generate per project.
    pub tasks: RangeInclusive<usize>,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            projects: 3..=10,
            tasks: 1..=5,
        }
    }
}

pub struct GeneratedWorkspace {
    pub sandbox: Sandbox,
    pub seed: u64,
    /// Workspace-relative files to pass as changed files.
    pub changed_files: Vec<String>,
    /// Project ids, in generation order.
    pub projects: Vec<String>,
    /// Targets of every task, in generation order.
    pub targets: Vec<String>,
    /// Toolchain ids configured in `.moon/toolchains.yml`.
    pub toolchains: Vec<String>,
    /// The directory to run from (see [`GeneratedWorkingDir`]).
    pub working_dir: PathBuf,
    pub working_dir_kind: GeneratedWorkingDir,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunInCi {
    Enabled(bool),
    Always,
    Only,
    Skip,
}

impl RunInCi {
    fn as_yaml(&self) -> &'static str {
        match self {
            Self::Enabled(true) => "true",
            Self::Enabled(false) => "false",
            Self::Always => "'always'",
            Self::Only => "'only'",
            Self::Skip => "'skip'",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DepType {
    Required,
    Wait,
    Cleanup,
}

struct GeneratedDep {
    target: usize,
    type_of: DepType,
    args: Option<&'static str>,
    env: Option<&'static str>,
}

struct GeneratedTask {
    id: String,
    project: usize,
    index: usize,
    deps: Vec<GeneratedDep>,
    no_inputs: bool,
    persistent: bool,
    priority: Option<&'static str>,
    run_in_ci: Option<RunInCi>,
    serial: bool,
    toolchains: Option<Vec<&'static str>>,
}

impl GeneratedTask {
    // What the task builder infers when `runInCI` isn't set: persistent
    // tasks are disabled, while every task generated here is a test task
    fn effective_run_in_ci(&self) -> RunInCi {
        self.run_in_ci.unwrap_or(RunInCi::Enabled(!self.persistent))
    }
}

struct GeneratedProject {
    id: String,
    // Index of the depended on project, and the scope
    deps: Vec<(usize, &'static str)>,
    toolchains: Vec<String>,
    tasks: Vec<usize>,
}

/// Generate a workspace with the default options. See
/// [`create_generated_workspace_with`].
pub fn create_generated_workspace(seed: u64) -> GeneratedWorkspace {
    create_generated_workspace_with(seed, GeneratorOptions::default())
}

/// Generate a random, but valid, workspace from the seed, that exercises
/// every kind of node and edge in the action graph: projects with dependencies
/// in every scope (including cycles across scopes), tasks with required, wait,
/// and cleanup dependencies (and dependency variants with args and env),
/// serial parents (`runDepsInParallel: false`), persistent tasks, every
/// `runInCI` mode, priorities, and the `tc-*` test toolchains, with
/// requirements, environment setup, and project version overrides.
///
/// Every task is ranked by its project index, then its index within the project.
/// Required and wait dependencies only point at lower ranked tasks, of the same
/// project or of a production (or peer) dependency, which are always lower
/// indexed projects. Cleanup dependencies only point at higher ranked tasks, of
/// the same project or of a development (or build) dependency, which are always
/// higher indexed projects. As cleanup edges are reversed in the graphs, every
/// task edge points from a higher to a lower rank, so the task graph is acyclic,
/// while the unioned project graph may still contain cycles across scopes.
pub fn create_generated_workspace_with(seed: u64, options: GeneratorOptions) -> GeneratedWorkspace {
    let mut rng = SeededRng::new(seed);
    let project_count = rng.between(options.projects.clone()).max(2);

    // Projects
    let mut projects = (0..project_count)
        .map(|index| GeneratedProject {
            id: format!("p{index}"),
            deps: vec![],
            toolchains: vec![],
            tasks: vec![],
        })
        .collect::<Vec<_>>();

    let working_dir_kind = match rng.below(5) {
        0 | 1 => GeneratedWorkingDir::WorkspaceRoot,
        2 | 3 => GeneratedWorkingDir::Project,
        _ => GeneratedWorkingDir::DependenciesWorkspace,
    };
    let working_project = rng.below(project_count);

    if working_dir_kind == GeneratedWorkingDir::DependenciesWorkspace {
        projects[working_project].id = "in".into();
    }

    for (index, project) in projects.iter_mut().enumerate() {
        for dep_index in 0..project_count {
            if dep_index < index && rng.chance(30) {
                let scope = *rng.pick(&["production", "production", "peer"]);
                project.deps.push((dep_index, scope));
            } else if dep_index > index && rng.chance(15) {
                let scope = *rng.pick(&["development", "build"]);
                project.deps.push((dep_index, scope));
            }
        }
    }

    // Ensure most workspaces have a cycle across scopes
    if rng.chance(75) {
        let low = rng.below(project_count - 1);
        let high = low + 1 + rng.below(project_count - low - 1);

        if !projects[high].deps.iter().any(|(dep, _)| *dep == low) {
            projects[high].deps.push((low, "production"));
        }

        if !projects[low].deps.iter().any(|(dep, _)| *dep == high) {
            projects[low].deps.push((high, "development"));
        }
    }

    for project in &mut projects {
        project.deps.sort_by_key(|(dep, _)| *dep);

        // Override the version of a tier 3 toolchain, which is setup separately
        if rng.chance(20) {
            project.toolchains.push(format!(
                "{}:\n    version: '{}'",
                rng.pick(&["tc-tier3", "tc-tier3-reqs", "tc-tier3-setup-env"]),
                rng.pick(&["2.0.0", "3.1.0"]),
            ));
        }

        // Disable a toolchain that no other toolchain requires
        if rng.chance(10) {
            project
                .toolchains
                .push(format!("{}: false", rng.pick(&["tc-tier1", "tc-tier2"])));
        }
    }

    // Tasks
    let mut tasks = vec![];

    for (project_index, project) in projects.iter_mut().enumerate() {
        for index in 0..rng.between(options.tasks.clone()) {
            let persistent = rng.chance(15);

            let run_in_ci = if persistent {
                Some(*rng.pick(&[
                    RunInCi::Enabled(true),
                    RunInCi::Enabled(false),
                    RunInCi::Skip,
                ]))
            } else if rng.chance(45) {
                Some(*rng.pick(&[
                    RunInCi::Enabled(true),
                    RunInCi::Enabled(false),
                    RunInCi::Always,
                    RunInCi::Only,
                    RunInCi::Skip,
                ]))
            } else {
                None
            };

            let toolchains = if rng.chance(20) {
                None
            } else {
                let mut list = vec![];

                for _ in 0..rng.between(1..=2) {
                    let id = if rng.chance(5) {
                        "system"
                    } else {
                        *rng.pick(&GENERATED_TOOLCHAINS)
                    };

                    if !list.contains(&id) {
                        list.push(id);
                    }
                }

                Some(list)
            };

            project.tasks.push(tasks.len());

            tasks.push(GeneratedTask {
                id: format!("t{index}"),
                project: project_index,
                index,
                deps: vec![],
                no_inputs: rng.chance(15),
                persistent,
                priority: if rng.chance(10) {
                    Some(*rng.pick(&["critical", "high", "low"]))
                } else {
                    None
                },
                run_in_ci,
                serial: false,
                toolchains,
            });
        }
    }

    // Task dependencies
    for task_index in 0..tasks.len() {
        let task = &tasks[task_index];
        let project = &projects[task.project];
        let mut before = vec![];
        let mut after = vec![];

        for &other in &project.tasks {
            if tasks[other].index < task.index {
                before.push(other);
            } else if tasks[other].index > task.index {
                after.push(other);
            }
        }

        for &(dep_project, scope) in &project.deps {
            let list = if scope == "production" || scope == "peer" {
                &mut before
            } else {
                &mut after
            };

            list.extend(&projects[dep_project].tasks);
        }

        let mut deps: Vec<GeneratedDep> = vec![];

        for _ in 0..rng.between(0..=3) {
            let (target, mut type_of) = if !after.is_empty() && !task.persistent && rng.chance(20) {
                (*rng.pick(&after), DepType::Cleanup)
            } else if !before.is_empty() {
                (
                    *rng.pick(&before),
                    if rng.chance(25) {
                        DepType::Wait
                    } else {
                        DepType::Required
                    },
                )
            } else {
                continue;
            };

            let dep = &tasks[target];

            if !is_valid_dep(task, dep, &mut type_of) {
                continue;
            }

            match deps.iter().find(|existing| existing.target == target) {
                // The same target must always be depended on with the same type
                Some(existing) if existing.type_of != type_of => continue,
                Some(_) => {}
                None => {
                    deps.push(GeneratedDep {
                        target,
                        type_of,
                        args: None,
                        env: None,
                    });
                }
            };

            // Variants of the same dependency, which are separate nodes
            if rng.chance(15) {
                let args = rng.chance(50).then_some("--variant");
                let env = if args.is_none() || rng.chance(50) {
                    Some(*rng.pick(&["1", "2"]))
                } else {
                    None
                };

                if !deps.iter().any(|existing| {
                    existing.target == target && existing.args == args && existing.env == env
                }) {
                    deps.push(GeneratedDep {
                        target,
                        type_of,
                        args,
                        env,
                    });
                }
            }
        }

        let serial = deps.len() >= 2 && rng.chance(35);
        let task = &mut tasks[task_index];

        task.deps = deps;
        task.serial = serial;
    }

    // Changed files
    let mut changed_files = vec![];

    for project in &projects {
        if rng.chance(30) {
            changed_files.push(format!("{}/src/changed.txt", project.id));
        }
    }

    if changed_files.is_empty() {
        changed_files.push(format!("{}/src/changed.txt", rng.pick(&projects).id));
    }

    // Write the workspace
    let sandbox = create_empty_sandbox();
    let root = sandbox.path().to_path_buf();

    for project in &projects {
        let dir = root.join(&project.id);

        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/index.txt"), "index").unwrap();
        fs::write(
            dir.join("moon.yml"),
            render_project(project, &projects, &tasks),
        )
        .unwrap();
    }

    for file in &changed_files {
        fs::write(root.join(file), "changed").unwrap();
    }

    let moon_dir = root.join(".moon");

    fs::create_dir_all(&moon_dir).unwrap();
    fs::write(moon_dir.join("workspace.yml"), "projects: ['*']\n").unwrap();
    fs::write(moon_dir.join("toolchains.yml"), render_toolchains(&mut rng)).unwrap();

    let working_dir = match working_dir_kind {
        GeneratedWorkingDir::WorkspaceRoot => root.clone(),
        _ => root.join(&projects[working_project].id),
    };

    GeneratedWorkspace {
        changed_files,
        targets: tasks
            .iter()
            .map(|task| format!("{}:{}", projects[task.project].id, task.id))
            .collect(),
        projects: projects.into_iter().map(|project| project.id).collect(),
        sandbox,
        seed,
        toolchains: GENERATED_TOOLCHAINS
            .iter()
            .map(|id| id.to_string())
            .collect(),
        working_dir,
        working_dir_kind,
    }
}

/// Whether the task builder accepts the dependency, adjusting its type
/// when it can be made valid.
fn is_valid_dep(task: &GeneratedTask, dep: &GeneratedTask, type_of: &mut DepType) -> bool {
    match type_of {
        // Neither side of a cleanup can be persistent
        DepType::Cleanup => return !task.persistent && !dep.persistent,
        // Only a persistent task can require a persistent task,
        // but anything can wait on one
        DepType::Required if dep.persistent && !task.persistent => {
            *type_of = DepType::Wait;
        }
        _ => {}
    };

    // A task that runs in CI can't depend on a task that doesn't,
    // unless either is skipped in CI
    !(dep.effective_run_in_ci() == RunInCi::Enabled(false)
        && !matches!(
            task.effective_run_in_ci(),
            RunInCi::Enabled(false) | RunInCi::Skip
        ))
}

fn render_project(
    project: &GeneratedProject,
    projects: &[GeneratedProject],
    tasks: &[GeneratedTask],
) -> String {
    let mut out = String::new();

    if !project.deps.is_empty() {
        out.push_str("dependsOn:\n");

        for (dep, scope) in &project.deps {
            writeln!(out, "  - id: '{}'\n    scope: '{scope}'", projects[*dep].id).unwrap();
        }
    }

    if !project.toolchains.is_empty() {
        out.push_str("toolchains:\n");

        for toolchain in &project.toolchains {
            writeln!(out, "  {toolchain}").unwrap();
        }
    }

    if project.tasks.is_empty() {
        return out;
    }

    out.push_str("tasks:\n");

    for &index in &project.tasks {
        let task = &tasks[index];

        writeln!(out, "  {}:\n    command: 'echo {}'", task.id, task.id).unwrap();

        if let Some(toolchains) = &task.toolchains {
            writeln!(
                out,
                "    toolchains: [{}]",
                toolchains
                    .iter()
                    .map(|id| format!("'{id}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }

        if task.no_inputs {
            out.push_str("    inputs: []\n");
        }

        if !task.deps.is_empty() {
            out.push_str("    deps:\n");

            for dep in &task.deps {
                let dep_task = &tasks[dep.target];

                writeln!(
                    out,
                    "      - target: '{}:{}'",
                    projects[dep_task.project].id, dep_task.id
                )
                .unwrap();

                match dep.type_of {
                    DepType::Required => {}
                    DepType::Wait => out.push_str("        type: 'wait'\n"),
                    DepType::Cleanup => out.push_str("        type: 'cleanup'\n"),
                };

                if let Some(arg) = dep.args {
                    writeln!(out, "        args: ['{arg}']").unwrap();
                }

                if let Some(value) = dep.env {
                    writeln!(out, "        env:\n          GEN_VARIANT: '{value}'").unwrap();
                }
            }
        }

        let mut task_options = vec![];

        if task.persistent {
            task_options.push("persistent: true".to_owned());
        }

        if let Some(run_in_ci) = &task.run_in_ci {
            task_options.push(format!("runInCI: {}", run_in_ci.as_yaml()));
        }

        if task.serial {
            task_options.push("runDepsInParallel: false".to_owned());
        }

        if let Some(priority) = task.priority {
            task_options.push(format!("priority: '{priority}'"));
        }

        if !task_options.is_empty() {
            out.push_str("    options:\n");

            for option in task_options {
                writeln!(out, "      {option}").unwrap();
            }
        }
    }

    out
}

fn render_toolchains(rng: &mut SeededRng) -> String {
    let mut out = String::new();

    for id in GENERATED_TOOLCHAINS {
        let mut settings = vec![];

        match id {
            "tc-tier2" if rng.chance(15) => {
                settings.push("installDependencies: false".to_owned());
            }
            "tc-tier2-reqs" if rng.chance(40) => {
                settings.push("testRequiresForEnvironment: true".to_owned());
            }
            // Requirements only point from tier 2 to tier 3 setup env, and from
            // tier 3 setup env to tier 3, so they never form a cycle
            "tc-tier2-setup-env" => match rng.below(4) {
                1 => settings.push("testRequirements: ['tc-tier3-setup-env']".to_owned()),
                2 => settings.push("testEnvRequirements: ['tc-tier3-setup-env']".to_owned()),
                3 => settings
                    .push("testEnvRequirements: ['tc-tier3-setup-env', 'tc-tier3']".to_owned()),
                _ => {}
            },
            "tc-tier3-setup-env" => match rng.below(3) {
                1 => settings.push("testRequirements: ['tc-tier3']".to_owned()),
                2 => settings.push("testEnvRequirements: ['tc-tier3']".to_owned()),
                _ => {}
            },
            _ => {}
        };

        if settings.is_empty() {
            writeln!(out, "{id}: {{}}").unwrap();
        } else {
            writeln!(out, "{id}:").unwrap();

            for setting in settings {
                writeln!(out, "  {setting}").unwrap();
            }
        }
    }

    out
}
