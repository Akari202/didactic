use ecow::EcoVec;
use thiserror::Error;
use typst::diag::SourceDiagnostic;

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
    MissingTitle(String),
    #[error("Html body not found")]
    MissingBody,
    #[error("Error: {:?}", 0)]
    EngineError(#[from] CompilationDiagnostics),
    #[error("Path parsing error: {0}")]
    PathError(#[from] typst_syntax::PathError),
    #[error("Files were not mapped before compilation")]
    FilesNotMapped
}

#[derive(Debug, Clone)]
pub struct CompilationDiagnostics(pub EcoVec<SourceDiagnostic>);

impl std::fmt::Display for CompilationDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let structural_errors = self
            .0
            .iter()
            .map(|e| format!("[:{:?}] {}", e.span, e.message))
            .collect::<Vec<_>>()
            .join("\n");
        write!(f, "Typst compilation failed:\n{}", structural_errors)
    }
}

impl std::error::Error for CompilationDiagnostics {}

impl From<EcoVec<SourceDiagnostic>> for CompilationDiagnostics {
    fn from(errors: EcoVec<SourceDiagnostic>) -> Self {
        Self(errors)
    }
}
