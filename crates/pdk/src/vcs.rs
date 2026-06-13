use extism_pdk::*;
use moon_pdk_api::{
    AnyResult, ProcessCommandInput, ProcessCommandOutput, ProcessCommandResult, ProcessOutputChunk,
    ProcessOutputChunkInput, ProcessOutputCloseInput, ProcessOutputCloseOutput,
    ProcessOutputStream,
};

#[host_fn]
extern "ExtismHost" {
    fn exec_process_command_v1(input: Json<ProcessCommandInput>) -> Json<ProcessCommandResult>;
    fn read_process_output_v1(input: Json<ProcessOutputChunkInput>) -> Json<ProcessOutputChunk>;
    fn close_process_output_v1(
        input: Json<ProcessOutputCloseInput>,
    ) -> Json<ProcessOutputCloseOutput>;
}

/// Execute an opaque process command on the host.
pub fn exec_process_command(input: ProcessCommandInput) -> AnyResult<ProcessCommandOutput> {
    let result = unsafe { exec_process_command_v1(Json(input))? }.0;
    let output = (|| {
        Ok(ProcessCommandOutput {
            exit_code: result.exit_code,
            stderr: read_output(&result, ProcessOutputStream::Stderr, result.stderr_len)?,
            stdout: read_output(&result, ProcessOutputStream::Stdout, result.stdout_len)?,
        })
    })();
    let close = unsafe {
        close_process_output_v1(Json(ProcessOutputCloseInput {
            result_id: result.result_id,
        }))
    };

    match (output, close) {
        (Ok(output), Ok(_)) => Ok(output),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn read_output(
    result: &ProcessCommandResult,
    stream: ProcessOutputStream,
    expected_len: u64,
) -> AnyResult<Vec<u8>> {
    let mut offset = 0u64;
    let mut output = Vec::new();

    loop {
        let chunk = unsafe {
            read_process_output_v1(Json(ProcessOutputChunkInput {
                result_id: result.result_id,
                stream: stream.clone(),
                offset,
            }))?
        }
        .0;
        let bytes = chunk.decode()?;

        if bytes.is_empty() && !chunk.eof {
            return Err(Error::msg("process output chunk made no progress"));
        }

        let next_offset = offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| Error::msg("process output length overflowed"))?;

        if next_offset > expected_len {
            return Err(Error::msg(format!(
                "process output exceeded declared length {expected_len}"
            )));
        }

        offset = next_offset;
        output.extend_from_slice(&bytes);

        if chunk.eof {
            break;
        }
    }

    if offset != expected_len {
        return Err(Error::msg(format!(
            "process output length mismatch: expected {expected_len}, received {offset}"
        )));
    }

    Ok(output)
}
