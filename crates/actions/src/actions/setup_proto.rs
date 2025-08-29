use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::is_test_env;
use moon_common::path::exe_name;
use moon_console::Checkpoint;
use moon_env_var::GlobalEnvBag;
use moon_platform::is_using_global_toolchains;
use proto_core::flow::install::{InstallOptions, ProtoInstallError};
use proto_core::{Id, ToolContext, ToolSpec, is_offline, load_tool_from_locator};
use std::sync::Arc;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn setup_proto(
    _action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
) -> miette::Result<ActionStatus> {
    let bin_name = exe_name("proto");
    let proto_version = app_context.toolchain_config.proto.version.to_string();
    let install_dir = app_context
        .proto_env
        .store
        .inventory_dir
        .join("proto")
        .join(&proto_version);

    debug!(proto = ?install_dir.join(&bin_name), "Checking if proto is installed");

    // Set the version so that proto lookup paths take it into account
    let bag = GlobalEnvBag::instance();
    bag.set("PROTO_VERSION", &proto_version);
    bag.set("PROTO_IGNORE_MIGRATE_WARNING", "true");
    bag.set("PROTO_VERSION_CHECK", "false");
    bag.set("PROTO_LOOKUP_DIR", &install_dir);

    // This causes a ton of issues when running the test suite,
    // so just avoid it and assume proto exists!
    if install_dir.exists() || is_test_env() {
        debug!("proto has already been installed!");

        return Ok(ActionStatus::Skipped);
    }

    if is_using_global_toolchains(bag) || !app_context.toolchain_config.requires_proto() {
        debug!("Skipping proto install as the toolchain has been disabled or is not necessary");

        return Ok(ActionStatus::Skipped);
    }

    // If offline but a primary proto binary exists,
    // use that instead of failing, even if a different version!
    if is_offline() {
        let existing_bin = app_context.proto_env.store.bin_dir.join(&bin_name);

        if existing_bin.exists() {
            debug!(
                proto = ?existing_bin,
                "No internet connection, but using existing {} binary",
                bin_name
            );

            return Ok(ActionStatus::Skipped);
        } else {
            return Err(ProtoInstallError::RequiredInternetConnection.into());
        }
    }

    // Install proto
    let _lock = app_context.cache_engine.create_lock("proto-install")?;

    app_context.console.print_checkpoint(
        Checkpoint::Setup,
        format!("installing proto {proto_version}"),
    )?;

    // Load the built-in proto tool
    let mut tool = load_tool_from_locator(
        ToolContext::new(Id::raw("proto")),
        app_context.proto_env.clone(),
        app_context.proto_env.load_config()?.builtin_proto_plugin(),
    )
    .await?;

    // Install using proto itself
    let spec = ToolSpec::new(
        app_context
            .toolchain_config
            .proto
            .version
            .to_unresolved_spec(),
    );

    if tool.is_setup(&spec).await? {
        return Ok(ActionStatus::Skipped);
    }

    let record = tool
        .setup(
            &spec,
            InstallOptions {
                skip_prompts: true,
                skip_ui: true,
                ..Default::default()
            },
        )
        .await?;

    debug!("Successfully installed proto!");

    Ok(if record.is_some() {
        ActionStatus::Passed
    } else {
        ActionStatus::Skipped
    })
}
