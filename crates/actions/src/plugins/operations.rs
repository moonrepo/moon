use convert_case::{Case, Casing};
use moon_action::{ActionStatus, Operation};
use moon_common::Id;
use moon_pdk_api::{Operation as PluginOperation, OperationStatus, VirtualPath};
use moon_time::chrono::{DateTime, Local};
use moon_toolchain_plugin::ToolchainPlugin;

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

pub fn inherit_changed_files(op: &mut Operation, files: Vec<VirtualPath>) {
    if let Some(meta) = op.get_sync_result_mut() {
        for file in files {
            if let Some(file) = file.real_path() {
                meta.changed_files.push(file);
            }
        }
    }
}
