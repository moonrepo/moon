use convert_case::{Case, Casing};
use moon_action::{ActionStatus, Operation};
use moon_pdk_api::{Operation as PluginOperation, OperationStatus, SyncOutput};
use moon_time::chrono::{DateTime, Local};

pub fn convert_plugin_sync_operation(base: PluginOperation) -> Operation {
    let id_parts = base
        .id
        .split(':')
        .map(|part| part.to_case(Case::Kebab))
        .collect::<Vec<_>>();

    let mut op = Operation::sync_operation(id_parts.join(":"));

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

    op
}

pub fn convert_plugin_sync_operation_with_output(
    op: PluginOperation,
    output: SyncOutput,
) -> Operation {
    let mut op = convert_plugin_sync_operation(op);

    op.operations = output
        .operations_performed
        .into_iter()
        .map(convert_plugin_sync_operation)
        .collect();

    if let Some(meta) = op.get_sync_result_mut() {
        for file in output.changed_files {
            if let Some(file) = file.real_path() {
                meta.changed_files.push(file);
            }
        }
    }

    op
}
