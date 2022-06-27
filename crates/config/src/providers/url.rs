// Based on https://docs.rs/figment/latest/figment/trait.Provider.html

use figment::{
    providers::{Format, Yaml},
    value::{Dict, Map},
    Error, Metadata, Profile, Provider,
};

pub struct Url {
    url: String,
    pub profile: Option<Profile>,
}

impl Url {
    pub fn from(url: String) -> Self {
        Url { url, profile: None }
    }

    pub fn profile<P: Into<Profile>>(mut self, profile: P) -> Self {
        self.profile = Some(profile.into());
        self
    }
}

impl Provider for Url {
    fn metadata(&self) -> Metadata {
        Metadata::from("Extends", self.url.clone())
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        // Unfortunate we must use blocking here,
        // but figment doesn't support async/await
        let resp = reqwest::blocking::get(&self.url)
            .map_err(|e| {
                Error::from(format!(
                    "Failed to load extended config <url>{}</url>: {}",
                    self.url, e
                ))
            })?
            .text()
            .map_err(|e| {
                Error::from(format!(
                    "Failed to parse extended config <url>{}</url>: {}",
                    self.url, e
                ))
            })?;

        // We expect the URLs to point to YAML files,
        // so piggyback off the default YAML provider
        Yaml::string(&resp).data()
    }
}
