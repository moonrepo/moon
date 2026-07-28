use miette::IntoDiagnostic;
use moon_config::RemoteConfig;
use moon_env_var::{EnvSubstitutor, GlobalEnvBag};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tracing::warn;

pub fn extract_headers(config: &RemoteConfig) -> miette::Result<Option<HeaderMap>> {
    let env_bag = GlobalEnvBag::instance();
    let mut headers = HeaderMap::default();
    let mut substitutor = EnvSubstitutor::default().with_global_vars(env_bag);

    if let Some(auth) = &config.auth {
        for (key, value) in &auth.headers {
            let value = substitutor.substitute(value);

            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).into_diagnostic()?,
                HeaderValue::from_str(&value).into_diagnostic()?,
            );
        }

        if let Some(token_name) = &auth.token {
            let token = env_bag.get(token_name).unwrap_or_default();

            if token.is_empty() {
                // Allow unauthed locally!
                if !config.cache.local_read_only {
                    warn!(
                        "Auth token {} does not exist, unable to authorize for remote storage",
                        moon_common::color::property(token_name)
                    );

                    return Ok(None);
                }
            } else {
                let mut value =
                    HeaderValue::from_str(&format!("Bearer {token}")).into_diagnostic()?;
                value.set_sensitive(true);

                headers.insert(
                    HeaderName::from_bytes("Authorization".as_bytes()).into_diagnostic()?,
                    value,
                );
            }
        }
    }

    Ok(Some(headers))
}
