use async_trait::async_trait;
use std::future::Future;

#[async_trait]
pub trait JobAction<T>: Send {
    async fn run(self: Box<Self>) -> miette::Result<T>;
}

#[async_trait]
impl<T: Send, F: Send + Sync, Fut> JobAction<T> for F
where
    F: Fn() -> Fut,
    Fut: Future<Output = miette::Result<T>> + Send + 'static,
{
    async fn run(self: Box<Self>) -> miette::Result<T> {
        self().await
    }
}
