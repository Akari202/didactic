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
    pub description: String,
    #[serde(default = "default_root_content_path")]
    pub root_content_path: String,
    #[serde(default = "default_static_content_path")]
    pub static_content_path: String,
    #[serde(default = "default_output_path")]
    pub output_path: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct LinkConfig {
    pub slug: String,
    pub path: String
}

fn default_root_content_path() -> String {
    "content".to_string()
}

fn default_static_content_path() -> String {
    "static".to_string()
}

fn default_output_path() -> String {
    "dist".to_string()
}
