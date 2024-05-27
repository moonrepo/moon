use crate::target_hash::BunTargetHash;
use moon_bun_tool::BunTool;
use moon_config::{HasherConfig, HasherOptimization};
use moon_node_lang::PackageJsonCache;
use moon_project::Project;
use moon_tool::DependencyManager;
use rustc_hash::FxHashMap;
use std::path::Path;

pub async fn create_target_hasher(
    bun: Option<&BunTool>,
    project: &Project,
    workspace_root: &Path,
    hasher_config: &HasherConfig,
) -> miette::Result<BunTargetHash> {
    let mut hasher = BunTargetHash::new(
        bun.map(|n| n.config.version.as_ref().map(|v| v.to_string()))
            .unwrap_or_default(),
    );

    let resolved_dependencies =
        if matches!(hasher_config.optimization, HasherOptimization::Accuracy) && bun.is_some() {
            bun.unwrap()
                .get_resolved_dependencies(&project.root)
                .await?
        } else {
            FxHashMap::default()
        };

    if let Some(root_package) = PackageJsonCache::read(
        workspace_root.join(bun.map(|n| n.config.packages_root.as_str()).unwrap_or(".")),
    )? {
        hasher.hash_package_json(&root_package.data, &resolved_dependencies);
    }

    if let Some(package) = PackageJsonCache::read(&project.root)? {
        hasher.hash_package_json(&package.data, &resolved_dependencies);
    }

    Ok(hasher)
}
