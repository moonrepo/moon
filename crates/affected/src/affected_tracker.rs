use crate::affected::*;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_env_var::GlobalEnvBag;
use moon_project::Project;
use moon_task::{Target, Task, TaskOptionRunInCI};
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct AffectedTracker {
    ci: bool,

    workspace_graph: Arc<WorkspaceGraph>,
    touched_files: FxHashSet<WorkspaceRelativePathBuf>,

    projects: FxHashMap<Id, FxHashSet<AffectedBy>>,
    project_downstream: DownstreamScope,
    project_upstream: UpstreamScope,

    tasks: FxHashMap<Target, FxHashSet<AffectedBy>>,
    task_downstream: DownstreamScope,
    task_upstream: UpstreamScope,
}

impl AffectedTracker {
    pub fn new(
        workspace_graph: Arc<WorkspaceGraph>,
        touched_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> Self {
        debug!("Creating affected tracker");

        Self {
            workspace_graph,
            touched_files,
            projects: FxHashMap::default(),
            project_downstream: DownstreamScope::None,
            project_upstream: UpstreamScope::Deep,
            tasks: FxHashMap::default(),
            task_downstream: DownstreamScope::None,
            task_upstream: UpstreamScope::Deep,
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
                upstream = ?state.upstream.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
                downstream = ?state.downstream.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
                other = state.other,
                "Task {} is affected by", color::label(&target),
            );

            affected.tasks.insert(target, state);
        }

        affected.should_check = !self.touched_files.is_empty();
        affected
    }

    pub fn set_ci_check(&mut self, ci: bool) -> &mut Self {
        self.ci = ci;
        self
    }

    pub fn with_project_scopes(
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

    pub fn with_task_scopes(
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

    pub fn with_scopes(
        &mut self,
        upstream_scope: UpstreamScope,
        downstream_scope: DownstreamScope,
    ) -> &mut Self {
        self.with_project_scopes(upstream_scope, downstream_scope);
        self.with_task_scopes(upstream_scope, downstream_scope);
        self
    }

    pub fn track_projects(&mut self) -> miette::Result<&mut Self> {
        debug!("Tracking projects and marking any affected");

        for project in self.workspace_graph.get_projects()? {
            if let Some(affected) = self.is_project_affected(&project) {
                self.mark_project_affected(&project, affected)?;
            }
        }

        Ok(self)
    }

    pub fn is_project_affected(&self, project: &Project) -> Option<AffectedBy> {
        if project.is_root_level() {
            // If at the root, any file affects it
            self.touched_files
                .iter()
                .next()
                .map(|file| AffectedBy::TouchedFile(file.to_owned()))
        } else {
            self.touched_files
                .iter()
                .find(|file| file.starts_with(&project.source))
                .map(|file| AffectedBy::TouchedFile(file.to_owned()))
        }
    }

    pub fn is_project_marked(&self, project: &Project) -> bool {
        self.projects.contains_key(&project.id)
    }

    pub fn mark_project_affected(
        &mut self,
        project: &Project,
        affected: AffectedBy,
    ) -> miette::Result<()> {
        if affected == AffectedBy::AlreadyMarked {
            return Ok(());
        }

        trace!(
            project_id = project.id.as_str(),
            "Marking project as affected"
        );

        self.projects
            .entry(project.id.clone())
            .or_default()
            .insert(affected);

        self.track_project_dependencies(project, 0, &mut FxHashSet::default())?;
        self.track_project_dependents(project, 0, &mut FxHashSet::default())?;

        Ok(())
    }

    fn track_project_dependencies(
        &mut self,
        project: &Project,
        depth: u16,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<()> {
        if cycle.contains(&project.id) {
            return Ok(());
        }

        cycle.insert(project.id.clone());

        if self.project_upstream == UpstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking project dependencies as upstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.project_upstream == UpstreamScope::Direct {
                trace!(
                    project_id = project.id.as_str(),
                    "Tracking direct project dependencies"
                );
            } else {
                trace!(
                    project_id = project.id.as_str(),
                    "Tracking deep project dependencies"
                );
            }
        }

        for dep_config in &project.dependencies {
            self.projects
                .entry(dep_config.id.clone())
                .or_default()
                .insert(AffectedBy::DownstreamProject(project.id.clone()));

            if depth == 0 && self.project_upstream == UpstreamScope::Direct {
                continue;
            }

            let dep_project = self.workspace_graph.get_project(&dep_config.id)?;

            self.track_project_dependencies(&dep_project, depth + 1, cycle)?;
        }

        Ok(())
    }

    fn track_project_dependents(
        &mut self,
        project: &Project,
        depth: u16,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<()> {
        if cycle.contains(&project.id) {
            return Ok(());
        }

        cycle.insert(project.id.clone());

        if self.project_downstream == DownstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking project dependents as downstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.project_downstream == DownstreamScope::Direct {
                trace!(
                    project_id = project.id.as_str(),
                    "Tracking direct project dependents"
                );
            } else {
                trace!(
                    project_id = project.id.as_str(),
                    "Tracking deep project dependents"
                );
            }
        }

        for dep_id in self.workspace_graph.projects.dependents_of(project) {
            self.projects
                .entry(dep_id.clone())
                .or_default()
                .insert(AffectedBy::UpstreamProject(project.id.clone()));

            if depth == 0 && self.project_downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_project = self.workspace_graph.get_project(&dep_id)?;

            self.track_project_dependents(&dep_project, depth + 1, cycle)?;
        }

        Ok(())
    }

    pub fn track_tasks(&mut self) -> miette::Result<()> {
        debug!("Tracking tasks and marking any affected");

        // Include internal since they can trigger affected
        // for any dependents!
        for task in self.workspace_graph.get_tasks_with_internal()? {
            if let Some(affected) = self.is_task_affected(&task)? {
                self.mark_task_affected(&task, affected)?;
            }
        }

        Ok(())
    }

    pub fn track_tasks_by_target(&mut self, targets: &[Target]) -> miette::Result<()> {
        debug!(
            task_targets = ?targets.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
            "Tracking tasks by target and marking any affected",
        );

        for target in targets {
            let task = self.workspace_graph.get_task(target)?;

            if let Some(affected) = self.is_task_affected(&task)? {
                self.mark_task_affected(&task, affected)?;
            }
        }

        Ok(())
    }

    pub fn is_task_affected(&self, task: &Task) -> miette::Result<Option<AffectedBy>> {
        if self.is_task_marked(task) {
            return Ok(Some(AffectedBy::AlreadyMarked));
        }

        if self.ci {
            match &task.options.run_in_ci {
                TaskOptionRunInCI::Always => {
                    return Ok(Some(AffectedBy::AlwaysAffected));
                }
                TaskOptionRunInCI::Enabled(false) => return Ok(None),
                _ => {}
            };
        }

        // inputs: []
        if task.state.empty_inputs {
            return Ok(None);
        }

        if !task.input_env.is_empty() {
            let bag = GlobalEnvBag::instance();

            for var_name in &task.input_env {
                if let Some(var) = bag.get(var_name) {
                    if !var.is_empty() {
                        return Ok(Some(AffectedBy::EnvironmentVariable(var_name.to_owned())));
                    }
                }
            }
        }

        let globset = task.create_globset()?;

        for file in self.touched_files.iter() {
            if task.input_files.contains(file) || globset.matches(file.as_str()) {
                return Ok(Some(AffectedBy::TouchedFile(file.to_owned())));
            }
        }

        Ok(None)
    }

    pub fn is_task_marked(&self, task: &Task) -> bool {
        self.tasks.contains_key(&task.target)
    }

    pub fn mark_task_affected(&mut self, task: &Task, affected: AffectedBy) -> miette::Result<()> {
        if affected == AffectedBy::AlreadyMarked {
            return Ok(());
        }

        trace!(
            task_target = task.target.as_str(),
            "Marking task as affected"
        );

        self.tasks
            .entry(task.target.clone())
            .or_default()
            .insert(affected);

        self.track_task_dependencies(task, 0, &mut FxHashSet::default())?;
        self.track_task_dependents(task, 0, &mut FxHashSet::default())?;

        if let Some(project_id) = task.target.get_project_id() {
            self.projects
                .entry(project_id.to_owned())
                .or_default()
                .insert(AffectedBy::Task(task.target.clone()));
        }

        Ok(())
    }

    fn track_task_dependencies(
        &mut self,
        task: &Task,
        depth: u16,
        cycle: &mut FxHashSet<Target>,
    ) -> miette::Result<()> {
        if cycle.contains(&task.target) {
            return Ok(());
        }

        cycle.insert(task.target.clone());

        if self.task_upstream == UpstreamScope::None {
            trace!(
                task_target = task.target.as_str(),
                "Not tracking task dependencies as upstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.task_upstream == UpstreamScope::Direct {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking direct task dependencies"
                );
            } else {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking deep task dependencies"
                );
            }
        }

        for dep_config in &task.deps {
            self.tasks
                .entry(dep_config.target.clone())
                .or_default()
                .insert(AffectedBy::DownstreamTask(task.target.clone()));

            if depth == 0 && self.task_upstream == UpstreamScope::Direct {
                continue;
            }

            let dep_task = self.workspace_graph.get_task(&dep_config.target)?;

            self.track_task_dependencies(&dep_task, depth + 1, cycle)?;
        }

        Ok(())
    }

    fn track_task_dependents(
        &mut self,
        task: &Task,
        depth: u16,
        cycle: &mut FxHashSet<Target>,
    ) -> miette::Result<()> {
        if cycle.contains(&task.target) {
            return Ok(());
        }

        cycle.insert(task.target.clone());

        if self.task_downstream == DownstreamScope::None {
            trace!(
                task_target = task.target.as_str(),
                "Not tracking task dependents as downstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.task_downstream == DownstreamScope::Direct {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking direct task dependents"
                );
            } else {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking deep task dependents"
                );
            }
        }

        for dep_target in self.workspace_graph.tasks.dependents_of(task) {
            self.tasks
                .entry(dep_target.clone())
                .or_default()
                .insert(AffectedBy::UpstreamTask(task.target.clone()));

            if depth == 0 && self.task_downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_task = self.workspace_graph.get_task(&dep_target)?;

            self.track_task_dependents(&dep_task, depth + 1, cycle)?;
        }

        Ok(())
    }
}

impl fmt::Debug for AffectedTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AffectedTracker")
            .field("touched_files", &self.touched_files)
            .field("projects", &self.projects)
            .field("project_downstream", &self.project_downstream)
            .field("project_upstream", &self.project_upstream)
            .field("tasks", &self.tasks)
            .field("task_downstream", &self.task_downstream)
            .field("task_upstream", &self.task_upstream)
            .finish()
    }
}
