use miette::IntoDiagnostic;
use moon_config::{RemoteMtlsConfig, RemoteTlsConfig};
use reqwest::{Certificate, ClientBuilder, Identity};
use starbase_utils::fs;
use std::path::Path;
use tracing::trace;

pub fn create_native_tls_config(client: ClientBuilder) -> miette::Result<ClientBuilder> {
    Ok(client
        .use_rustls_tls()
        .tls_built_in_native_certs(true)
        .https_only(true))
}

pub fn create_tls_config(
    client: ClientBuilder,
    config: &RemoteTlsConfig,
    workspace_root: &Path,
) -> miette::Result<ClientBuilder> {
    let cert = workspace_root.join(&config.cert);

    trace!(
        cert = ?cert,
        "Configuring TLS",
    );

    let client = client
        .use_rustls_tls()
        .tls_built_in_native_certs(false)
        .add_root_certificate(Certificate::from_pem(&fs::read_file_bytes(cert)?).into_diagnostic()?)
        .https_only(true);

    Ok(client)
}

pub fn create_mtls_config(
    client: ClientBuilder,
    config: &RemoteMtlsConfig,
    workspace_root: &Path,
) -> miette::Result<ClientBuilder> {
    let client_cert = workspace_root.join(&config.client_cert);
    let client_key = workspace_root.join(&config.client_key);
    let ca_cert = workspace_root.join(&config.ca_cert);

    trace!(
        client_cert = ?client_cert,
        client_key = ?client_key,
        ca_cert = ?ca_cert,
        "Configuring mTLS",
    );

    // Is this correct?
    let mut identity_buf = Vec::new();
    identity_buf.extend(fs::read_file_bytes(client_key)?);
    identity_buf.extend(fs::read_file_bytes(client_cert)?);

    let client = client
        .use_rustls_tls()
        .tls_built_in_native_certs(false)
        .add_root_certificate(
            Certificate::from_pem(&fs::read_file_bytes(ca_cert)?).into_diagnostic()?,
        )
        .identity(Identity::from_pem(&identity_buf).into_diagnostic()?)
        .https_only(true);

    Ok(client)
}
