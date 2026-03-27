use crate::affected::*;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_project::Project;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use tracing::trace;

pub struct ProjectTracker {
    pub changed_files: Arc<FxHashSet<WorkspaceRelativePathBuf>>,
    pub downstream: DownstreamScope,
    pub project: Arc<Project>,
    pub tracked: FxHashMap<Id, FxHashSet<AffectedBy>>,
    pub upstream: UpstreamScope,
    pub workspace_graph: Arc<WorkspaceGraph>,
}

impl ProjectTracker {
    pub async fn track(mut self) -> miette::Result<Self> {
        let project = Arc::clone(&self.project);

        if let Some(affected) = self.is_project_affected(&project) {
            self.mark_project_affected(&project, affected)?;
        }

        Ok(self)
    }

    fn is_project_affected(&self, project: &Project) -> Option<AffectedBy> {
        if project.is_root_level() {
            // If at the root, any file affects it
            self.changed_files
                .iter()
                .find(|file| !file.as_str().starts_with('.'))
                .map(|file| AffectedBy::ChangedFile(file.to_owned()))
        } else {
            self.changed_files
                .iter()
                .find(|file| file.starts_with(&project.source))
                .map(|file| AffectedBy::ChangedFile(file.to_owned()))
        }
    }

    fn mark_project_affected(
        &mut self,
        project: &Project,
        affected: AffectedBy,
    ) -> miette::Result<()> {
        if affected == AffectedBy::AlreadyMarked {
            // May have been already marked through an indirect dep,
            // but that doesn't mean its own deps have been checked!
            self.track_project_dependencies(project, 0, &mut FxHashSet::default())?;
            self.track_project_dependents(project, 0, &mut FxHashSet::default())?;

            return Ok(());
        }

        trace!(
            project_id = project.id.as_str(),
            "Marking project as affected"
        );

        self.tracked
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

        if self.upstream == UpstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking project dependencies as upstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.upstream == UpstreamScope::Direct {
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
            self.tracked
                .entry(dep_config.id.clone())
                .or_default()
                .insert(AffectedBy::DownstreamProject(project.id.clone()));

            if depth == 0 && self.upstream == UpstreamScope::Direct {
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

        if self.downstream == DownstreamScope::None {
            trace!(
                project_id = project.id.as_str(),
                "Not tracking project dependents as downstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.downstream == DownstreamScope::Direct {
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
            self.tracked
                .entry(dep_id.clone())
                .or_default()
                .insert(AffectedBy::UpstreamProject(project.id.clone()));

            if depth == 0 && self.downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_project = self.workspace_graph.get_project(&dep_id)?;

            self.track_project_dependents(&dep_project, depth + 1, cycle)?;
        }

        Ok(())
    }
}
