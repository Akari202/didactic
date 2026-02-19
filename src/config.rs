use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub site: SiteConfig,
    #[serde(default)]
    pub links: Vec<LinkConfig>
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SiteConfig {
    pub title: String,
    pub author: String,
    pub base_url: String,
    #[serde(default)]
    pub description: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct LinkConfig {
    pub slug: String,
    pub path: String
}
