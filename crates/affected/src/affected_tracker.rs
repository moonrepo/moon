use crate::affected::*;
use moon_async_utils::{get_concurrency, run_pooled_tasks};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_env_var::GlobalEnvBag;
use moon_graph_utils::{Depth, Direction, GraphConnections};
use moon_project::Project;
use moon_task::{Target, Task, TaskOptionRunInCI};
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::fs::{self, FsError};
use starbase_utils::glob::GlobSet;
use std::collections::VecDeque;
use std::fmt;
use std::io;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct AffectedTracker {
    ci: bool,

    workspace_graph: Arc<WorkspaceGraph>,
    changed_files: Arc<FxHashSet<WorkspaceRelativePathBuf>>,
    cache: Arc<CheckCache>,

    /// How many hops away from an affected project or task its dependents
    /// are also marked as affected.
    depth: Depth,

    projects: FxHashMap<Id, FxHashSet<AffectedBy>>,

    /// Projects that have been checked and are not directly affected by
    /// the changed files. They may still be marked through a relationship,
    /// but don't need to be checked again.
    unaffected_projects: FxHashSet<Id>,

    tasks: FxHashMap<Target, FxHashSet<AffectedBy>>,

    /// Tasks that have been checked and are not directly affected by the
    /// changed files, environment variables, or CI. They may still be marked
    /// through a relationship, but don't need to be checked again.
    unaffected_tasks: FxHashSet<Target>,
}

impl AffectedTracker {
    pub fn new(
        workspace_graph: Arc<WorkspaceGraph>,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> Self {
        debug!("Creating affected tracker");

        Self {
            workspace_graph,
            changed_files: Arc::new(changed_files),
            cache: Arc::new(CheckCache::default()),
            depth: Depth::Deep,
            projects: FxHashMap::default(),
            unaffected_projects: FxHashSet::default(),
            tasks: FxHashMap::default(),
            unaffected_tasks: FxHashSet::default(),
            ci: false,
        }
    }

    pub fn build(self) -> Affected {
        let mut affected = Affected::default();

        if self.projects.is_empty() && self.tasks.is_empty() {
            debug!("No affected projects or tasks");
        }

        for (id, list) in self.projects {
            let state = AffectedProjectState::from(list);

            debug!(
                files = ?state.files.iter().collect::<Vec<_>>(),
                dependencies = ?state.dependencies.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
                distance = state.distance,
                tasks = ?state.tasks.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
                other = state.other,
                "Project {} is affected by", color::id(&id),
            );

            affected.projects.insert(id, state);
        }

        for (target, list) in self.tasks {
            let state = AffectedTaskState::from(list);

            debug!(
                env = ?state.env.iter().collect::<Vec<_>>(),
                files = ?state.files.iter().collect::<Vec<_>>(),
                dependencies = ?state.dependencies.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
                distance = state.distance,
                other = state.other,
                "Task {} is affected by", color::id(&target),
            );

            affected.tasks.insert(target, state);
        }

        affected.should_check = !self.changed_files.is_empty();
        affected
    }

    pub fn set_ci_check(&mut self, ci: bool) -> &mut Self {
        self.ci = ci;
        self
    }

    /// Set how many hops away from an affected project or task its
    /// dependents are also marked as affected. Defaults to deep.
    pub fn set_depth(&mut self, depth: Depth) -> &mut Self {
        debug!(depth = %depth, "Setting affected depth");

        self.depth = depth;
        self
    }

    pub async fn track_projects(&mut self) -> miette::Result<&mut Self> {
        debug!("Tracking projects and marking any affected");

        let projects = self.workspace_graph.get_projects()?;

        // Index projects by their source directory, so that each changed
        // file only needs to look up its ancestor directories, instead of
        // every project scanning every changed file
        let mut root_indexes = vec![];
        let mut source_indexes: FxHashMap<&str, Vec<usize>> = FxHashMap::default();

        for (index, project) in projects.iter().enumerate() {
            if project.is_root_level() {
                root_indexes.push(index);
            } else {
                source_indexes
                    .entry(project.source.as_str().trim_end_matches('/'))
                    .or_default()
                    .push(index);
            }
        }

        // A project is affected by the first changed file found within it
        let mut matched_files: Vec<Option<&WorkspaceRelativePathBuf>> = vec![None; projects.len()];

        for file in self.changed_files.iter() {
            // If at the root, any file affects it
            if !file.as_str().starts_with('.') {
                for index in &root_indexes {
                    matched_files[*index].get_or_insert(file);
                }
            }

            let mut dir = file.parent();

            while let Some(current) = dir {
                if current.as_str().is_empty() {
                    break;
                }

                if let Some(indexes) = source_indexes.get(current.as_str()) {
                    for index in indexes {
                        matched_files[*index].get_or_insert(file);
                    }
                }

                dir = current.parent();
            }
        }

        let mut affected_projects = vec![];

        for (index, file) in matched_files.into_iter().enumerate() {
            match file {
                Some(file) => affected_projects.push((
                    Arc::clone(&projects[index]),
                    AffectedBy::ChangedFile(file.to_owned()),
                )),
                None => {
                    self.unaffected_projects.insert(projects[index].id.clone());
                }
            };
        }

        self.mark_projects_affected(affected_projects)?;

        Ok(self)
    }

    pub fn is_project_affected(&mut self, project: &Project) -> Option<AffectedBy> {
        if self.is_project_marked_ignoring_relations(project) {
            return Some(AffectedBy::AlreadyMarked);
        }

        // Was already checked, so don't check again
        if self.unaffected_projects.contains(&project.id) {
            return None;
        }

        let affected = is_project_directly_affected(project, &self.changed_files);

        if affected.is_none() {
            self.unaffected_projects.insert(project.id.clone());
        }

        affected
    }

    pub fn is_project_marked(&self, project: &Project) -> bool {
        self.projects.contains_key(&project.id)
    }

    pub fn is_project_marked_ignoring_relations(&self, project: &Project) -> bool {
        self.projects.get(&project.id).is_some_and(|by_list| {
            by_list.iter().any(|by| {
                matches!(
                    by,
                    AffectedBy::AlwaysAffected
                        | AffectedBy::ChangedFile(_)
                        | AffectedBy::EnvironmentVariable(_)
                )
            })
        })
    }

    pub fn mark_project_affected(
        &mut self,
        project: &Project,
        affected: AffectedBy,
    ) -> miette::Result<()> {
        // May have been already marked through a dependency, but that
        // doesn't mean its own dependents have been marked!
        if affected != AffectedBy::AlreadyMarked {
            trace!(
                project_id = project.id.as_str(),
                "Marking project as affected"
            );

            self.projects
                .entry(project.id.clone())
                .or_default()
                .insert(affected);
        }

        self.track_project_dependents(&[project]);

        Ok(())
    }

    fn mark_projects_affected(
        &mut self,
        projects: Vec<(Arc<Project>, AffectedBy)>,
    ) -> miette::Result<()> {
        let mut marked = Vec::with_capacity(projects.len());

        for (project, affected) in projects {
            trace!(
                project_id = project.id.as_str(),
                "Marking project as affected"
            );

            self.projects
                .entry(project.id.clone())
                .or_default()
                .insert(affected);

            marked.push(project);
        }

        let marked = marked
            .iter()
            .map(|project| project.as_ref())
            .collect::<Vec<_>>();

        self.track_project_dependents(&marked);

        Ok(())
    }

    /// Walk the dependents of the provided projects, which have all been
    /// marked as affected, and mark them as affected, up to the configured
    /// depth. The walk starts from every project at once, so each dependent
    /// is visited once, at its minimum distance from any of them, and is
    /// marked through the project it was reached from.
    fn track_project_dependents(&mut self, projects: &[&Project]) {
        if self.depth.is_none() {
            trace!("Not tracking project dependents as depth is none");

            return;
        }

        if projects.is_empty() {
            return;
        }

        trace!(
            project_ids = ?projects.iter().map(|project| project.id.as_str()).collect::<Vec<_>>(),
            depth = %self.depth,
            "Tracking project dependents",
        );

        let workspace_graph = Arc::clone(&self.workspace_graph);
        let marked = &mut self.projects;

        workspace_graph.projects.traverse_within(
            projects,
            Direction::Incoming,
            self.depth,
            |project, via, distance| {
                marked.entry(project.id.clone()).or_default().insert(
                    AffectedBy::DependencyProject {
                        id: via.id.clone(),
                        distance,
                    },
                );
            },
        );
    }

    pub async fn track_tasks(&mut self) -> miette::Result<()> {
        // Include internal since they can trigger affected for any dependents!
        let tasks = self.workspace_graph.get_tasks_with_internal()?;

        self.track_tasks_by_instance(&tasks).await
    }

    pub async fn track_tasks_by_instance(&mut self, tasks: &[Arc<Task>]) -> miette::Result<()> {
        debug!("Tracking tasks and marking any affected");

        let ci = self.ci;
        let changed_files = Arc::clone(&self.changed_files);
        let workspace_graph = Arc::clone(&self.workspace_graph);
        let cache = Arc::clone(&self.cache);
        let mut affected_tasks = vec![];

        // Check every task in parallel, as each check is independent, then
        // mark them and walk their relationships all at once. Checks are cheap
        // enough that spawning one per task would cost more than the check
        // itself, so they're batched, with enough batches to fill the pool
        let batch_size = tasks.len().div_ceil(get_concurrency() * 4).max(1);

        run_pooled_tasks(
            VecDeque::from_iter(tasks.chunks(batch_size).map(|batch| batch.to_vec())),
            |batch| {
                let changed_files = Arc::clone(&changed_files);
                let workspace_graph = Arc::clone(&workspace_graph);
                let cache = Arc::clone(&cache);

                Ok(async move {
                    let mut results = Vec::with_capacity(batch.len());

                    for task in batch {
                        let affected = is_task_directly_affected(
                            &task,
                            &changed_files,
                            &workspace_graph.root,
                            ci,
                            &cache,
                        )?;

                        results.push((task, affected));
                    }

                    Ok(results)
                })
            },
            |results| {
                for (task, affected) in results {
                    match affected {
                        Some(affected) => affected_tasks.push((task, affected)),
                        None => {
                            self.unaffected_tasks.insert(task.target.clone());
                        }
                    };
                }

                Ok(())
            },
        )
        .await?;

        self.mark_tasks_affected(affected_tasks)
    }

    pub async fn track_tasks_by_target(&mut self, targets: &[Target]) -> miette::Result<()> {
        let mut tasks = Vec::with_capacity(targets.len());

        for target in targets {
            tasks.push(self.workspace_graph.get_task(target)?);
        }

        self.track_tasks_by_instance(&tasks).await
    }

    pub fn is_task_affected(&mut self, task: &Task) -> miette::Result<Option<AffectedBy>> {
        if self.is_task_marked_ignoring_relations(task) {
            return Ok(Some(AffectedBy::AlreadyMarked));
        }

        // Was already checked, so don't check again
        if self.unaffected_tasks.contains(&task.target) {
            return Ok(None);
        }

        let affected = is_task_directly_affected(
            task,
            &self.changed_files,
            &self.workspace_graph.root,
            self.ci,
            &self.cache,
        )?;

        if affected.is_none() {
            self.unaffected_tasks.insert(task.target.clone());
        }

        Ok(affected)
    }

    pub fn is_task_marked(&self, task: &Task) -> bool {
        self.tasks.contains_key(&task.target)
    }

    pub fn is_task_marked_ignoring_relations(&self, task: &Task) -> bool {
        self.tasks.get(&task.target).is_some_and(|by_list| {
            by_list.iter().any(|by| {
                matches!(
                    by,
                    AffectedBy::AlwaysAffected
                        | AffectedBy::ChangedFile(_)
                        | AffectedBy::EnvironmentVariable(_)
                )
            })
        })
    }

    pub fn mark_task_affected(&mut self, task: &Task, affected: AffectedBy) -> miette::Result<()> {
        // May have been already marked through a dependency, but that
        // doesn't mean its own dependents have been marked!
        if affected != AffectedBy::AlreadyMarked {
            trace!(
                task_target = task.target.as_str(),
                "Marking task as affected"
            );

            self.tasks
                .entry(task.target.clone())
                .or_default()
                .insert(affected);

            self.mark_task_project_affected(task);
        }

        self.track_task_dependents(&[task]);

        Ok(())
    }

    fn mark_tasks_affected(&mut self, tasks: Vec<(Arc<Task>, AffectedBy)>) -> miette::Result<()> {
        let mut marked = Vec::with_capacity(tasks.len());

        for (task, affected) in tasks {
            trace!(
                task_target = task.target.as_str(),
                "Marking task as affected"
            );

            self.tasks
                .entry(task.target.clone())
                .or_default()
                .insert(affected);

            self.mark_task_project_affected(&task);

            marked.push(task);
        }

        let marked = marked.iter().map(|task| task.as_ref()).collect::<Vec<_>>();

        self.track_task_dependents(&marked);

        Ok(())
    }

    /// The owning project is affected by its affected tasks.
    fn mark_task_project_affected(&mut self, task: &Task) {
        if let Ok(project_id) = task.target.get_project_id() {
            let affected = AffectedBy::Task(task.target.clone());

            // Only allocate an ID when the project hasn't been marked yet
            match self.projects.get_mut(project_id) {
                Some(by_list) => {
                    by_list.insert(affected);
                }
                None => {
                    self.projects
                        .entry(Id::raw(project_id))
                        .or_default()
                        .insert(affected);
                }
            };
        }
    }

    /// Walk the dependents of the provided tasks, which have all been
    /// marked as affected, and mark them as affected, up to the configured
    /// depth. The walk starts from every task at once, so each dependent
    /// is visited once, at its minimum distance from any of them, and is
    /// marked through the task it was reached from.
    fn track_task_dependents(&mut self, tasks: &[&Task]) {
        if self.depth.is_none() {
            trace!("Not tracking task dependents as depth is none");

            return;
        }

        if tasks.is_empty() {
            return;
        }

        trace!(
            task_targets = ?tasks.iter().map(|task| task.target.as_str()).collect::<Vec<_>>(),
            depth = %self.depth,
            "Tracking task dependents",
        );

        let workspace_graph = Arc::clone(&self.workspace_graph);
        let marked = &mut self.tasks;

        workspace_graph.tasks.traverse_within(
            tasks,
            Direction::Incoming,
            self.depth,
            |task, via, distance| {
                marked
                    .entry(task.target.clone())
                    .or_default()
                    .insert(AffectedBy::DependencyTask {
                        target: via.target.clone(),
                        distance,
                    });
            },
        );
    }
}

impl fmt::Debug for AffectedTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AffectedTracker")
            .field("changed_files", &self.changed_files)
            .field("depth", &self.depth)
            .field("projects", &self.projects)
            .field("unaffected_projects", &self.unaffected_projects)
            .field("tasks", &self.tasks)
            .field("unaffected_tasks", &self.unaffected_tasks)
            .finish()
    }
}

fn is_project_directly_affected(
    project: &Project,
    changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
) -> Option<AffectedBy> {
    let file = if project.is_root_level() {
        // If at the root, any visible file affects it
        changed_files
            .iter()
            .find(|file| !file.as_str().starts_with('.'))
    } else {
        changed_files
            .iter()
            .find(|file| file.starts_with(&project.source))
    };

    file.map(|file| AffectedBy::ChangedFile(file.to_owned()))
}

fn is_task_directly_affected(
    task: &Task,
    changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
    workspace_root: &Path,
    ci: bool,
    cache: &CheckCache,
) -> miette::Result<Option<AffectedBy>> {
    // Special CI handling
    match (ci, &task.options.run_in_ci) {
        (true, TaskOptionRunInCI::Always) => {
            return Ok(Some(AffectedBy::AlwaysAffected));
        }
        (true, TaskOptionRunInCI::Enabled(false))
        | (true, TaskOptionRunInCI::Skip)
        | (false, TaskOptionRunInCI::Only) => {
            return Ok(None);
        }
        _ => {}
    };

    // Never affected
    if task.state.empty_inputs {
        return Ok(None);
    }

    // By env vars
    if !task.input_env.is_empty() {
        let bag = GlobalEnvBag::instance();

        for var_name in &task.input_env {
            // Only check for a non-empty value, instead of allocating a copy of it
            if bag.get_as(var_name, |value| !value.is_empty()) == Some(true) {
                return Ok(Some(AffectedBy::EnvironmentVariable(var_name.to_owned())));
            }
        }
    }

    // By files
    if task.input_files.is_empty() && task.input_globs.is_empty() {
        return Ok(None);
    }

    // Only match globs when there are input globs, as nothing can match
    // without them, and resolving the glob set is not free
    let glob_matches = if task.input_globs.is_empty() {
        None
    } else {
        Some(cache.match_globs(&task.create_globset()?, changed_files))
    };

    for file in changed_files {
        let affected = if let Some(params) = task.input_files.get(file) {
            match &params.content {
                Some(matcher) => match cache.read_file(workspace_root, file)? {
                    Some(contents) => matcher.is_match(&contents),
                    None => false,
                },
                None => true,
            }
        } else {
            glob_matches
                .as_ref()
                .is_some_and(|matches| matches.contains(file))
        };

        if affected {
            return Ok(Some(AffectedBy::ChangedFile(file.to_owned())));
        }
    }

    Ok(None)
}

/// Caches shared between all task checks within a tracking pass.
#[derive(Default)]
struct CheckCache {
    /// Contents of changed files that are matched by content. Many tasks
    /// may match against the same file, so it's only read once.
    file_contents: scc::HashMap<WorkspaceRelativePathBuf, Option<Arc<String>>>,

    /// The changed files that each compiled glob set matches, keyed by the
    /// set's address, as sets are shared between tasks with the same patterns.
    /// Every task matches the same files, so each set only matches them once.
    glob_matches: scc::HashMap<usize, Arc<FxHashSet<WorkspaceRelativePathBuf>>>,
}

impl CheckCache {
    fn match_globs(
        &self,
        globset: &Arc<GlobSet<'static>>,
        changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
    ) -> Arc<FxHashSet<WorkspaceRelativePathBuf>> {
        let key = Arc::as_ptr(globset).addr();

        if let Some(matches) = self
            .glob_matches
            .read_sync(&key, |_, matches| Arc::clone(matches))
        {
            return matches;
        }

        let matches = Arc::new(
            changed_files
                .iter()
                .filter(|file| globset.matches(file.as_str()))
                .cloned()
                .collect::<FxHashSet<_>>(),
        );

        let _ = self.glob_matches.insert_sync(key, Arc::clone(&matches));

        matches
    }

    fn read_file(
        &self,
        workspace_root: &Path,
        file: &WorkspaceRelativePathBuf,
    ) -> miette::Result<Option<Arc<String>>> {
        if let Some(contents) = self
            .file_contents
            .read_sync(file, |_, contents| contents.clone())
        {
            return Ok(contents);
        }

        let contents = match fs::read_file(file.to_logical_path(workspace_root)) {
            Ok(contents) => Some(Arc::new(contents)),
            // A deleted file is still a changed file, but has nothing to match
            Err(FsError::Read { error, .. }) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };

        let _ = self
            .file_contents
            .insert_sync(file.to_owned(), contents.clone());

        Ok(contents)
    }
}
