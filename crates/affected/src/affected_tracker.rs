use crate::affected::*;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{color, Id};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::{Target, TargetScope, Task};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use tracing::{debug, trace};

pub struct AffectedTracker<'app> {
    project_graph: &'app ProjectGraph,
    touched_files: &'app FxHashSet<WorkspaceRelativePathBuf>,

    projects: FxHashMap<Id, Vec<AffectedBy>>,
    project_downstream: DownstreamScope,
    project_upstream: UpstreamScope,

    tasks: FxHashMap<Target, Vec<AffectedBy>>,
    task_downstream: DownstreamScope,
    task_upstream: UpstreamScope,
}

impl<'app> AffectedTracker<'app> {
    pub fn new(
        project_graph: &'app ProjectGraph,
        touched_files: &'app FxHashSet<WorkspaceRelativePathBuf>,
    ) -> Self {
        debug!("Creating affected tracker");

        Self {
            project_graph,
            touched_files,
            projects: FxHashMap::default(),
            project_downstream: DownstreamScope::default(),
            project_upstream: UpstreamScope::default(),
            tasks: FxHashMap::default(),
            task_downstream: DownstreamScope::default(),
            task_upstream: UpstreamScope::default(),
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

        for project in self.project_graph.get_all()? {
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
            .push(affected);

        self.track_project_dependencies(project, 0)?;
        self.track_project_dependents(project, 0)?;

        Ok(())
    }

    fn track_project_dependencies(&mut self, project: &Project, depth: u16) -> miette::Result<()> {
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

        for dep_id in self.project_graph.dependencies_of(project)? {
            self.projects
                .entry(dep_id.to_owned())
                .or_default()
                .push(AffectedBy::DownstreamProject(project.id.clone()));

            if depth == 0 && self.project_upstream == UpstreamScope::Direct {
                continue;
            }

            let dep_project = self.project_graph.get(dep_id)?;

            self.track_project_dependencies(&dep_project, depth + 1)?;
        }

        Ok(())
    }

    fn track_project_dependents(&mut self, project: &Project, depth: u16) -> miette::Result<()> {
        if self.project_downstream == DownstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking dependents as downstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.project_downstream == DownstreamScope::Direct {
                trace!(
                    project_id = project.id.as_str(),
                    "Tracking direct dependents"
                );
            } else {
                trace!(
                    project_id = project.id.as_str(),
                    "Tracking deep dependents (entire family)"
                );
            }
        }

        for dep_id in self.project_graph.dependents_of(project)? {
            self.projects
                .entry(dep_id.to_owned())
                .or_default()
                .push(AffectedBy::UpstreamProject(project.id.clone()));

            if depth == 0 && self.project_downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_project = self.project_graph.get(dep_id)?;

            self.track_project_dependents(&dep_project, depth + 1)?;
        }

        Ok(())
    }

    pub fn track_tasks(&mut self) -> miette::Result<()> {
        debug!("Tracking tasks and marking any affected");

        for project in self.project_graph.get_all()? {
            for task in project.get_tasks()? {
                if let Some(affected) = self.is_task_affected(task)? {
                    self.mark_task_affected(task, affected)?;
                }
            }
        }

        Ok(())
    }

    pub fn track_tasks_by_target(&mut self, targets: &[Target]) -> miette::Result<()> {
        debug!(
            targets = ?targets.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
            "Tracking tasks by target and marking any affected",
        );

        let mut lookup = FxHashMap::<&Id, Vec<&Id>>::default();

        for target in targets {
            if let TargetScope::Project(project_id) = &target.scope {
                lookup.entry(project_id).or_default().push(&target.task_id);
            }
        }

        for (project_id, task_ids) in lookup {
            let project = self.project_graph.get(project_id)?;

            for task_id in task_ids {
                let task = project.get_task(task_id)?;

                if let Some(affected) = self.is_task_affected(task)? {
                    self.mark_task_affected(task, affected)?;
                }
            }
        }

        Ok(())
    }

    pub fn is_task_affected(&self, task: &Task) -> miette::Result<Option<AffectedBy>> {
        if self.is_task_marked(task) {
            return Ok(Some(AffectedBy::AlreadyMarked));
        }

        if task.metadata.empty_inputs {
            return Ok(Some(AffectedBy::AlwaysAffected));
        }

        for var_name in &task.input_env {
            if let Ok(var) = env::var(var_name) {
                if !var.is_empty() {
                    return Ok(Some(AffectedBy::EnvironmentVariable(var_name.to_owned())));
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

        trace!(target = task.target.as_str(), "Marking task as affected");

        self.tasks
            .entry(task.target.clone())
            .or_default()
            .push(affected);

        self.track_task_dependencies(task, 0)?;

        Ok(())
    }

    fn track_task_dependencies(&mut self, task: &Task, depth: u16) -> miette::Result<()> {
        if self.task_upstream == UpstreamScope::None {
            trace!(
                target = task.target.as_str(),
                "Not tracking task dependencies as upstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.task_upstream == UpstreamScope::Direct {
                trace!(
                    target = task.target.as_str(),
                    "Tracking direct task dependencies"
                );
            } else {
                trace!(
                    target = task.target.as_str(),
                    "Tracking deep task dependencies"
                );
            }
        }

        for dep_config in &task.deps {
            self.tasks
                .entry(dep_config.target.clone())
                .or_default()
                .push(AffectedBy::DownstreamTask(task.target.clone()));

            if depth == 0 && self.task_upstream == UpstreamScope::Direct {
                continue;
            }

            if let TargetScope::Project(project_id) = &dep_config.target.scope {
                let dep_project = self.project_graph.get(project_id)?;
                let dep_task = dep_project.get_task(&dep_config.target.task_id)?;

                self.track_task_dependencies(dep_task, depth + 1)?;
            }
        }

        Ok(())
    }
}
