use crate::affected::*;
use moon_async_utils::run_pooled_tasks;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_env_var::GlobalEnvBag;
use moon_project::Project;
use moon_task::{Target, Task, TaskOptionRunInCI};
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::fs;
use std::collections::VecDeque;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct AffectedTracker {
    ci: bool,

    workspace_graph: Arc<WorkspaceGraph>,
    changed_files: Arc<FxHashSet<WorkspaceRelativePathBuf>>,

    projects: FxHashMap<Id, FxHashSet<AffectedBy>>,
    project_downstream: DownstreamScope,
    project_upstream: UpstreamScope,

    /// Projects that have been checked and are not directly affected by
    /// the changed files. They may still be marked through a relationship,
    /// but don't need to be checked again.
    unaffected_projects: FxHashSet<Id>,

    tasks: FxHashMap<Target, FxHashSet<AffectedBy>>,
    task_downstream: DownstreamScope,
    task_upstream: UpstreamScope,

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
            projects: FxHashMap::default(),
            project_downstream: DownstreamScope::None,
            project_upstream: UpstreamScope::Deep,
            unaffected_projects: FxHashSet::default(),
            tasks: FxHashMap::default(),
            task_downstream: DownstreamScope::None,
            task_upstream: UpstreamScope::Deep,
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
                upstream = ?state.upstream.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
                downstream = ?state.downstream.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
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
                projects = ?state.projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
                upstream = ?state.upstream.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
                downstream = ?state.downstream.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
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

    pub fn set_project_scopes(
        &mut self,
        upstream_scope: UpstreamScope,
        downstream_scope: DownstreamScope,
    ) -> &mut Self {
        debug!(
            upstream = %upstream_scope,
            downstream = %downstream_scope,
            "Setting project relationship scopes"
        );

        self.project_upstream = upstream_scope;
        self.project_downstream = downstream_scope;
        self
    }

    pub fn set_task_scopes(
        &mut self,
        upstream_scope: UpstreamScope,
        downstream_scope: DownstreamScope,
    ) -> &mut Self {
        debug!(
            upstream = %upstream_scope,
            downstream = %downstream_scope,
            "Setting task relationship scopes"
        );

        self.task_upstream = upstream_scope;
        self.task_downstream = downstream_scope;
        self
    }

    pub fn set_scopes(
        &mut self,
        upstream_scope: UpstreamScope,
        downstream_scope: DownstreamScope,
    ) -> &mut Self {
        self.set_project_scopes(upstream_scope, downstream_scope);
        self.set_task_scopes(upstream_scope, downstream_scope);
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
        if affected == AffectedBy::AlreadyMarked {
            // May have been already marked through an indirect dep,
            // but that doesn't mean its own deps have been checked!
            return self.track_project_relations(&[project]);
        }

        trace!(
            project_id = project.id.as_str(),
            "Marking project as affected"
        );

        self.projects
            .entry(project.id.clone())
            .or_default()
            .insert(affected);

        self.track_project_relations(&[project])
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

        self.track_project_relations(&marked)
    }

    /// Walk the relationships of the provided projects, which have all been
    /// marked as affected, and mark their dependencies and dependents. Every
    /// project is visited once, no matter how many of the provided projects
    /// reach it, so the work is bound by the size of the graph.
    fn track_project_relations(&mut self, projects: &[&Project]) -> miette::Result<()> {
        self.track_project_dependencies(projects)?;
        self.track_project_dependents(projects)?;

        Ok(())
    }

    fn track_project_dependencies(&mut self, projects: &[&Project]) -> miette::Result<()> {
        if self.project_upstream == UpstreamScope::None {
            trace!("Not tracking project dependencies as upstream scope is none");

            return Ok(());
        }

        let deep = self.project_upstream == UpstreamScope::Deep;
        let mut visited = FxHashSet::default();
        let mut queue = VecDeque::new();

        for project in projects {
            trace!(
                project_id = project.id.as_str(),
                "Tracking {} project dependencies",
                if deep { "deep" } else { "direct" }
            );

            self.visit_project_dependencies(project, deep, &mut visited, &mut queue);
        }

        while let Some(id) = queue.pop_front() {
            if visited.contains(&id) {
                continue;
            }

            let project = self.workspace_graph.get_project(&id)?;

            self.visit_project_dependencies(&project, deep, &mut visited, &mut queue);
        }

        Ok(())
    }

    fn visit_project_dependencies(
        &mut self,
        project: &Project,
        deep: bool,
        visited: &mut FxHashSet<Id>,
        queue: &mut VecDeque<Id>,
    ) {
        if !visited.insert(project.id.clone()) {
            return;
        }

        for dep_config in &project.dependencies {
            self.projects
                .entry(dep_config.id.clone())
                .or_default()
                .insert(AffectedBy::DownstreamProject(project.id.clone()));

            if deep && !visited.contains(&dep_config.id) {
                queue.push_back(dep_config.id.clone());
            }
        }
    }

    fn track_project_dependents(&mut self, projects: &[&Project]) -> miette::Result<()> {
        if self.project_downstream == DownstreamScope::None {
            trace!("Not tracking project dependents as downstream scope is none");

            return Ok(());
        }

        let deep = self.project_downstream == DownstreamScope::Deep;
        let mut visited = FxHashSet::default();
        let mut queue = VecDeque::new();

        for project in projects {
            trace!(
                project_id = project.id.as_str(),
                "Tracking {} project dependents",
                if deep { "deep" } else { "direct" }
            );

            self.visit_project_dependents(project, deep, &mut visited, &mut queue);
        }

        while let Some(id) = queue.pop_front() {
            if visited.contains(&id) {
                continue;
            }

            let project = self.workspace_graph.get_project(&id)?;

            self.visit_project_dependents(&project, deep, &mut visited, &mut queue);
        }

        Ok(())
    }

    fn visit_project_dependents(
        &mut self,
        project: &Project,
        deep: bool,
        visited: &mut FxHashSet<Id>,
        queue: &mut VecDeque<Id>,
    ) {
        if !visited.insert(project.id.clone()) {
            return;
        }

        for dep_id in self.workspace_graph.projects.dependents_of(project) {
            self.projects
                .entry(dep_id.clone())
                .or_default()
                .insert(AffectedBy::UpstreamProject(project.id.clone()));

            if deep && !visited.contains(&dep_id) {
                queue.push_back(dep_id);
            }
        }
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
        let mut affected_tasks = vec![];

        // Check every task in parallel, as each check is independent (and
        // expensive), then mark them and walk their relationships all at once
        run_pooled_tasks(
            VecDeque::from_iter(tasks.iter().cloned()),
            |task| {
                let changed_files = Arc::clone(&changed_files);
                let workspace_graph = Arc::clone(&workspace_graph);

                Ok(async move {
                    let affected = is_task_directly_affected(
                        &task,
                        &changed_files,
                        &workspace_graph.root,
                        ci,
                    )?;

                    Ok((task, affected))
                })
            },
            |(task, affected)| {
                match affected {
                    Some(affected) => affected_tasks.push((task, affected)),
                    None => {
                        self.unaffected_tasks.insert(task.target.clone());
                    }
                };

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
        if affected == AffectedBy::AlreadyMarked {
            // May have been already marked through an indirect dep,
            // but that doesn't mean its own deps have been checked!
            return self.track_task_relations(&[task]);
        }

        trace!(
            task_target = task.target.as_str(),
            "Marking task as affected"
        );

        self.tasks
            .entry(task.target.clone())
            .or_default()
            .insert(affected);

        self.mark_task_project_affected(task);
        self.track_task_relations(&[task])
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

        self.track_task_relations(&marked)
    }

    /// The owning project is affected by its affected tasks.
    fn mark_task_project_affected(&mut self, task: &Task) {
        if let Ok(project_id) = task.target.get_project_id() {
            self.projects
                .entry(Id::raw(project_id))
                .or_default()
                .insert(AffectedBy::Task(task.target.clone()));
        }
    }

    /// Walk the relationships of the provided tasks, which have all been
    /// marked as affected, and mark their dependencies and dependents. Every
    /// task is visited once, no matter how many of the provided tasks reach
    /// it, so the work is bound by the size of the graph.
    fn track_task_relations(&mut self, tasks: &[&Task]) -> miette::Result<()> {
        self.track_task_dependencies(tasks)?;
        self.track_task_dependents(tasks)?;

        Ok(())
    }

    fn track_task_dependencies(&mut self, tasks: &[&Task]) -> miette::Result<()> {
        if self.task_upstream == UpstreamScope::None {
            trace!("Not tracking task dependencies as upstream scope is none");

            return Ok(());
        }

        let deep = self.task_upstream == UpstreamScope::Deep;
        let mut visited = FxHashSet::default();
        let mut queue = VecDeque::new();

        for task in tasks {
            trace!(
                task_target = task.target.as_str(),
                "Tracking {} task dependencies",
                if deep { "deep" } else { "direct" }
            );

            self.visit_task_dependencies(task, deep, &mut visited, &mut queue);
        }

        while let Some(target) = queue.pop_front() {
            if visited.contains(&target) {
                continue;
            }

            let task = self.workspace_graph.get_task(&target)?;

            self.visit_task_dependencies(&task, deep, &mut visited, &mut queue);
        }

        Ok(())
    }

    fn visit_task_dependencies(
        &mut self,
        task: &Task,
        deep: bool,
        visited: &mut FxHashSet<Target>,
        queue: &mut VecDeque<Target>,
    ) {
        if !visited.insert(task.target.clone()) {
            return;
        }

        for dep_config in &task.deps {
            self.tasks
                .entry(dep_config.target.clone())
                .or_default()
                .insert(AffectedBy::DownstreamTask(task.target.clone()));

            if deep && !visited.contains(&dep_config.target) {
                queue.push_back(dep_config.target.clone());
            }
        }
    }

    fn track_task_dependents(&mut self, tasks: &[&Task]) -> miette::Result<()> {
        if self.task_downstream == DownstreamScope::None {
            trace!("Not tracking task dependents as downstream scope is none");

            return Ok(());
        }

        let deep = self.task_downstream == DownstreamScope::Deep;
        let mut visited = FxHashSet::default();
        let mut queue = VecDeque::new();

        for task in tasks {
            trace!(
                task_target = task.target.as_str(),
                "Tracking {} task dependents",
                if deep { "deep" } else { "direct" }
            );

            self.visit_task_dependents(task, deep, &mut visited, &mut queue);
        }

        while let Some(target) = queue.pop_front() {
            if visited.contains(&target) {
                continue;
            }

            let task = self.workspace_graph.get_task(&target)?;

            self.visit_task_dependents(&task, deep, &mut visited, &mut queue);
        }

        Ok(())
    }

    fn visit_task_dependents(
        &mut self,
        task: &Task,
        deep: bool,
        visited: &mut FxHashSet<Target>,
        queue: &mut VecDeque<Target>,
    ) {
        if !visited.insert(task.target.clone()) {
            return;
        }

        for dep_target in self.workspace_graph.tasks.dependents_of(task) {
            self.tasks
                .entry(dep_target.clone())
                .or_default()
                .insert(AffectedBy::UpstreamTask(task.target.clone()));

            if deep && !visited.contains(&dep_target) {
                queue.push_back(dep_target);
            }
        }
    }
}

impl fmt::Debug for AffectedTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AffectedTracker")
            .field("changed_files", &self.changed_files)
            .field("projects", &self.projects)
            .field("project_downstream", &self.project_downstream)
            .field("project_upstream", &self.project_upstream)
            .field("unaffected_projects", &self.unaffected_projects)
            .field("tasks", &self.tasks)
            .field("task_downstream", &self.task_downstream)
            .field("task_upstream", &self.task_upstream)
            .field("unaffected_tasks", &self.unaffected_tasks)
            .finish()
    }
}

/// Check whether the project itself is affected by the changed files,
/// ignoring any relationships.
fn is_project_directly_affected(
    project: &Project,
    changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
) -> Option<AffectedBy> {
    let file = if project.is_root_level() {
        // If at the root, any file affects it
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

/// Check whether the task itself is affected by CI, environment variables,
/// or the changed files, ignoring any relationships.
fn is_task_directly_affected(
    task: &Task,
    changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
    workspace_root: &Path,
    ci: bool,
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
            if let Some(var) = bag.get(var_name)
                && !var.is_empty()
            {
                return Ok(Some(AffectedBy::EnvironmentVariable(var_name.to_owned())));
            }
        }
    }

    // By files
    if task.input_files.is_empty() && task.input_globs.is_empty() {
        return Ok(None);
    }

    // Only compile the glob set when there are input globs, as nothing
    // can match without them, and compiling is expensive
    let globset = if task.input_globs.is_empty() {
        None
    } else {
        Some(task.create_globset()?)
    };

    for file in changed_files {
        let affected = if let Some(params) = task.input_files.get(file) {
            match &params.content {
                Some(matcher) => {
                    let abs_file = file.to_logical_path(workspace_root);

                    if abs_file.exists() {
                        matcher.is_match(&fs::read_file(abs_file)?)
                    } else {
                        false
                    }
                }
                None => true,
            }
        } else {
            globset
                .as_ref()
                .is_some_and(|globset| globset.matches(file.as_str()))
        };

        if affected {
            return Ok(Some(AffectedBy::ChangedFile(file.to_owned())));
        }
    }

    Ok(None)
}
