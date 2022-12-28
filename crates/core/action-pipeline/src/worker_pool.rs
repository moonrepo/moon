use async_channel::Sender;
use futures::future::FutureObj;
use std::future::Future;
use std::num::NonZeroUsize;
use std::thread;
use tokio::sync::oneshot;
use tokio::task;

// This is the type of the job that will be run in the worker pool. It consists of a closure
// that returns a `Future` and a `oneshot::Sender` that can be used to send the result of the
// future back to the main thread.
pub type Job<T> = (FutureObj<'static, T>, oneshot::Sender<T>);

// This is the worker thread that will run jobs. It consists of a `mpsc::Receiver` that
// is used to receive new jobs and a `handle` that it can use to spawn new tasks.
pub struct Worker {
    pub handle: task::JoinHandle<()>,
}

// This is the main worker pool struct. It consists of a `Vec` of `Worker` threads and a
// `Sender` that the main thread can use to send new jobs to the worker pool.
pub struct WorkerPool<T> {
    pub workers: Vec<Worker>,
    pub pool: Sender<Job<T>>,
}

impl<T> WorkerPool<T>
where
    T: Send + 'static,
{
    pub fn new(concurrency: Option<usize>) -> Self {
        let concurrency = concurrency.unwrap_or_else(|| {
            thread::available_parallelism()
                .unwrap_or(NonZeroUsize::new(8).unwrap())
                .get()
        });

        let (pool, worker) = async_channel::unbounded::<Job<T>>();
        let mut workers = vec![];

        for _ in 0..concurrency {
            let worker = worker.clone();

            let handle = task::spawn(async move {
                while let Ok(job) = worker.recv().await {
                    let (task, result) = job;
                    let _ = result.send(task.await);
                }
            });

            workers.push(Worker { handle });
        }

        WorkerPool { workers, pool }
    }

    pub async fn run(&self, future: impl Future<Output = T> + 'static + Send) -> T {
        let (sender, receiver) = oneshot::channel();

        self.pool
            .send((FutureObj::new(Box::new(future)), sender))
            .await
            .unwrap();

        receiver.await.unwrap()
    }
}
