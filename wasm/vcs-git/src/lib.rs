//! Built-in Git source-control provider.

use extism_pdk::*;
use moon_pdk_api::*;
use warpgate_pdk::*;

#[plugin_fn]
pub fn register_vcs(Json(input): Json<RegisterVcsInput>) -> FnResult<Json<VcsPluginMetadata>> {
    if input.host_protocol_version != VCS_PLUGIN_PROTOCOL_VERSION {
        return Err(anyhow!("unsupported host protocol {}", input.host_protocol_version).into());
    }

    Ok(Json(VcsPluginMetadata {
        name: "Git".into(),
        description: Some("Moon's bundled Git source-control provider".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        protocol_version: VCS_PLUGIN_PROTOCOL_VERSION,
        supports_hooks: true,
    }))
}

#[plugin_fn]
pub fn detect_vcs(Json(input): Json<DetectVcsInput>) -> FnResult<Json<DetectVcsOutput>> {
    let output = run_git(&input.context, ["rev-parse", "--show-toplevel"])?;

    Ok(Json(DetectVcsOutput {
        active: output.exit_code == 0,
        reason: if output.exit_code == 0 {
            format!("Git repository at {}", output.stdout.trim())
        } else {
            "Git did not detect a repository".into()
        },
    }))
}

#[plugin_fn]
pub fn observe_vcs(Json(input): Json<ObserveVcsInput>) -> FnResult<Json<VcsObservation>> {
    if let Some(baseline) = &input.baseline {
        validate_ref(baseline)?;
    }

    let mut args = vec![
        "rev-parse".to_owned(),
        "--show-toplevel".to_owned(),
        "HEAD^{commit}".to_owned(),
    ];

    if let Some(baseline) = &input.baseline {
        args.push(format!("{baseline}^{{commit}}"));
    }

    args.extend([
        "--is-shallow-repository".to_owned(),
        "--show-prefix".to_owned(),
    ]);

    let state = run_git(&input.context, args)?;

    if state.exit_code != 0
        && run_git(&input.context, ["rev-parse", "--show-toplevel"])?.exit_code != 0
    {
        return Ok(Json(VcsObservation {
            id: "unavailable".into(),
            provider: "git".into(),
            client_version: git_version(&input.context),
            enabled: false,
            repository_root: input.context.workspace_root.to_string(),
            working_root: input.context.workspace_root.to_string(),
            current: VcsStateIdentity {
                id: String::new(),
                label: None,
            },
            baseline: input.baseline.map(|label| VcsStateIdentity {
                id: String::new(),
                label: Some(label),
            }),
            repository_slug: None,
            history: VcsHistoryCompleteness::Unknown,
        }));
    }

    let (repository_root, current_id, baseline, shallow, workspace_prefix) = if state.exit_code == 0
    {
        let mut values = state.stdout.lines();
        let repository_root = values
            .next()
            .ok_or_else(|| anyhow!("Git did not return a repository root"))?
            .to_owned();
        let current_id = values
            .next()
            .ok_or_else(|| anyhow!("Git did not return the current state"))?
            .to_owned();
        let baseline = input
            .baseline
            .as_ref()
            .map(|label| {
                values
                    .next()
                    .map(|id| VcsStateIdentity {
                        id: id.to_owned(),
                        label: Some(label.to_owned()),
                    })
                    .ok_or_else(|| anyhow!("Git did not return the baseline state"))
            })
            .transpose()?;
        let shallow = values
            .next()
            .ok_or_else(|| anyhow!("Git did not return the history state"))?
            .to_owned();
        let workspace_prefix = values.next().unwrap_or_default().to_owned();

        (
            repository_root,
            current_id,
            baseline,
            shallow,
            workspace_prefix,
        )
    } else {
        let fallback = run_git(
            &input.context,
            [
                "rev-parse",
                "--show-toplevel",
                "--is-shallow-repository",
                "--show-prefix",
            ],
        )?;
        require_success(&fallback)?;

        let mut values = fallback.stdout.lines();
        let repository_root = values
            .next()
            .ok_or_else(|| anyhow!("Git did not return a repository root"))?
            .to_owned();
        let shallow = values
            .next()
            .ok_or_else(|| anyhow!("Git did not return the history state"))?
            .to_owned();
        let workspace_prefix = values.next().unwrap_or_default().to_owned();
        let current_id = successful_stdout(run_git(
            &input.context,
            ["rev-parse", "--verify", "HEAD^{commit}"],
        )?)
        .unwrap_or_default();
        let baseline = input.baseline.and_then(|label| {
            resolve_ref(&input.context, &label, &input.remote_candidates)
                .ok()
                .map(|id| VcsStateIdentity {
                    id,
                    label: Some(label),
                })
        });

        (
            repository_root,
            current_id,
            baseline,
            shallow,
            workspace_prefix,
        )
    };
    let label = successful_stdout(run_git(&input.context, ["branch", "--show-current"])?)
        .filter(|value| !value.is_empty());
    let sequence = var::get::<u64>("observation_sequence")?.unwrap_or_default() + 1;
    let workspace_key = format!("workspace_effects_{sequence}");
    let pinned_workspace = workspace_effects(&input.context, &workspace_prefix)?;
    var::set("observation_sequence", sequence)?;
    var::set(&workspace_key, json::to_string(&pinned_workspace)?)?;

    Ok(Json(VcsObservation {
        id: format!(
            "{current_id}\0{workspace_prefix}\0{workspace_key}\0{}",
            json::to_string(&input.remote_candidates)?
        ),
        provider: "git".into(),
        client_version: git_version(&input.context),
        enabled: true,
        repository_root: repository_root.clone(),
        working_root: repository_root,
        current: VcsStateIdentity {
            id: current_id.clone(),
            label: label.or_else(|| Some(short_id(&current_id))),
        },
        baseline,
        repository_slug: repository_slug(&input.context, &input.remote_candidates),
        history: match shallow.as_str() {
            "true" => VcsHistoryCompleteness::Incomplete,
            "false" => VcsHistoryCompleteness::Complete,
            _ => VcsHistoryCompleteness::Unknown,
        },
    }))
}

#[plugin_fn]
pub fn get_vcs_impacts(
    Json(input): Json<GetVcsImpactsInput>,
) -> FnResult<Json<GetVcsImpactsOutput>> {
    let mut diagnostics = vec![];
    let mut completeness = VcsImpactCompleteness::Exact;
    let mut observation = input.observation_id.splitn(4, '\0');
    let observed_current = observation
        .next()
        .ok_or_else(|| anyhow!("invalid Git observation identifier"))?;
    let workspace_prefix = observation
        .next()
        .ok_or_else(|| anyhow!("invalid Git observation identifier"))?;
    let workspace_key = observation
        .next()
        .ok_or_else(|| anyhow!("invalid Git observation identifier"))?;
    let remote_candidates = observation
        .next()
        .map(json::from_str::<Vec<String>>)
        .transpose()?
        .unwrap_or_else(|| vec!["origin".into(), "upstream".into()]);
    let pinned_workspace = var::get::<String>(workspace_key)?
        .ok_or_else(|| anyhow!("Git observation is no longer available"))?;
    let pinned_workspace: Vec<VcsPathEffect> = json::from_str(&pinned_workspace)?;
    let mut effects = match input.intent {
        VcsImpactIntent::Workspace => pinned_workspace,
        VcsImpactIntent::Submission {
            base,
            head,
            include_workspace,
        } => {
            let head = if let Some(head) = head {
                resolve_ref(&input.context, &head, &remote_candidates)?
            } else {
                observed_current.to_owned()
            };
            let mut effects = if let Some(base) = base {
                let base = resolve_ref(&input.context, &base, &remote_candidates)?;
                let merge_base = run_git(&input.context, ["merge-base", &base, &head])?;
                let from = if merge_base.exit_code == 0 && !merge_base.stdout.trim().is_empty() {
                    merge_base.stdout.trim().to_owned()
                } else {
                    completeness = VcsImpactCompleteness::Conservative;
                    diagnostics.push(format!(
                        "no common state for {base} and {head}; compared the states directly"
                    ));
                    base
                };

                diff_effects(&input.context, &from, &head, workspace_prefix)?
            } else {
                current_change_effects(&input.context, &head, workspace_prefix)?
            };

            if include_workspace {
                effects.extend(pinned_workspace);
            }

            effects
        }
    };

    dedupe_effects(&mut effects);

    Ok(Json(GetVcsImpactsOutput {
        effects,
        completeness,
        diagnostics,
    }))
}

#[plugin_fn]
pub fn setup_vcs_hooks(
    Json(input): Json<SetupVcsHooksInput>,
) -> FnResult<Json<SetupVcsHooksOutput>> {
    let hooks_dir = format!(
        "{}{}",
        get_workspace_prefix(&input.context)?,
        input.hooks_dir
    );
    let mut output = run_git(
        &input.context,
        ["config", "--worktree", "core.hooksPath", hooks_dir.as_str()],
    )?;

    if output.exit_code != 0 {
        require_success(&run_git(
            &input.context,
            ["config", "extensions.worktreeConfig", "true"],
        )?)?;
        output = run_git(
            &input.context,
            ["config", "--worktree", "core.hooksPath", hooks_dir.as_str()],
        )?;
    }
    require_success(&output)?;

    Ok(Json(SetupVcsHooksOutput {
        hooks_dir: None,
        working_dir: Some(input.context.workspace_root.to_string()),
    }))
}

#[plugin_fn]
pub fn teardown_vcs_hooks(
    Json(input): Json<TeardownVcsHooksInput>,
) -> FnResult<Json<TeardownVcsHooksOutput>> {
    let output = run_git(
        &input.context,
        ["config", "--worktree", "--unset", "core.hooksPath"],
    )?;

    Ok(Json(TeardownVcsHooksOutput {
        removed: output.exit_code == 0,
    }))
}

fn git_version(context: &MoonContext) -> Option<String> {
    successful_stdout(run_git(context, ["--version"]).ok()?).map(|output| {
        output
            .split_whitespace()
            .find(|part| {
                part.chars()
                    .next()
                    .is_some_and(|char| char.is_ascii_digit())
            })
            .unwrap_or(&output)
            .to_owned()
    })
}

fn repository_slug(context: &MoonContext, remote_candidates: &[String]) -> Option<String> {
    for remote in remote_candidates {
        let Some(url) = successful_stdout(run_git(context, ["remote", "get-url", remote]).ok()?)
        else {
            continue;
        };

        let url = url.trim_end_matches('/').trim_end_matches(".git");
        let path = if let Some((_, path)) = url.rsplit_once(':') {
            path
        } else if let Some((_, path)) = url.split_once("://") {
            path.split_once('/').map(|(_, path)| path).unwrap_or(path)
        } else {
            url
        };
        let segments = path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if segments.len() >= 2 {
            return Some(format!(
                "{}/{}",
                segments[segments.len() - 2],
                segments[segments.len() - 1]
            ));
        }
    }

    None
}

fn resolve_ref(
    context: &MoonContext,
    value: &str,
    remote_candidates: &[String],
) -> AnyResult<String> {
    validate_ref(value)?;

    for candidate in std::iter::once(value.to_owned()).chain(
        remote_candidates
            .iter()
            .map(|remote| format!("{remote}/{value}")),
    ) {
        let output = run_git(
            context,
            ["rev-parse", "--verify", &format!("{candidate}^{{commit}}")],
        )?;

        if let Some(value) = successful_stdout(output) {
            return Ok(value);
        }
    }

    Err(anyhow!("reference `{value}` did not resolve to one state"))
}

fn validate_ref(value: &str) -> AnyResult<()> {
    if value.starts_with('-') || value.contains('\0') {
        Err(anyhow!("invalid source-control reference `{value}`"))
    } else {
        Ok(())
    }
}

fn current_change_effects(
    context: &MoonContext,
    head: &str,
    workspace_prefix: &str,
) -> AnyResult<Vec<VcsPathEffect>> {
    let parents = run_git(context, ["rev-list", "--parents", "-n", "1", head])?;
    require_success(&parents)?;
    let ids = parents.stdout.split_whitespace().collect::<Vec<_>>();

    if ids.len() > 1 {
        diff_effects(context, ids[1], head, workspace_prefix)
    } else {
        Ok(vec![])
    }
}

fn diff_effects(
    context: &MoonContext,
    from: &str,
    to: &str,
    workspace_prefix: &str,
) -> AnyResult<Vec<VcsPathEffect>> {
    let output = run_git(
        context,
        [
            "diff",
            "--name-status",
            "-z",
            "--find-renames",
            "--no-ext-diff",
            "--no-textconv",
            from,
            to,
            "--",
            ".",
        ],
    )?;
    require_success(&output)?;

    let mut effects = parse_diff(&output.stdout);

    for submodule in submodule_paths(context)? {
        let Some(relative) = submodule.strip_prefix(workspace_prefix) else {
            continue;
        };
        let (Some(base), Some(head)) = (
            submodule_commit(context, from, &submodule)?,
            submodule_commit(context, to, &submodule)?,
        ) else {
            continue;
        };

        if base == head {
            continue;
        }

        let output = run_git_at(
            context,
            context.workspace_root.join(relative),
            [
                "diff",
                "--name-status",
                "-z",
                "--find-renames",
                "--no-ext-diff",
                "--no-textconv",
                &base,
                &head,
                "--",
                ".",
            ],
        )?;
        require_success(&output)?;
        effects.retain(|effect| {
            effect.before.as_deref() != Some(&submodule)
                && effect.after.as_deref() != Some(&submodule)
        });
        effects.extend(prefix_effects(parse_diff(&output.stdout), &submodule));
    }

    Ok(scope_effects(effects, workspace_prefix))
}

fn submodule_commit(
    context: &MoonContext,
    revision: &str,
    path: &str,
) -> AnyResult<Option<String>> {
    let output = run_git(context, ["ls-tree", "-z", revision, "--", path])?;
    require_success(&output)?;

    Ok(output.stdout.split_once('\t').and_then(|(metadata, _)| {
        let mut fields = metadata.split_whitespace();
        (fields.next() == Some("160000") && fields.next() == Some("commit"))
            .then(|| fields.next().map(String::from))
            .flatten()
    }))
}

fn workspace_effects(
    context: &MoonContext,
    workspace_prefix: &str,
) -> AnyResult<Vec<VcsPathEffect>> {
    let output = run_git(
        context,
        [
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules",
            "-z",
            "--",
            ".",
        ],
    )?;
    require_success(&output)?;

    let mut effects = parse_status(&output.stdout);

    for submodule in submodule_paths(context)? {
        let Some(relative) = submodule.strip_prefix(workspace_prefix) else {
            continue;
        };
        let output = run_git_at(
            context,
            context.workspace_root.join(relative),
            [
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
                "--ignore-submodules",
                "-z",
                "--",
                ".",
            ],
        )?;

        if output.exit_code == 0 {
            effects.extend(prefix_effects(parse_status(&output.stdout), &submodule));
        }
    }

    Ok(scope_effects(effects, workspace_prefix))
}

fn submodule_paths(context: &MoonContext) -> AnyResult<Vec<String>> {
    let output = run_git(
        context,
        ["ls-files", "--stage", "--full-name", "-z", "--", "."],
    )?;
    require_success(&output)?;

    Ok(output
        .stdout
        .split('\0')
        .filter_map(|entry| {
            let (metadata, path) = entry.split_once('\t')?;
            metadata.starts_with("160000 ").then(|| path.to_owned())
        })
        .collect())
}

fn prefix_effects(mut effects: Vec<VcsPathEffect>, prefix: &str) -> Vec<VcsPathEffect> {
    for effect in &mut effects {
        effect.before = effect.before.take().map(|path| format!("{prefix}/{path}"));
        effect.after = effect.after.take().map(|path| format!("{prefix}/{path}"));
    }

    effects
}

fn get_workspace_prefix(context: &MoonContext) -> AnyResult<String> {
    let output = run_git(context, ["rev-parse", "--show-prefix"])?;
    require_success(&output)?;

    Ok(output.stdout.trim().into())
}

fn scope_effects(effects: Vec<VcsPathEffect>, workspace_prefix: &str) -> Vec<VcsPathEffect> {
    if workspace_prefix.is_empty() {
        return effects;
    }

    effects
        .into_iter()
        .filter_map(|effect| {
            let before = effect
                .before
                .and_then(|path| path.strip_prefix(workspace_prefix).map(String::from));
            let after = effect
                .after
                .and_then(|path| path.strip_prefix(workspace_prefix).map(String::from));

            (before.is_some() || after.is_some()).then_some(VcsPathEffect {
                before,
                after,
                layers: effect.layers,
            })
        })
        .collect()
}

fn parse_diff(output: &str) -> Vec<VcsPathEffect> {
    let mut effects = vec![];
    let mut fields = output.split('\0');

    while let Some(status) = fields.next() {
        if status.is_empty() {
            continue;
        }

        let Some(path) = fields.next().filter(|path| !path.is_empty()) else {
            break;
        };
        let layer = vec![VcsChangeLayer::Recorded];

        if status.starts_with('R') || status.starts_with('C') {
            let Some(target) = fields.next().filter(|path| !path.is_empty()) else {
                break;
            };
            effects.push(VcsPathEffect {
                before: status.starts_with('R').then(|| path.into()),
                after: Some(target.into()),
                layers: layer,
            });
        } else {
            effects.push(VcsPathEffect {
                before: (status != "A").then(|| path.into()),
                after: (status != "D").then(|| path.into()),
                layers: layer,
            });
        }
    }

    effects
}

fn parse_status(output: &str) -> Vec<VcsPathEffect> {
    let mut effects = vec![];
    let mut fields = output.split('\0');

    while let Some(entry) = fields.next() {
        if entry.len() < 4 {
            continue;
        }

        let mut chars = entry.chars();
        let index = chars.next().unwrap_or(' ');
        let workspace = chars.next().unwrap_or(' ');
        let path = &entry[3..];
        if index == '?' && workspace == '?' {
            effects.push(status_effect('?', path, None, VcsChangeLayer::Untracked));
            continue;
        }

        let source = if index == 'R' || index == 'C' || workspace == 'R' || workspace == 'C' {
            fields.next().filter(|path| !path.is_empty())
        } else {
            None
        };

        if index != ' ' {
            effects.push(status_effect(index, path, source, VcsChangeLayer::Staged));
        }
        if workspace != ' ' {
            effects.push(status_effect(
                workspace,
                path,
                source,
                VcsChangeLayer::Workspace,
            ));
        }
    }

    effects
}

fn status_effect(
    status: char,
    path: &str,
    source: Option<&str>,
    layer: VcsChangeLayer,
) -> VcsPathEffect {
    let (before, after) = match status {
        'A' | 'C' | '?' => (None, Some(path.into())),
        'D' => (Some(path.into()), None),
        'R' => (source.map(String::from), Some(path.into())),
        _ => (Some(path.into()), Some(path.into())),
    };

    VcsPathEffect {
        before,
        after,
        layers: vec![layer],
    }
}

fn dedupe_effects(effects: &mut Vec<VcsPathEffect>) {
    let mut unique: Vec<VcsPathEffect> = vec![];

    for mut effect in effects.drain(..) {
        if let Some(candidate) = unique
            .iter_mut()
            .find(|candidate| candidate.before == effect.before && candidate.after == effect.after)
        {
            for layer in effect.layers.drain(..) {
                if !candidate.layers.contains(&layer) {
                    candidate.layers.push(layer);
                }
            }
        } else {
            unique.push(effect);
        }
    }

    *effects = unique;
}

fn run_git<I, S>(context: &MoonContext, args: I) -> AnyResult<ExecCommandOutput>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    run_git_at(context, context.workspace_root.clone(), args)
}

fn run_git_at<I, S>(
    _context: &MoonContext,
    cwd: VirtualPath,
    args: I,
) -> AnyResult<ExecCommandOutput>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut command =
        ExecCommandInput::pipe("git", args.into_iter().map(Into::into).collect::<Vec<_>>());
    command.cwd = Some(cwd);
    exec(command)
}

fn successful_stdout(output: ExecCommandOutput) -> Option<String> {
    (output.exit_code == 0).then(|| output.stdout.trim().to_owned())
}

fn require_success(output: &ExecCommandOutput) -> AnyResult<()> {
    if output.exit_code == 0 {
        Ok(())
    } else {
        Err(anyhow!(output.get_output()))
    }
}

fn short_id(id: &str) -> String {
    id[..id.len().min(8)].into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_mixed_index_and_workspace_statuses() {
        let effects = parse_status("AM added.txt\0MD deleted.txt\0");

        assert!(effects.contains(&VcsPathEffect {
            before: None,
            after: Some("added.txt".into()),
            layers: vec![VcsChangeLayer::Staged],
        }));
        assert!(effects.contains(&VcsPathEffect {
            before: Some("added.txt".into()),
            after: Some("added.txt".into()),
            layers: vec![VcsChangeLayer::Workspace],
        }));
        assert!(effects.contains(&VcsPathEffect {
            before: Some("deleted.txt".into()),
            after: Some("deleted.txt".into()),
            layers: vec![VcsChangeLayer::Staged],
        }));
        assert!(effects.contains(&VcsPathEffect {
            before: Some("deleted.txt".into()),
            after: None,
            layers: vec![VcsChangeLayer::Workspace],
        }));
    }

    #[test]
    fn does_not_delete_the_source_of_a_copy() {
        assert_eq!(
            parse_diff("C100\0source.txt\0copy.txt\0"),
            vec![VcsPathEffect {
                before: None,
                after: Some("copy.txt".into()),
                layers: vec![VcsChangeLayer::Recorded],
            }]
        );
        assert_eq!(
            parse_status("C  copy.txt\0source.txt\0"),
            vec![VcsPathEffect {
                before: None,
                after: Some("copy.txt".into()),
                layers: vec![VcsChangeLayer::Staged],
            }]
        );
    }
}
