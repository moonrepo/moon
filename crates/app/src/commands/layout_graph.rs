use crate::session::{MoonSession, SessionResult};
use clap::Args;
use iocraft::prelude::{View, element};
use moon_common::Id;
use moon_console::ui::{Container, Entry, List, ListItem, Section, Style, StyledText};
use moon_layout_graph::{
    Binding, LayoutGraph, Placement, RootKind, ToolchainDependenciesRoot, get_root_label,
};
use moon_toolchain::{DependenciesWorkspaceRole, ToolchainSpec};
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct LayoutGraphArgs {
    #[arg(help = "Project ID to *only* place, like a run would")]
    id: Option<Id>,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[instrument(skip(session))]
pub async fn layout_graph(session: MoonSession, args: LayoutGraphArgs) -> SessionResult {
    let mut builder = session.build_layout_graph().await?;

    match &args.id {
        Some(id) => {
            let project = session.get_workspace_graph().await?.get_project(id)?;

            builder.place_project(project).await?;
        }
        None => {
            let mut toolchain_ids = session
                .toolchains_config
                .plugins
                .keys()
                .cloned()
                .collect::<Vec<_>>();

            toolchain_ids.sort();

            builder.place_all().await?;
            builder.place_workspace(toolchain_ids).await?;
        }
    };

    let graph = builder.build();

    if args.dot {
        session.console.out.write_line(graph.to_dot())?;

        return Ok(None);
    }

    if args.json {
        session.console.out.write_line(graph.to_json(true)?)?;

        return Ok(None);
    }

    render_graph(&session, &graph)?;

    Ok(None)
}

fn format_spec(spec: &ToolchainSpec) -> String {
    match &spec.req {
        Some(req) => req.to_string(),
        None => "global".into(),
    }
}

fn format_role(role: DependenciesWorkspaceRole) -> &'static str {
    match role {
        DependenciesWorkspaceRole::WorkspaceRoot => "workspace root",
        DependenciesWorkspaceRole::WorkspaceMember => "workspace member",
        DependenciesWorkspaceRole::PackageRoot => "package root",
    }
}

fn format_paths<T: AsRef<str>>(paths: &[T]) -> String {
    paths
        .iter()
        .map(|path| format!("<file>{}</file>", path.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_placement(graph: &LayoutGraph, binding: &Binding) -> String {
    match &binding.placement {
        Placement::Root { index, role } => format!(
            "{} <mutedlight>({})</mutedlight>",
            graph.root(*index).label(),
            format_role(*role)
        ),
        Placement::Outside { root } => format!(
            "{} <mutedlight>(outside, not a member)</mutedlight>",
            get_root_label(&binding.toolchain, root)
        ),
        Placement::None => "<mutedlight>no root</mutedlight>".into(),
    }
}

/// A member of a root, with its role, and its own version when it differs.
fn format_member(graph: &LayoutGraph, root: &ToolchainDependenciesRoot, member: &Id) -> String {
    let mut details = vec![];

    if let Some(binding) = graph.binding(member, &root.toolchain) {
        if let Some(role) = binding.role() {
            details.push(format_role(role).to_owned());
        }

        if binding.spec != root.spec {
            details.push(format!("runs tasks with {}", format_spec(&binding.spec)));
        }
    }

    if details.is_empty() {
        member.to_string()
    } else {
        format!("{member} <mutedlight>({})</mutedlight>", details.join(", "))
    }
}

fn render_graph(session: &MoonSession, graph: &LayoutGraph) -> miette::Result<()> {
    // Bindings that aren't in a root, as roots already list their members
    let unplaced = graph
        .iter_bindings()
        .flat_map(|(project_id, bindings)| {
            bindings
                .iter()
                .filter(|binding| binding.root_index().is_none())
                .map(move |binding| (project_id, binding))
        })
        .collect::<Vec<_>>();

    if graph.roots().is_empty() && unplaced.is_empty() && graph.workspace_bindings().is_empty() {
        session.console.render(element! {
            Container {
                StyledText(
                    content: "No toolchains were placed into the layout graph.",
                )
            }
        })?;

        return Ok(());
    }

    session.console.render(element! {
        Container {
            #(graph.roots().iter().map(|root| {
                let capabilities = [
                    root.capabilities.setup_environment.then_some("setup environment"),
                    root.capabilities.install_dependencies.then_some("install dependencies"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

                element! {
                    Section(title: root.label()) {
                        Entry(
                            name: "Kind",
                            content: match &root.kind {
                                RootKind::Workspace { .. } => "workspace",
                                RootKind::Package => "package",
                            }.to_owned(),
                        )
                        #(match &root.kind {
                            RootKind::Workspace { members } => Some(element! {
                                Entry(
                                    name: "Member globs",
                                    value: element! {
                                        StyledText(
                                            content: format_paths(members),
                                        )
                                    }.into_any()
                                )
                            }),
                            RootKind::Package => None,
                        })
                        Entry(
                            name: "Owner project",
                            value: element! {
                                StyledText(
                                    content: root
                                        .owner_project
                                        .as_ref()
                                        .map(|id| id.to_string())
                                        .unwrap_or_else(|| "<mutedlight>none</mutedlight>".into()),
                                    style: Style::Id
                                )
                            }.into_any()
                        )
                        Entry(
                            name: "Version",
                            content: format_spec(&root.spec),
                        )
                        #((!root.files.lockfiles.is_empty()).then(|| {
                            element! {
                                Entry(
                                    name: "Lockfiles",
                                    value: element! {
                                        StyledText(
                                            content: format_paths(&root.files.lockfiles),
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                        #((!root.files.manifests.is_empty()).then(|| {
                            element! {
                                Entry(
                                    name: "Manifests",
                                    value: element! {
                                        StyledText(
                                            content: format_paths(&root.files.manifests),
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                        #(root.files.vendor_dir.as_ref().map(|vendor_dir| {
                            element! {
                                Entry(
                                    name: "Vendor directory",
                                    value: element! {
                                        StyledText(
                                            content: vendor_dir.as_str(),
                                            style: Style::File
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                        Entry(
                            name: "Supports",
                            content: if capabilities.is_empty() {
                                "nothing".into()
                            } else {
                                capabilities.join(", ")
                            },
                        )
                        Entry(
                            name: "Projects",
                            no_children: root.members.is_empty()
                        ) {
                            List {
                                #(root.members.iter().map(|member| {
                                    element! {
                                        ListItem {
                                            StyledText(
                                                content: format_member(graph, root, member),
                                                style: Style::Id
                                            )
                                        }
                                    }
                                }))
                            }
                        }
                    }
                }.into_any()
            }))

            #((!unplaced.is_empty()).then(|| {
                element! {
                    Section(title: "Not in a root") {
                        #(unplaced.iter().map(|(project_id, binding)| {
                            element! {
                                Entry(
                                    name: format!("{project_id} ({})", binding.toolchain),
                                    value: element! {
                                        StyledText(
                                            content: format_placement(graph, binding),
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                    }
                }
            }))

            #((!graph.workspace_bindings().is_empty()).then(|| {
                element! {
                    Section(title: "Workspace root") {
                        #(graph.workspace_bindings().iter().map(|binding| {
                            element! {
                                Entry(
                                    name: binding.toolchain.as_str(),
                                    value: element! {
                                        StyledText(
                                            content: format_placement(graph, binding),
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                    }
                }
            }))

            #((!graph.is_complete()).then(|| {
                element! {
                    View {
                        StyledText(
                            content: "Only the requested project was placed, so roots only list the projects that were placed.",
                            style: Style::MutedLight
                        )
                    }
                }
            }))
        }
    })?;

    Ok(())
}
