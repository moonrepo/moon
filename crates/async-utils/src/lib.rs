use miette::IntoDiagnostic;
use std::collections::VecDeque;
use tokio::task::JoinSet;

pub async fn run_pooled_tasks<I, O, In, Fut, Out>(
    mut queue: VecDeque<I>,
    mut on_input: In,
    mut on_output: Out,
) -> miette::Result<()>
where
    O: Send + 'static,
    In: FnMut(I) -> miette::Result<Fut>,
    Fut: Future<Output = miette::Result<O>> + Send + 'static,
    Out: FnMut(O) -> miette::Result<()>,
{
    let concurrency = num_cpus::get();
    let mut set = JoinSet::new();

    loop {
        if let Some(input) = queue.pop_front() {
            match on_input(input) {
                Ok(future) => {
                    set.spawn(Box::pin(future));
                }
                Err(error) => {
                    set.abort_all();

                    return Err(error);
                }
            };
        }

        // Keep enqueuing until we hit the concurrency limit
        if set.len() < concurrency && !queue.is_empty() {
            continue;
        }

        // If all tasks are complete, or the queue is empty, break the loop
        let Some(result) = set.join_next().await else {
            break;
        };

        // Unwrap the output and handle all errors
        match result.into_diagnostic() {
            Ok(Ok(output)) => {
                if let Err(error) = on_output(output) {
                    set.abort_all();

                    return Err(error);
                }
            }
            Ok(Err(error)) | Err(error) => {
                set.abort_all();

                return Err(error);
            }
        };
    }

    Ok(())
}
