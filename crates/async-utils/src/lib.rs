use miette::IntoDiagnostic;
use std::collections::{BTreeMap, VecDeque};
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

    // While tasks run concurrently and complete in any order, outputs are
    // applied in input order, so that consumers are deterministic
    let mut next_index = 0;
    let mut flush_index = 0;
    let mut completed = BTreeMap::new();

    loop {
        if let Some(input) = queue.pop_front() {
            match on_input(input) {
                Ok(future) => {
                    let index = next_index;
                    next_index += 1;

                    set.spawn(Box::pin(async move {
                        future.await.map(|output| (index, output))
                    }));
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
            Ok(Ok((index, output))) => {
                completed.insert(index, output);

                while let Some(output) = completed.remove(&flush_index) {
                    if let Err(error) = on_output(output) {
                        set.abort_all();

                        return Err(error);
                    }

                    flush_index += 1;
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
