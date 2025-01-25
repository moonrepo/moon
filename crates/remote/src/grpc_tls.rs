use moon_config::{RemoteMtlsConfig, RemoteTlsConfig};
use starbase_utils::fs;
use std::path::Path;
use tonic::transport::{Certificate, ClientTlsConfig, Identity};
use tracing::trace;

// TLS server:
//  - `*.pem` file
//  - `*.key` file
// TLS client:
//  - `*.pem` file
//  - domain name
// mTLS client:
//  - cert authority `*.pem` file
//  - client `*.pem` file
//  - client `*.key` file
//  - domain name

pub fn create_native_tls_config() -> miette::Result<ClientTlsConfig> {
    Ok(ClientTlsConfig::new().with_enabled_roots())
}

// https://github.com/hyperium/tonic/blob/master/examples/src/tls/client.rs
pub fn create_tls_config(
    config: &RemoteTlsConfig,
    workspace_root: &Path,
) -> miette::Result<ClientTlsConfig> {
    let cert = workspace_root.join(&config.cert);

    trace!(
        cert = ?cert,
        domain = &config.domain,
        http2 = config.assume_http2,
        "Configuring TLS",
    );

    let mut tls = ClientTlsConfig::new()
        .with_enabled_roots()
        .ca_certificate(Certificate::from_pem(fs::read_file_bytes(cert)?));

    if let Some(domain) = &config.domain {
        tls = tls.domain_name(domain.to_owned());
    }

    Ok(tls.assume_http2(config.assume_http2))
}

// https://github.com/hyperium/tonic/blob/master/examples/src/tls_client_auth/client.rs
pub fn create_mtls_config(
    config: &RemoteMtlsConfig,
    workspace_root: &Path,
) -> miette::Result<ClientTlsConfig> {
    let client_cert = workspace_root.join(&config.client_cert);
    let client_key = workspace_root.join(&config.client_key);
    let ca_cert = workspace_root.join(&config.ca_cert);

    trace!(
        client_cert = ?client_cert,
        client_key = ?client_key,
        ca_cert = ?ca_cert,
        domain = &config.domain,
        http2 = config.assume_http2,
        "Configuring mTLS",
    );

    let mut mtls = ClientTlsConfig::new()
        .with_enabled_roots()
        .ca_certificate(Certificate::from_pem(fs::read_file_bytes(ca_cert)?))
        .identity(Identity::from_pem(
            fs::read_file_bytes(client_cert)?,
            fs::read_file_bytes(client_key)?,
        ));

    if let Some(domain) = &config.domain {
        mtls = mtls.domain_name(domain.to_owned());
    }

    Ok(mtls.assume_http2(config.assume_http2))
}
