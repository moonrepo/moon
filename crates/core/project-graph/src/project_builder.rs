#![allow(unused_imports, unused_variables)]

use crate::errors::ProjectGraphError;
use crate::graph_hasher::GraphHasher;
use crate::helpers::detect_projects_with_globs;
use crate::project_graph::{GraphType, IndicesType, ProjectGraph, LOG_TARGET};
use crate::token_resolver::{TokenContext, TokenResolver};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{consts, is_test_env, Id};
use moon_config::{InputPath, ProjectsAliasesMap, ProjectsSourcesMap, WorkspaceProjects};
use moon_hasher::{convert_paths_to_strings, to_hash};
use moon_logger::{debug, map_list, trace, warn};
use moon_platform::PlatformManager;
use moon_platform_detector::{detect_project_language, detect_task_platform};
use moon_project::Project;
use moon_project_builder::{ProjectBuilder, ProjectBuilderError};
use moon_project_constraints::{enforce_project_type_relationships, enforce_tag_relationships};
use moon_target::{Target, TargetScope};
use moon_task::Task;
use moon_utils::regex::ENV_VAR_SUBSTITUTE;
use moon_utils::{path, time};
use moon_workspace::Workspace;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_styles::color;
use starbase_utils::glob::{self, GlobSet};
use std::collections::BTreeMap;
use std::env;
use std::mem;

pub struct ProjectGraphBuilder<'ws> {
    workspace: &'ws mut Workspace,

    aliases: ProjectsAliasesMap,
    graph: GraphType,
    indices: IndicesType,
    sources: ProjectsSourcesMap,

    // Project and its dependencies being created.
    // We use this to prevent circular dependencies.
    created: FxHashSet<Id>,

    pub is_cached: bool,
    pub hash: String,
}

impl<'ws> ProjectGraphBuilder<'ws> {
    fn validate_outputs(&self) -> miette::Result<()> {
        // TODO: Remove when we refactor the graph
        if is_test_env() || env::var("MOON_DISABLE_OVERLAPPING_OUTPUTS").is_ok() {
            return Ok(());
        }

        let mut file_outputs = FxHashMap::<&WorkspaceRelativePathBuf, &Target>::default();
        let mut glob_outputs = FxHashMap::<&WorkspaceRelativePathBuf, &Target>::default();

        // Do paths first so that we can aggregate everything before globbing
        for project in self.graph.node_weights() {
            for task in project.tasks.values() {
                if task.output_paths.is_empty() {
                    continue;
                }

                for path in &task.output_paths {
                    if let Some(existing_target) = file_outputs.get(&path) {
                        return Err(ProjectGraphError::OverlappingTaskOutputs {
                            output: path.to_string(),
                            targets: vec![(*existing_target).to_owned(), task.target.to_owned()],
                        }
                        .into());
                    }

                    file_outputs.insert(path, &task.target);
                }
            }
        }

        // Do globs second so we can match against the aggregated paths
        for project in self.graph.node_weights() {
            for task in project.tasks.values() {
                if task.output_globs.is_empty() {
                    continue;
                }

                // Do explicit checks first as they're faster
                for glob in &task.output_globs {
                    if let Some(existing_target) = glob_outputs.get(&glob) {
                        if *existing_target != &task.target {
                            return Err(ProjectGraphError::OverlappingTaskOutputs {
                                output: glob.to_string(),
                                targets: vec![
                                    (*existing_target).to_owned(),
                                    task.target.to_owned(),
                                ],
                            }
                            .into());
                        }
                    }

                    glob_outputs.insert(glob, &task.target);
                }

                // Now attempt to match
                let globset = GlobSet::new(&task.output_globs)?;

                for (existing_file, existing_target) in &file_outputs {
                    if *existing_target != &task.target && globset.is_match(existing_file.as_str())
                    {
                        return Err(ProjectGraphError::OverlappingTaskOutputs {
                            output: existing_file.to_string(),
                            targets: vec![(*existing_target).to_owned(), task.target.to_owned()],
                        }
                        .into());
                    }
                }
            }
        }

        Ok(())
    }
}
