use crate::plugin::Plugin;
use crate::plugin_registry::*;
use futures::StreamExt;
use futures::stream::FuturesOrdered;
use miette::IntoDiagnostic;
use moon_common::Id;
use moon_pdk_api::Operation;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

#[derive(Debug)]
pub struct CallOptions {
    pub check_func_exists: bool,
}

// Most plugin functions are optional, so by default we must verify
// they exist before calling them, otherwise we fail with a
// "Function not found" error. This can be disabled for functions
// that are required, or are guarded by the caller.
impl Default for CallOptions {
    fn default() -> Self {
        Self {
            check_func_exists: true,
        }
    }
}

#[derive(Debug)]
pub struct CallResult<Inst: Plugin, Out: Debug> {
    pub id: Id,
    pub operation: Operation,
    pub output: Out,
    pub plugin: Arc<Inst>,
}

impl<Inst: Plugin, Out: Debug> CallResult<Inst, Out> {
    pub fn map_output<O: Debug>(self, op: impl FnOnce(Out) -> O) -> CallResult<Inst, O> {
        CallResult {
            id: self.id,
            operation: self.operation,
            output: op(self.output),
            plugin: self.plugin,
        }
    }
}

impl<Cfg: PluginsConfig, Inst: Plugin> PluginRegistry<Cfg, Inst> {
    pub async fn call_func<It, I, InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        ids: It,
        input_factory: InFn,
        output_factory: OutFn,
    ) -> miette::Result<Vec<CallResult<Inst, Out>>>
    where
        It: IntoIterator<Item = I>,
        I: AsRef<str> + Clone,
        InFn: Fn(&Inst) -> In,
        OutFn: Fn(Arc<Inst>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        self.call_func_with_options(
            func_name,
            ids,
            input_factory,
            output_factory,
            CallOptions::default(),
        )
        .await
    }

    pub async fn call_func_with_options<It, I, InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        ids: It,
        input_factory: InFn,
        output_factory: OutFn,
        options: CallOptions,
    ) -> miette::Result<Vec<CallResult<Inst, Out>>>
    where
        It: IntoIterator<Item = I>,
        I: AsRef<str> + Clone,
        InFn: Fn(&Inst) -> In,
        OutFn: Fn(Arc<Inst>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        self.internal_call_func(func_name, ids, input_factory, output_factory, options)
            .await
    }

    pub async fn call_func_all<InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        input_factory: InFn,
        output_factory: OutFn,
    ) -> miette::Result<Vec<CallResult<Inst, Out>>>
    where
        InFn: Fn(&Inst) -> In,
        OutFn: Fn(Arc<Inst>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        self.call_func_all_with_options(
            func_name,
            input_factory,
            output_factory,
            CallOptions::default(),
        )
        .await
    }

    pub async fn call_func_all_with_options<InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        input_factory: InFn,
        output_factory: OutFn,
        options: CallOptions,
    ) -> miette::Result<Vec<CallResult<Inst, Out>>>
    where
        InFn: Fn(&Inst) -> In,
        OutFn: Fn(Arc<Inst>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        self.internal_call_func(
            func_name,
            self.get_plugin_ids(),
            input_factory,
            output_factory,
            options,
        )
        .await
    }

    async fn internal_call_func<It, I, InFn, In, OutFn, OutFut, Out>(
        &self,
        func_name: &str,
        ids: It,
        input_factory: InFn,
        output_factory: OutFn,
        options: CallOptions,
    ) -> miette::Result<Vec<CallResult<Inst, Out>>>
    where
        It: IntoIterator<Item = I>,
        I: AsRef<str> + Clone,
        InFn: Fn(&Inst) -> In,
        OutFn: Fn(Arc<Inst>, In) -> OutFut,
        OutFut: Future<Output = miette::Result<Out>> + Send + 'static,
        Out: Debug + Send + 'static,
    {
        let mut results = vec![];

        // Load the plugins on-demand when we need them
        let plugins = self.load_many(ids).await?;

        if plugins.is_empty() {
            return Ok(results);
        }

        // Use ordered futures because we need the results to
        // be in a deterministic order for operations to work
        // correctly, like hashing
        let mut futures = FuturesOrdered::new();

        for plugin in plugins {
            if options.check_func_exists && !plugin.has_func(func_name).await {
                continue;
            }

            let mut operation = Operation::new(func_name).unwrap();
            let input = input_factory(&plugin);
            let future = output_factory(Arc::clone(&plugin), input);

            futures.push_back(tokio::spawn(Box::pin(async move {
                let result = future.await;

                operation.finish_with_result(&result);

                Ok::<_, miette::Report>(CallResult {
                    id: plugin.get_id().to_owned(),
                    operation,
                    output: result?,
                    plugin,
                })
            })));
        }

        while let Some(result) = futures.next().await {
            results.push(result.into_diagnostic()??);
        }

        Ok(results)
    }
}
