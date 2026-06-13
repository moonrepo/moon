//! PROTOTYPE: Jujutsu source-control provider.

use extism_pdk::*;
use moon_pdk_api::*;
use warpgate_pdk::*;

#[plugin_fn]
pub fn register_vcs(Json(input): Json<RegisterVcsInput>) -> FnResult<Json<VcsPluginMetadata>> {
    if input.host_protocol_version != VCS_PLUGIN_PROTOCOL_VERSION {
        return Err(anyhow!("unsupported host protocol {}", input.host_protocol_version).into());
    }

    Ok(Json(VcsPluginMetadata {
        name: "Jujutsu prototype provider".into(),
        description: Some("Answers Moon source-control intents with Jujutsu".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        protocol_version: VCS_PLUGIN_PROTOCOL_VERSION,
        supports_hooks: false,
    }))
}

#[plugin_fn]
pub fn detect_vcs(Json(input): Json<DetectVcsInput>) -> FnResult<Json<DetectVcsOutput>> {
    let output = match run_jj(
        &input.context,
        vec!["--ignore-working-copy".into(), "root".into()],
    ) {
        Ok(output) => output,
        Err(error) => {
            return Ok(Json(DetectVcsOutput {
                active: false,
                reason: format!("jj is unavailable ({error})"),
            }));
        }
    };

    Ok(Json(DetectVcsOutput {
        active: output.exit_code == 0,
        reason: if output.exit_code == 0 {
            format!("Jujutsu workspace at {}", output.stdout.trim())
        } else {
            "Jujutsu did not detect a workspace".into()
        },
    }))
}

#[plugin_fn]
pub fn observe_vcs(Json(input): Json<ObserveVcsInput>) -> FnResult<Json<VcsObservation>> {
    let mut args = vec![];

    if input.consistency == VcsConsistency::ExistingObservation {
        args.push("--ignore-working-copy".into());
    }

    args.extend([
        "op".into(),
        "log".into(),
        "--no-graph".into(),
        "-n".into(),
        "1".into(),
        "-T".into(),
        "id".into(),
    ]);

    let operation = run_jj(&input.context, args)?;
    require_success(&operation)?;
    let observation_id = operation.stdout.trim().to_owned();

    if observation_id.is_empty() {
        return Err(anyhow!("jj returned an empty operation ID").into());
    }

    let root = run_jj(
        &input.context,
        vec![format!("--at-operation={observation_id}"), "root".into()],
    )?;
    require_success(&root)?;
    let current = resolve_state(&input.context, &observation_id, "@")?;
    let baseline = input.baseline.and_then(|label| {
        resolve_state(&input.context, &observation_id, &label)
            .ok()
            .map(|mut state| {
                state.label = Some(label);
                state
            })
    });
    let version = run_jj(&input.context, vec!["--version".into()])?;

    Ok(Json(VcsObservation {
        id: observation_id,
        provider: "jj".into(),
        client_version: (version.exit_code == 0).then(|| {
            version
                .stdout
                .split_whitespace()
                .find(|part| {
                    part.chars()
                        .next()
                        .is_some_and(|char| char.is_ascii_digit())
                })
                .unwrap_or_default()
                .to_owned()
        }),
        enabled: true,
        repository_root: root.stdout.trim().into(),
        working_root: root.stdout.trim().into(),
        current,
        baseline,
        repository_slug: git_repository_slug(&input.context),
        history: VcsHistoryCompleteness::Complete,
    }))
}

#[plugin_fn]
pub fn get_vcs_impacts(
    Json(input): Json<GetVcsImpactsInput>,
) -> FnResult<Json<GetVcsImpactsOutput>> {
    let workspace_prefix = get_workspace_prefix(&input.context, &input.observation_id)?;
    let mut effects = match input.intent {
        VcsImpactIntent::Workspace => {
            working_copy_effects(&input.context, &input.observation_id, &workspace_prefix)?
        }
        VcsImpactIntent::Submission {
            base,
            head,
            include_workspace,
        } => {
            let head = resolve_revision(
                &input.context,
                &input.observation_id,
                head.as_deref().unwrap_or("@"),
            )?;
            let mut effects = if let Some(base) = base {
                let base = resolve_revision(&input.context, &input.observation_id, &base)?;
                between_effects(
                    &input.context,
                    &input.observation_id,
                    &base,
                    &head,
                    &workspace_prefix,
                )?
            } else {
                let previous =
                    resolve_previous_revision(&input.context, &input.observation_id, &head)?;
                diff_effects(
                    &input.context,
                    &input.observation_id,
                    vec!["--from".into(), previous, "--to".into(), head],
                    VcsChangeLayer::Recorded,
                    &workspace_prefix,
                )?
            };

            if include_workspace {
                effects.extend(working_copy_effects(
                    &input.context,
                    &input.observation_id,
                    &workspace_prefix,
                )?);
            }

            effects
        }
    };

    dedupe_effects(&mut effects);

    Ok(Json(GetVcsImpactsOutput {
        effects,
        completeness: VcsImpactCompleteness::Exact,
        diagnostics: vec![],
    }))
}

fn resolve_state(
    context: &MoonContext,
    observation_id: &str,
    revision: &str,
) -> AnyResult<VcsStateIdentity> {
    let revision = normalize_reference(revision)?;
    let output = run_jj(
        context,
        vec![
            format!("--at-operation={observation_id}"),
            "log".into(),
            "--no-graph".into(),
            "--color".into(),
            "never".into(),
            "-r".into(),
            revision.clone(),
            "-T".into(),
            "bookmarks.map(|bookmark| bookmark.name()).join(\" \") ++ \"\\0\" ++ change_id.short(8) ++ \"\\0\" ++ commit_id".into(),
        ],
    )?;
    require_success(&output)?;

    let mut fields = output.stdout.trim().split('\0');
    let bookmarks = fields.next().unwrap_or_default().trim();
    let change_id = fields.next().unwrap_or_default().trim();
    let commit_id = fields.next().unwrap_or_default().trim();

    if commit_id.is_empty() {
        return Err(anyhow!(
            "revision `{revision}` did not resolve to one state"
        ));
    }

    Ok(VcsStateIdentity {
        id: commit_id.into(),
        label: Some(
            bookmarks
                .split_whitespace()
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or(change_id)
                .into(),
        ),
    })
}

fn normalize_reference(reference: &str) -> AnyResult<String> {
    if reference.starts_with('-') || reference.contains('\0') {
        return Err(anyhow!("invalid source-control reference `{reference}`"));
    }

    Ok(if reference == "HEAD" {
        "@".into()
    } else {
        reference.into()
    })
}

fn resolve_revision(
    context: &MoonContext,
    observation_id: &str,
    revision: &str,
) -> AnyResult<String> {
    let expression = normalize_reference(revision)?;
    let output = run_jj(
        context,
        vec![
            format!("--at-operation={observation_id}"),
            "log".into(),
            "--no-graph".into(),
            "-r".into(),
            expression.clone(),
            "-T".into(),
            "commit_id ++ \"\\0\"".into(),
        ],
    )?;
    require_success(&output)?;

    let commit_ids = output
        .stdout
        .split('\0')
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if commit_ids.len() != 1 {
        return Err(anyhow!(
            "revision `{expression}` resolved to {} states; expected exactly one",
            commit_ids.len()
        ));
    }

    Ok(commit_ids[0].into())
}

fn working_copy_effects(
    context: &MoonContext,
    observation_id: &str,
    workspace_prefix: &str,
) -> AnyResult<Vec<VcsPathEffect>> {
    diff_effects(
        context,
        observation_id,
        vec!["-r".into(), "@".into()],
        VcsChangeLayer::Workspace,
        workspace_prefix,
    )
}

fn between_effects(
    context: &MoonContext,
    observation_id: &str,
    base: &str,
    head: &str,
    workspace_prefix: &str,
) -> AnyResult<Vec<VcsPathEffect>> {
    let merge_bases = resolve_merge_bases(context, observation_id, base, head)?;
    let (operation_id, merge_base) = if merge_bases.len() == 1 {
        (observation_id.into(), merge_bases[0].clone())
    } else {
        create_virtual_merge(context, observation_id, merge_bases)?
    };

    diff_effects(
        context,
        &operation_id,
        vec!["--from".into(), merge_base, "--to".into(), head.into()],
        VcsChangeLayer::Recorded,
        workspace_prefix,
    )
}

fn diff_effects(
    context: &MoonContext,
    observation_id: &str,
    mut args: Vec<String>,
    layer: VcsChangeLayer,
    workspace_prefix: &str,
) -> AnyResult<Vec<VcsPathEffect>> {
    let mut command_args = vec![format!("--at-operation={observation_id}"), "diff".into()];
    command_args.append(&mut args);
    command_args.extend([
        "-T".into(),
        "status_char ++ \"\\0\" ++ source.path() ++ \"\\0\" ++ target.path() ++ \"\\0\"".into(),
        "--color".into(),
        "never".into(),
        ".".into(),
    ]);
    let output = run_jj(context, command_args)?;
    require_success(&output)?;

    Ok(scope_effects(
        parse_effects(&output.stdout, layer),
        workspace_prefix,
    ))
}

fn get_workspace_prefix(context: &MoonContext, observation_id: &str) -> AnyResult<String> {
    let root = run_jj(
        context,
        vec![format!("--at-operation={observation_id}"), "root".into()],
    )?;
    require_success(&root)?;
    let workspace_root = context
        .workspace_root
        .real_path()
        .unwrap_or_else(|| context.workspace_root.to_path_buf());
    let repository_root = std::path::Path::new(root.stdout.trim());
    let relative = workspace_root
        .strip_prefix(repository_root)
        .ok()
        .or_else(|| {
            root.stdout
                .trim()
                .strip_prefix("/private")
                .and_then(|root| workspace_root.strip_prefix(root).ok())
        })
        .ok_or_else(|| {
            anyhow!(
                "Moon workspace `{}` is outside the Jujutsu repository `{}`",
                workspace_root.display(),
                repository_root.display()
            )
        })?;
    let prefix = relative.to_string_lossy().replace('\\', "/");

    Ok(if prefix.is_empty() {
        prefix
    } else {
        format!("{prefix}/")
    })
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

fn resolve_previous_revision(
    context: &MoonContext,
    observation_id: &str,
    revision: &str,
) -> AnyResult<String> {
    let output = run_jj(
        context,
        vec![
            format!("--at-operation={observation_id}"),
            "log".into(),
            "--no-graph".into(),
            "-r".into(),
            revision.into(),
            "-T".into(),
            "if(root, commit_id, if(parents.first().root(), commit_id, parents.first().commit_id()))"
                .into(),
        ],
    )?;
    require_success(&output)?;

    Ok(output.stdout.trim().into())
}

fn resolve_merge_bases(
    context: &MoonContext,
    observation_id: &str,
    base: &str,
    head: &str,
) -> AnyResult<Vec<String>> {
    let output = run_jj(
        context,
        vec![
            format!("--at-operation={observation_id}"),
            "log".into(),
            "--no-graph".into(),
            "-r".into(),
            format!("heads(::({base}) & ::({head}))"),
            "-T".into(),
            "commit_id ++ \"\\0\"".into(),
        ],
    )?;
    require_success(&output)?;

    let commit_ids = output
        .stdout
        .split('\0')
        .filter(|value| !value.is_empty())
        .map(String::from)
        .collect::<Vec<_>>();

    if commit_ids.is_empty() {
        return Err(anyhow!(
            "states `{base}` and `{head}` have no common history"
        ));
    }

    Ok(commit_ids)
}

fn create_virtual_merge(
    context: &MoonContext,
    observation_id: &str,
    merge_bases: Vec<String>,
) -> AnyResult<(String, String)> {
    let mut args = vec![
        format!("--at-operation={observation_id}"),
        "--no-integrate-operation".into(),
        "new".into(),
    ];
    args.extend(merge_bases);
    args.extend(["-m".into(), "moon VCS virtual merge base".into()]);
    let output = run_jj(context, args)?;
    require_success(&output)?;
    let operation_id = output
        .stdout
        .split_whitespace()
        .last()
        .or_else(|| output.stderr.split_whitespace().last())
        .unwrap_or_default()
        .trim()
        .to_owned();

    if operation_id.is_empty() || !operation_id.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(anyhow!("jj returned no isolated virtual-merge operation"));
    }

    let commit = resolve_revision(context, &operation_id, "@")?;

    Ok((operation_id, commit))
}

fn parse_effects(output: &str, layer: VcsChangeLayer) -> Vec<VcsPathEffect> {
    let mut effects = vec![];
    let mut fields = output.split('\0');

    while let Some(status) = fields.next() {
        if status.is_empty() {
            continue;
        }

        let source = fields.next().unwrap_or_default();
        let target = fields.next().unwrap_or_default();
        effects.push(VcsPathEffect {
            before: (status != "A").then(|| source.into()),
            after: (status != "D").then(|| target.into()),
            layers: vec![layer],
        });
    }

    effects
}

fn git_repository_slug(context: &MoonContext) -> Option<String> {
    for remote in ["origin", "upstream"] {
        let output = run_command(context, "git", ["remote", "get-url", remote]).ok()?;

        if output.exit_code != 0 {
            continue;
        }

        let url = output.stdout.trim().trim_end_matches(".git");
        let path = url.rsplit_once(':').map(|(_, path)| path).unwrap_or(url);
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

fn run_jj(context: &MoonContext, args: Vec<String>) -> AnyResult<ExecCommandOutput> {
    run_command(context, "jj", args)
}

fn run_command<I, S>(
    context: &MoonContext,
    command_name: &str,
    args: I,
) -> AnyResult<ExecCommandOutput>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut command = ExecCommandInput::pipe(
        command_name,
        args.into_iter().map(Into::into).collect::<Vec<_>>(),
    );
    command.cwd = Some(context.workspace_root.clone());
    exec(command)
}

fn require_success(output: &ExecCommandOutput) -> AnyResult<()> {
    if output.exit_code == 0 {
        Ok(())
    } else {
        Err(anyhow!(output.get_output()))
    }
}
