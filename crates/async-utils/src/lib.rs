use miette::IntoDiagnostic;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::task::JoinSet;

pub fn get_concurrency() -> usize {
    static CONCURRENCY: OnceLock<usize> = OnceLock::new();

    *CONCURRENCY.get_or_init(num_cpus::get)
}

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
    let concurrency = get_concurrency();
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

pub async fn run_pooled_blocking_tasks<I, O, In, Out>(
    inputs: Vec<I>,
    on_input: In,
    mut on_output: Out,
) -> miette::Result<()>
where
    I: Send + Sync + 'static,
    O: Send + 'static,
    In: Fn(&I) -> miette::Result<O> + Send + Sync + 'static,
    Out: FnMut(O) -> miette::Result<()>,
{
    if inputs.is_empty() {
        return Ok(());
    }

    let inputs = Arc::new(inputs);
    let on_input = Arc::new(on_input);
    let next_index = Arc::new(AtomicUsize::new(0));
    // Blocking tasks can't be aborted, so they stop pulling inputs instead
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut set = JoinSet::new();

    for _ in 0..get_concurrency().min(inputs.len()) {
        let inputs = Arc::clone(&inputs);
        let on_input = Arc::clone(&on_input);
        let next_index = Arc::clone(&next_index);
        let cancelled = Arc::clone(&cancelled);

        set.spawn_blocking(move || {
            let mut outputs = vec![];

            while !cancelled.load(Ordering::Relaxed) {
                let index = next_index.fetch_add(1, Ordering::Relaxed);

                let Some(input) = inputs.get(index) else {
                    break;
                };

                match on_input(input) {
                    Ok(output) => outputs.push((index, output)),
                    Err(error) => {
                        cancelled.store(true, Ordering::Relaxed);

                        return Err(error);
                    }
                }
            }

            Ok(outputs)
        });
    }

    let mut completed = BTreeMap::new();

    while let Some(result) = set.join_next().await {
        match result.into_diagnostic() {
            Ok(Ok(outputs)) => {
                completed.extend(outputs);
            }
            Ok(Err(error)) | Err(error) => {
                cancelled.store(true, Ordering::Relaxed);

                return Err(error);
            }
        }
    }

    for (_, output) in completed {
        on_output(output)?;
    }

    Ok(())
}
