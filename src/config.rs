use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub site: SiteConfig
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct SiteConfig {
    pub title: String,
    pub author: String,
    pub base_url: String
}
