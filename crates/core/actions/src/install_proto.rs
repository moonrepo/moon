use moon_common::consts::PROTO_CLI_VERSION;
use moon_common::is_test_env;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_workspace::Workspace;
use proto_core::{is_offline, ProtoError};
use proto_installer::{determine_triple, download_release, unpack_release};
use std::env;
use tracing::debug;

pub async fn install_proto(workspace: &Workspace) -> miette::Result<()> {
    let install_dir = workspace
        .proto_env
        .tools_dir
        .join("proto")
        .join(PROTO_CLI_VERSION);

    // Set the version so that proto lookup paths take it into account
    env::set_var("PROTO_VERSION", PROTO_CLI_VERSION);

    // This causes a ton of issues when running the test suite,
    // so just avoid it and assume proto exists!
    if install_dir.exists() || is_test_env() || !workspace.toolchain_config.should_install_proto() {
        return Ok(());
    }

    debug!("Installing proto");

    print_checkpoint(
        format!("installing proto {}", PROTO_CLI_VERSION),
        Checkpoint::Setup,
    );

    if is_offline() {
        return Err(ProtoError::InternetConnectionRequired.into());
    }

    let target_triple = determine_triple()?;

    debug!("Downloading proto archive ({})", target_triple);

    let result = download_release(
        &target_triple,
        PROTO_CLI_VERSION,
        &workspace.proto_env.temp_dir,
        |_, _| {},
    )
    .await?;

    debug!("Unpacking archive and installing proto");

    unpack_release(result, &install_dir, &workspace.proto_env.tools_dir)?;

    debug!("Successfully installed proto!");

    Ok(())
}
