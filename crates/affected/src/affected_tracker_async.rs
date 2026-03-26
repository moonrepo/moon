use crate::affected::*;
use miette::IntoDiagnostic;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_env_var::GlobalEnvBag;
use moon_project::Project;
use moon_task::{Target, Task, TaskOptionRunInCI};
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::fs;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, trace};

/// The result of a single mark-project operation, computed in a spawned task.
/// Contains all the `(Id, AffectedBy)` entries to insert into the projects map.
struct ProjectMarkEntries {
    entries: Vec<(Id, AffectedBy)>,
}

/// The result of a single mark-task operation, computed in a spawned task.
/// Contains entries for both the tasks map and the projects map (cross-reference).
struct TaskMarkEntries {
    task_entries: Vec<(Target, AffectedBy)>,
    project_entries: Vec<(Id, AffectedBy)>,
}

pub struct AffectedTrackerAsync {
    ci: bool,

    workspace_graph: Arc<WorkspaceGraph>,
    changed_files: FxHashSet<WorkspaceRelativePathBuf>,

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
        debug!("Creating affected tracker (async)");

        Self {
            workspace_graph,
            changed_files,
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

    // -- Helpers to apply collected entries --

    fn apply_project_entries(&mut self, entries: Vec<(Id, AffectedBy)>) {
        for (id, affected_by) in entries {
            self.projects.entry(id).or_default().insert(affected_by);
        }
    }

    fn apply_task_entries(&mut self, entries: Vec<(Target, AffectedBy)>) {
        for (target, affected_by) in entries {
            self.tasks.entry(target).or_default().insert(affected_by);
        }
    }

    // =========================================================================
    // Parallel project tracking
    // =========================================================================

    pub async fn track_projects(&mut self) -> miette::Result<&mut Self> {
        debug!("Tracking projects and marking any affected (async)");

        let projects = self.workspace_graph.get_projects()?;
        let changed_files = Arc::new(self.changed_files.clone());

        // Phase 1: Parallel check — spawn one task per project
        let mut check_set = JoinSet::new();

        for project in &projects {
            let project = Arc::clone(project);
            let changed_files = Arc::clone(&changed_files);
            let already_marked = self.is_project_marked_ignoring_relations(&project);

            check_set.spawn(async move {
                let affected =
                    Self::check_project_affected(&project, &changed_files, already_marked);
                (project, affected)
            });
        }

        // Collect check results
        let mut affected_projects = Vec::new();
        while let Some(result) = check_set.join_next().await {
            let (project, affected) = result.into_diagnostic()?;
            if let Some(affected) = affected {
                affected_projects.push((project, affected));
            }
        }

        // Phase 2: Parallel mark — spawn graph traversal per affected project
        let mut mark_set = JoinSet::new();

        for (project, affected) in affected_projects {
            let workspace_graph = Arc::clone(&self.workspace_graph);
            let upstream = self.project_upstream;
            let downstream = self.project_downstream;

            mark_set.spawn(async move {
                let result = Self::compute_project_mark_entries(
                    &workspace_graph,
                    &project,
                    affected,
                    upstream,
                    downstream,
                )?;
                Ok::<_, miette::Report>(result)
            });
        }

        // Phase 3: Apply all collected entries sequentially (fast hashmap inserts)
        while let Some(result) = mark_set.join_next().await {
            let mark_result = result.into_diagnostic()??;
            self.apply_project_entries(mark_result.entries);
        }

        Ok(self)
    }

    /// Compute all entries that `mark_project_affected` would insert, without
    /// mutating any shared state. Safe to run in a spawned task.
    fn compute_project_mark_entries(
        workspace_graph: &WorkspaceGraph,
        project: &Project,
        affected: AffectedBy,
        upstream: UpstreamScope,
        downstream: DownstreamScope,
    ) -> miette::Result<ProjectMarkEntries> {
        let mut entries = Vec::new();

        if affected != AffectedBy::AlreadyMarked {
            trace!(
                project_id = project.id.as_str(),
                "Marking project as affected"
            );

            entries.push((project.id.clone(), affected));
        }

        // Collect dependency entries (upstream)
        Self::collect_project_dependency_entries(
            workspace_graph,
            project,
            upstream,
            0,
            &mut FxHashSet::default(),
            &mut entries,
        )?;

        // Collect dependent entries (downstream)
        Self::collect_project_dependent_entries(
            workspace_graph,
            project,
            downstream,
            0,
            &mut FxHashSet::default(),
            &mut entries,
        )?;

        Ok(ProjectMarkEntries { entries })
    }

    /// Pure collection function: recursively walks project dependencies and
    /// accumulates `(Id, AffectedBy)` entries. No `&self` needed.
    fn collect_project_dependency_entries(
        workspace_graph: &WorkspaceGraph,
        project: &Project,
        upstream: UpstreamScope,
        depth: u16,
        cycle: &mut FxHashSet<Id>,
        entries: &mut Vec<(Id, AffectedBy)>,
    ) -> miette::Result<()> {
        if cycle.contains(&project.id) {
            return Ok(());
        }

        cycle.insert(project.id.clone());

        if upstream == UpstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking project dependencies as upstream scope is none"
            );
            return Ok(());
        }

        if depth == 0 {
            if upstream == UpstreamScope::Direct {
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
            entries.push((
                dep_config.id.clone(),
                AffectedBy::DownstreamProject(project.id.clone()),
            ));

            if depth == 0 && upstream == UpstreamScope::Direct {
                continue;
            }

            let dep_project = workspace_graph.get_project(&dep_config.id)?;

            Self::collect_project_dependency_entries(
                workspace_graph,
                &dep_project,
                upstream,
                depth + 1,
                cycle,
                entries,
            )?;
        }

        Ok(())
    }

    /// Pure collection function: recursively walks project dependents and
    /// accumulates `(Id, AffectedBy)` entries. No `&self` needed.
    fn collect_project_dependent_entries(
        workspace_graph: &WorkspaceGraph,
        project: &Project,
        downstream: DownstreamScope,
        depth: u16,
        cycle: &mut FxHashSet<Id>,
        entries: &mut Vec<(Id, AffectedBy)>,
    ) -> miette::Result<()> {
        if cycle.contains(&project.id) {
            return Ok(());
        }

        cycle.insert(project.id.clone());

        if downstream == DownstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking project dependents as downstream scope is none"
            );
            return Ok(());
        }

        if depth == 0 {
            if downstream == DownstreamScope::Direct {
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

        for dep_id in workspace_graph.projects.dependents_of(project) {
            entries.push((
                dep_id.clone(),
                AffectedBy::UpstreamProject(project.id.clone()),
            ));

            if depth == 0 && downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_project = workspace_graph.get_project(&dep_id)?;

            Self::collect_project_dependent_entries(
                workspace_graph,
                &dep_project,
                downstream,
                depth + 1,
                cycle,
                entries,
            )?;
        }

        Ok(())
    }

    fn check_project_affected(
        project: &Project,
        changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
        already_marked: bool,
    ) -> Option<AffectedBy> {
        if already_marked {
            return Some(AffectedBy::AlreadyMarked);
        }

        if project.is_root_level() {
            // If at the root, any file affects it
            changed_files
                .iter()
                .find(|file| !file.as_str().starts_with('.'))
                .map(|file| AffectedBy::ChangedFile(file.to_owned()))
        } else {
            changed_files
                .iter()
                .find(|file| file.starts_with(&project.source))
                .map(|file| AffectedBy::ChangedFile(file.to_owned()))
        }
    }

    pub fn is_project_affected(&self, project: &Project) -> Option<AffectedBy> {
        Self::check_project_affected(
            project,
            &self.changed_files,
            self.is_project_marked_ignoring_relations(project),
        )
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
        let result = Self::compute_project_mark_entries(
            &self.workspace_graph,
            project,
            affected,
            self.project_upstream,
            self.project_downstream,
        )?;
        self.apply_project_entries(result.entries);
        Ok(())
    }

    // =========================================================================
    // Parallel task tracking
    // =========================================================================

    pub async fn track_tasks(&mut self) -> miette::Result<()> {
        debug!("Tracking tasks and marking any affected (async)");

        // Include internal since they can trigger affected
        // for any dependents!
        let tasks = self.workspace_graph.get_tasks_with_internal()?;
        let changed_files = Arc::new(self.changed_files.clone());
        let workspace_root = self.workspace_graph.root.clone();
        let ci = self.ci;

        // Phase 1: Parallel check — spawn one task per task
        let mut check_set = JoinSet::new();

        for task in &tasks {
            let task = Arc::clone(task);
            let changed_files = Arc::clone(&changed_files);
            let workspace_root = workspace_root.clone();
            let already_marked = self.is_task_marked_ignoring_relations(&task);

            check_set.spawn(async move {
                let affected = Self::check_task_affected(
                    &task,
                    &changed_files,
                    &workspace_root,
                    ci,
                    already_marked,
                )?;
                Ok::<_, miette::Report>((task, affected))
            });
        }

        // Collect check results
        let mut affected_tasks = Vec::new();
        while let Some(result) = check_set.join_next().await {
            let (task, affected) = result.into_diagnostic()??;
            if let Some(affected) = affected {
                affected_tasks.push((task, affected));
            }
        }

        // Phase 2: Parallel mark — spawn graph traversal per affected task
        let mut mark_set = JoinSet::new();

        for (task, affected) in affected_tasks {
            let workspace_graph = Arc::clone(&self.workspace_graph);
            let upstream = self.task_upstream;
            let downstream = self.task_downstream;

            mark_set.spawn(async move {
                let result = Self::compute_task_mark_entries(
                    &workspace_graph,
                    &task,
                    affected,
                    upstream,
                    downstream,
                )?;
                Ok::<_, miette::Report>(result)
            });
        }

        // Phase 3: Apply all collected entries sequentially (fast hashmap inserts)
        while let Some(result) = mark_set.join_next().await {
            let mark_result = result.into_diagnostic()??;
            self.apply_task_entries(mark_result.task_entries);
            self.apply_project_entries(mark_result.project_entries);
        }

        Ok(())
    }

    pub async fn track_tasks_by_target(&mut self, targets: &[Target]) -> miette::Result<()> {
        debug!(
            task_targets = ?targets.iter().map(|target| target.as_str()).collect::<Vec<_>>(),
            "Tracking tasks by target and marking any affected (async)",
        );

        let changed_files = Arc::new(self.changed_files.clone());
        let workspace_root = self.workspace_graph.root.clone();
        let ci = self.ci;

        let mut tasks_to_check = Vec::new();
        for target in targets {
            tasks_to_check.push(self.workspace_graph.get_task(target)?);
        }

        // Phase 1: Parallel check
        let mut check_set = JoinSet::new();

        for task in &tasks_to_check {
            let task = Arc::clone(task);
            let changed_files = Arc::clone(&changed_files);
            let workspace_root = workspace_root.clone();
            let already_marked = self.is_task_marked_ignoring_relations(&task);

            check_set.spawn(async move {
                let affected = Self::check_task_affected(
                    &task,
                    &changed_files,
                    &workspace_root,
                    ci,
                    already_marked,
                )?;
                Ok::<_, miette::Report>((task, affected))
            });
        }

        // Collect check results
        let mut affected_tasks = Vec::new();
        while let Some(result) = check_set.join_next().await {
            let (task, affected) = result.into_diagnostic()??;
            if let Some(affected) = affected {
                affected_tasks.push((task, affected));
            }
        }

        // Phase 2: Parallel mark
        let mut mark_set = JoinSet::new();

        for (task, affected) in affected_tasks {
            let workspace_graph = Arc::clone(&self.workspace_graph);
            let upstream = self.task_upstream;
            let downstream = self.task_downstream;

            mark_set.spawn(async move {
                let result = Self::compute_task_mark_entries(
                    &workspace_graph,
                    &task,
                    affected,
                    upstream,
                    downstream,
                )?;
                Ok::<_, miette::Report>(result)
            });
        }

        // Phase 3: Apply all collected entries
        while let Some(result) = mark_set.join_next().await {
            let mark_result = result.into_diagnostic()??;
            self.apply_task_entries(mark_result.task_entries);
            self.apply_project_entries(mark_result.project_entries);
        }

        Ok(())
    }

    /// Compute all entries that `mark_task_affected` would insert, without
    /// mutating any shared state. Safe to run in a spawned task.
    fn compute_task_mark_entries(
        workspace_graph: &WorkspaceGraph,
        task: &Task,
        affected: AffectedBy,
        upstream: UpstreamScope,
        downstream: DownstreamScope,
    ) -> miette::Result<TaskMarkEntries> {
        let mut task_entries = Vec::new();
        let mut project_entries = Vec::new();

        if affected != AffectedBy::AlreadyMarked {
            trace!(
                task_target = task.target.as_str(),
                "Marking task as affected"
            );

            task_entries.push((task.target.clone(), affected));

            // Cross-reference: mark the owning project as affected by this task
            if let Ok(project_id) = task.target.get_project_id() {
                project_entries.push((
                    project_id.to_owned(),
                    AffectedBy::Task(task.target.clone()),
                ));
            }
        }

        // Collect dependency entries (upstream)
        Self::collect_task_dependency_entries(
            workspace_graph,
            task,
            upstream,
            0,
            &mut FxHashSet::default(),
            &mut task_entries,
        )?;

        // Collect dependent entries (downstream)
        Self::collect_task_dependent_entries(
            workspace_graph,
            task,
            downstream,
            0,
            &mut FxHashSet::default(),
            &mut task_entries,
        )?;

        Ok(TaskMarkEntries {
            task_entries,
            project_entries,
        })
    }

    /// Pure collection function: recursively walks task dependencies and
    /// accumulates `(Target, AffectedBy)` entries. No `&self` needed.
    fn collect_task_dependency_entries(
        workspace_graph: &WorkspaceGraph,
        task: &Task,
        upstream: UpstreamScope,
        depth: u16,
        cycle: &mut FxHashSet<Target>,
        entries: &mut Vec<(Target, AffectedBy)>,
    ) -> miette::Result<()> {
        if cycle.contains(&task.target) {
            return Ok(());
        }

        cycle.insert(task.target.clone());

        if upstream == UpstreamScope::None {
            trace!(
                task_target = task.target.as_str(),
                "Not tracking task dependencies as upstream scope is none"
            );
            return Ok(());
        }

        if depth == 0 {
            if upstream == UpstreamScope::Direct {
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
            entries.push((
                dep_config.target.clone(),
                AffectedBy::DownstreamTask(task.target.clone()),
            ));

            if depth == 0 && upstream == UpstreamScope::Direct {
                continue;
            }

            let dep_task = workspace_graph.get_task(&dep_config.target)?;

            Self::collect_task_dependency_entries(
                workspace_graph,
                &dep_task,
                upstream,
                depth + 1,
                cycle,
                entries,
            )?;
        }

        Ok(())
    }

    /// Pure collection function: recursively walks task dependents and
    /// accumulates `(Target, AffectedBy)` entries. No `&self` needed.
    fn collect_task_dependent_entries(
        workspace_graph: &WorkspaceGraph,
        task: &Task,
        downstream: DownstreamScope,
        depth: u16,
        cycle: &mut FxHashSet<Target>,
        entries: &mut Vec<(Target, AffectedBy)>,
    ) -> miette::Result<()> {
        if cycle.contains(&task.target) {
            return Ok(());
        }

        cycle.insert(task.target.clone());

        if downstream == DownstreamScope::None {
            trace!(
                task_target = task.target.as_str(),
                "Not tracking task dependents as downstream scope is none"
            );
            return Ok(());
        }

        if depth == 0 {
            if downstream == DownstreamScope::Direct {
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

        for dep_target in workspace_graph.tasks.dependents_of(task) {
            entries.push((
                dep_target.clone(),
                AffectedBy::UpstreamTask(task.target.clone()),
            ));

            if depth == 0 && downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_task = workspace_graph.get_task(&dep_target)?;

            Self::collect_task_dependent_entries(
                workspace_graph,
                &dep_task,
                downstream,
                depth + 1,
                cycle,
                entries,
            )?;
        }

        Ok(())
    }

    fn check_task_affected(
        task: &Task,
        changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
        workspace_root: &Path,
        ci: bool,
        already_marked: bool,
    ) -> miette::Result<Option<AffectedBy>> {
        if already_marked {
            return Ok(Some(AffectedBy::AlreadyMarked));
        }

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
        let globset = task.create_globset()?;

        for file in changed_files.iter() {
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
                globset.matches(file.as_str())
            };

            if affected {
                return Ok(Some(AffectedBy::ChangedFile(file.to_owned())));
            }
        }

        Ok(None)
    }

    pub fn is_task_affected(&self, task: &Task) -> miette::Result<Option<AffectedBy>> {
        Self::check_task_affected(
            task,
            &self.changed_files,
            &self.workspace_graph.root,
            self.ci,
            self.is_task_marked_ignoring_relations(task),
        )
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
        let result = Self::compute_task_mark_entries(
            &self.workspace_graph,
            task,
            affected,
            self.task_upstream,
            self.task_downstream,
        )?;
        self.apply_task_entries(result.task_entries);
        self.apply_project_entries(result.project_entries);
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
