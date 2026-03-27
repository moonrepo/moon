use crate::affected::*;
use crate::project_tracker::ProjectTracker;
use crate::task_tracker::TaskTracker;
use miette::IntoDiagnostic;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_project::Project;
use moon_task::{Target, Task};
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::debug;

pub struct AffectedTrackerAsync {
    ci: bool,

    workspace_graph: Arc<WorkspaceGraph>,
    changed_files: Arc<FxHashSet<WorkspaceRelativePathBuf>>,

    projects: FxHashMap<Id, FxHashSet<AffectedBy>>,
    project_downstream: DownstreamScope,
    project_upstream: UpstreamScope,

    tasks: FxHashMap<Target, FxHashSet<AffectedBy>>,
    task_downstream: DownstreamScope,
    task_upstream: UpstreamScope,
}

impl AffectedTrackerAsync {
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

        let downstream = self.project_downstream;
        let upstream = self.project_upstream;

        // Spawn one task per project
        let mut set = JoinSet::new();

        for project in self.workspace_graph.get_projects()? {
            let changed_files = Arc::clone(&self.changed_files);
            let workspace_graph = Arc::clone(&self.workspace_graph);

            set.spawn(async move {
                ProjectTracker {
                    changed_files,
                    downstream,
                    project,
                    tracked: FxHashMap::default(),
                    upstream,
                    workspace_graph,
                }
                .track()
                .await
            });
        }

        // Collect tracker results
        while let Some(result) = set.join_next().await {
            let tracker = result.into_diagnostic()??;

            for (project_id, affected) in tracker.tracked {
                self.projects
                    .entry(project_id)
                    .or_default()
                    .extend(affected);
            }
        }

        Ok(self)
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

    pub async fn track_tasks(&mut self) -> miette::Result<()> {
        debug!("Tracking tasks and marking any affected");

        // Include internal since they can trigger affected for any dependents!
        self.internal_track_tasks(self.workspace_graph.get_tasks_with_internal()?)
            .await
    }

    pub async fn track_tasks_by_target(&mut self, targets: &[Target]) -> miette::Result<()> {
        debug!(
            task_targets = ?targets.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
            "Tracking tasks by target and marking any affected",
        );

        let mut tasks = Vec::with_capacity(targets.len());

        for target in targets {
            tasks.push(self.workspace_graph.get_task(target)?);
        }

        self.internal_track_tasks(tasks).await
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

    async fn internal_track_tasks(&mut self, tasks: Vec<Arc<Task>>) -> miette::Result<()> {
        debug!("Tracking tasks and marking any affected");

        let ci = self.ci;
        let downstream = self.task_downstream;
        let upstream = self.task_upstream;

        // Spawn one task per project
        let mut set = JoinSet::new();

        // Include internal since they can trigger affected for any dependents!
        for task in tasks {
            let changed_files = Arc::clone(&self.changed_files);
            let workspace_graph = Arc::clone(&self.workspace_graph);

            set.spawn(async move {
                TaskTracker {
                    changed_files,
                    ci,
                    downstream,
                    task,
                    tracked: FxHashMap::default(),
                    tracked_projects: FxHashMap::default(),
                    upstream,
                    workspace_graph,
                }
                .track()
                .await
            });
        }

        // Collect tracker results
        while let Some(result) = set.join_next().await {
            let tracker = result.into_diagnostic()??;

            for (task_target, affected) in tracker.tracked {
                self.tasks.entry(task_target).or_default().extend(affected);
            }

            for (project_id, affected) in tracker.tracked_projects {
                self.projects
                    .entry(project_id)
                    .or_default()
                    .extend(affected);
            }
        }

        Ok(())
    }
}

impl fmt::Debug for AffectedTrackerAsync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AffectedTrackerAsync")
            .field("changed_files", &self.changed_files)
            .field("projects", &self.projects)
            .field("project_downstream", &self.project_downstream)
            .field("project_upstream", &self.project_upstream)
            .field("tasks", &self.tasks)
            .field("task_downstream", &self.task_downstream)
            .field("task_upstream", &self.task_upstream)
            .finish()
    }
}
