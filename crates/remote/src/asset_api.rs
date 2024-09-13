use bazel_remote_apis::build::bazel::remote::asset::v1::fetch_client::FetchClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::push_client::PushClient;
use bazel_remote_apis::build::bazel::remote::asset::v1::PushBlobRequest;
use miette::IntoDiagnostic;
use tokio::sync::{OnceCell, RwLock};
use tonic::transport::Channel;

pub struct Asset {
    fetch_client: RwLock<FetchClient<Channel>>,
    push_client: RwLock<PushClient<Channel>>,
}

impl Asset {
    pub async fn new() -> miette::Result<Self> {
        let fetch_client = FetchClient::connect("http://[::1]:50051")
            .await
            .into_diagnostic()?;

        let push_client = PushClient::connect("http://[::1]:50051")
            .await
            .into_diagnostic()?;

        Ok(Self {
            fetch_client: RwLock::new(fetch_client),
            push_client: RwLock::new(push_client),
        })
    }

    pub fn create_push_blob_request(&self) -> PushBlobRequest {
        let mut request = PushBlobRequest::default();

        request
    }

    pub async fn push_asset(&self, request: PushBlobRequest) -> miette::Result<()> {
        let response = self
            .push_client
            .write()
            .await
            .push_blob(request)
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
