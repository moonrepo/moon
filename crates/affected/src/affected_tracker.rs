use crate::affected::*;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct AffectedTracker<'app> {
    project_graph: &'app ProjectGraph,
    touched_files: &'app FxHashSet<WorkspaceRelativePathBuf>,

    projects: FxHashMap<Id, Vec<AffectedBy>>,
    project_downstream: DownstreamScope,
    project_upstream: UpstreamScope,
}

impl<'app> AffectedTracker<'app> {
    pub fn with_project_scopes(
        &mut self,
        upstream_scope: UpstreamScope,
        downstream_scope: DownstreamScope,
    ) -> &mut Self {
        self.project_upstream = upstream_scope;
        self.project_downstream = downstream_scope;
        self
    }

    pub fn track(mut self) -> miette::Result<Affected> {
        self.track_projects()?;

        let mut affected = Affected::default();

        for (id, list) in self.projects {
            affected
                .projects
                .insert(id, AffectedProjectState::from(list));
        }

        Ok(affected)
    }

    fn track_projects(&mut self) -> miette::Result<()> {
        for project in self.project_graph.get_all()? {
            let affected = if project.is_root_level() {
                // If at the root, any file affects it
                if let Some(first_file) = self.touched_files.iter().next() {
                    AffectedBy::TouchedFile(first_file.to_owned())
                } else {
                    continue;
                }
            } else {
                let Some(file) = self
                    .touched_files
                    .iter()
                    .find(|file| file.starts_with(&project.source))
                else {
                    continue;
                };

                AffectedBy::TouchedFile(file.to_owned())
            };

            self.projects
                .entry(project.id.clone())
                .or_default()
                .push(affected);

            // If affected, handle up/down streams
            self.track_project_dependencies(&project, 0)?;
            self.track_project_dependents(&project, 0)?;
        }

        Ok(())
    }

    fn track_project_dependencies(&mut self, project: &Project, depth: u16) -> miette::Result<()> {
        for dep_id in self.project_graph.dependencies_of(project)? {
            self.projects
                .entry(dep_id.to_owned())
                .or_default()
                .push(AffectedBy::DownstreamProject(project.id.clone()));

            if depth == 0 {
                if self.project_upstream == UpstreamScope::Direct {
                    continue;
                } else {
                    let dep_project = self.project_graph.get(dep_id)?;

                    self.track_project_dependencies(&dep_project, depth + 1)?;
                }
            }
        }

        Ok(())
    }

    fn track_project_dependents(&mut self, project: &Project, depth: u16) -> miette::Result<()> {
        if self.project_downstream == DownstreamScope::None {
            return Ok(());
        }

        for dep_id in self.project_graph.dependents_of(project)? {
            self.projects
                .entry(dep_id.to_owned())
                .or_default()
                .push(AffectedBy::UpstreamProject(project.id.clone()));

            if depth == 0 {
                if self.project_downstream == DownstreamScope::Direct {
                    continue;
                } else {
                    let dep_project = self.project_graph.get(dep_id)?;

                    self.track_project_dependents(&dep_project, depth + 1)?;
                }
            }
        }

        Ok(())
    }
}
