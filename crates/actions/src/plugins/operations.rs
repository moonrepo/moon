use convert_case::{Case, Casing};
use moon_action::{Action, ActionStatus, Operation};
use moon_common::Id;
use moon_pdk_api::{Operation as PluginOperation, OperationStatus, SyncOutput};
use moon_time::chrono::{DateTime, Local};
use moon_toolchain_plugin::{CallResult, ToolchainPlugin};
use std::path::PathBuf;

pub fn convert_plugin_operation(
    toolchain: &ToolchainPlugin,
    base: PluginOperation,
) -> miette::Result<Operation> {
    let mut op = Operation::sync_operation(base.id.to_case(Case::Kebab))?;

    op.plugin = Some(Id::new(&toolchain.id)?);

    op.started_at = base
        .started_at
        .map(|ts| DateTime::<Local>::from(ts).naive_utc())
        .unwrap();

    op.finished_at = base
        .finished_at
        .map(|ts| DateTime::<Local>::from(ts).naive_utc());

    op.duration = base.duration;

    op.status = match base.status {
        OperationStatus::Pending => ActionStatus::Running,
        OperationStatus::Failed => ActionStatus::Failed,
        OperationStatus::Passed => ActionStatus::Passed,
    };

    Ok(op)
}

pub fn convert_plugin_operations(
    toolchain: &ToolchainPlugin,
    base: Vec<PluginOperation>,
) -> miette::Result<Vec<Operation>> {
    let mut ops = vec![];

    for item in base {
        ops.push(convert_plugin_operation(toolchain, item)?);
    }

    Ok(ops)
}

pub fn inherit_changed_files(op: &mut Operation, files: Vec<PathBuf>) {
    if let Some(meta) = op.get_file_state_mut() {
        meta.changed_files.extend(files);
    }
}

pub fn finalize_action_operations(
    action: &mut Action,
    toolchain: &ToolchainPlugin,
    mut op: Operation,
    plugin_ops: Vec<PluginOperation>,
    changed_files: Vec<PathBuf>,
) -> miette::Result<()> {
    op.plugin = Some(Id::new(&toolchain.id)?);

    if op.status == ActionStatus::Running {
        op.finish(ActionStatus::Passed);
    }

    // Inherit plugin operations
    action
        .operations
        .extend(convert_plugin_operations(toolchain, plugin_ops)?);

    // Inherit changed files
    inherit_changed_files(&mut op, changed_files);

    action.operations.push(op);

    Ok(())
}

pub fn finalize_sync_operation(sync_result: CallResult<SyncOutput>) -> miette::Result<Operation> {
    // Add an operation for the overall sync
    let mut op = convert_plugin_operation(&sync_result.toolchain, sync_result.operation)?;

    // Inherit plugin operations
    op.operations.extend(convert_plugin_operations(
        &sync_result.toolchain,
        sync_result.output.operations,
    )?);

    // Inherit changed files
    inherit_changed_files(&mut op, sync_result.output.changed_files);

    Ok(op)
}
