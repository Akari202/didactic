use thiserror::Error;

#[derive(Error, Debug)]
pub enum DidacticError {
    #[error("No manifest file found in root directory")]
    MissingManifest,
    #[error("Failed to parse manifest: {0}")]
    ConfigParse(#[from] toml::de::Error),
    #[error("Io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path prefix error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),
    #[error("Path contains invalid UTF-8 characters: {0}")]
    InvalidUtf8(String),
    #[error("Root path contains invalid UTF-8 characters")]
    InvalidUtf8Root,
    #[error("Terra error: {0}")]
    Terra(#[from] tera::Error),
    #[error("Grass SCSS compilation error: {0}")]
    Grass(#[from] Box<grass::Error>),
    #[error("Document time format error: {0}")]
    TimeFormat(#[from] time::error::Format),
    #[error("Typst compilation failed for path '{path}': {details}")]
    TypstCompilationFailed { path: String, details: String },
    #[error("Missing document title: {0}")]
    MissingTitle(String)
}
