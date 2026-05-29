use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use comemo::Prehashed;
use ecow::{EcoString, EcoVec};
use log::warn;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};
use typst::diag::{FileError, FileResult, SourceDiagnostic, Warned};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Str, Value};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};
use typst_html::HtmlDocument;
use typst_kit::fonts::FontSource;

use crate::error::{CompilationDiagnostics, DidacticError};
use crate::file_map::{FileMap, LogicalPath, RealPath};

// TODO: remote packages
pub struct TypstEngine {
    library: LazyHash<Library>,
    font_book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    file_map: Option<FileMap>,
    comemo_evict_max_age: Option<usize>,
    pub root_dir: RealPath,
    io_cache: Arc<Mutex<HashMap<FileId, CacheSlot>>>
}

struct TypstWorld<'a> {
    pub library: &'a LazyHash<Library>,
    pub font_book: &'a LazyHash<FontBook>,
    pub fonts: &'a Vec<Font>,
    pub file_map: &'a FileMap,
    pub main_id: FileId,
    pub root_dir: &'a RealPath,
    pub io_cache: Arc<Mutex<HashMap<FileId, CacheSlot>>>
}

#[derive(Debug, Clone, Default)]
pub struct CacheSlot {
    pub source: Option<Source>,
    pub binary: Option<Bytes>
}

impl TypstEngine {
    pub fn new(comemo_evict_max_age: Option<usize>, root_dir: RealPath) -> Self {
        let mut inputs = Dict::new();
        inputs.insert("compile-host".into(), Value::Str(Str::from("didactic")));

        let library = Library::builder()
            .with_features([typst::Feature::Html].into_iter().collect())
            .with_inputs(inputs)
            .build();

        let mut font_book = FontBook::new();
        let mut fonts = Vec::new();

        // for (path, info) in typst_kit::fonts::system() {
        //     if let Some(font) = path.load() {
        //         font_book.push(info);
        //         fonts.push(font);
        //     }
        // }
        warn!("No custom or system fonts are being used");
        for (font, info) in typst_kit::fonts::embedded() {
            font_book.push(info);
            fonts.push(font);
        }

        Self {
            library: LazyHash::new(library),
            font_book: LazyHash::new(font_book),
            fonts,
            file_map: None,
            root_dir,
            comemo_evict_max_age,
            io_cache: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn set_file_map(&mut self, file_map: FileMap) {
        self.file_map = Some(file_map);
    }

    fn comemo_evict(&self) {
        if let Some(comemo_evict_max_age) = self.comemo_evict_max_age {
            comemo::evict(comemo_evict_max_age);
        }
    }

    fn build_world(&self, target: &LogicalPath) -> Result<TypstWorld<'_>, DidacticError> {
        let main_id = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(target.to_url_string())?
        )
        .intern();
        Ok(TypstWorld {
            library: &self.library,
            font_book: &self.font_book,
            fonts: &self.fonts,
            root_dir: &self.root_dir,
            file_map: self
                .file_map
                .as_ref()
                .ok_or(DidacticError::FilesNotMapped)?,
            main_id,
            io_cache: Arc::clone(&self.io_cache)
        })
    }

    pub fn compile_to_html(
        &self,
        target: &LogicalPath
    ) -> Result<Warned<Result<HtmlDocument, EcoVec<SourceDiagnostic>>>, DidacticError> {
        let world = self.build_world(target)?;
        let compilation_output = typst::compile::<HtmlDocument>(&world);
        self.comemo_evict();
        Ok(compilation_output)
    }
}

impl TypstWorld<'_> {
    fn resolve_path(&self, id: FileId) -> FileResult<RealPath> {
        Ok(match id.root() {
            VirtualRoot::Project => {
                let logical = LogicalPath::from(id);

                self.file_map
                    .get_real(&logical)
                    .ok_or_else(|| {
                        FileError::NotFound(
                            format!(
                                "File not mapped in project workspace. Logical path requested: {}",
                                logical.0.display()
                            )
                            .into()
                        )
                    })?
                    .clone()
            }
            VirtualRoot::Package(spec) => {
                let pkg_root = self.local_package_dir(spec);

                let virtual_subpath = id.vpath().get_without_slash();
                let resolved_path = id.vpath().realize(&pkg_root.0);

                if !resolved_path.exists() {
                    return Err(FileError::NotFound(
                        format!(
                            "Missing package: @local/{}/v{} | Expected file: {} | Inside directory: {}",
                            spec.name,
                            spec.version,
                            virtual_subpath,
                            pkg_root.0.display()
                        )
                        .into()
                    ));
                }

                RealPath(id.vpath().realize(&pkg_root))
            }
        })
    }

    fn local_package_dir(&self, spec: &PackageSpec) -> RealPath {
        let mut base = self.root_dir.0.clone();
        base.push("templates");
        base.push("packages");
        base.push("local");
        base.push(spec.name.as_str());
        base.push(spec.version.to_string());

        RealPath(base)
    }
}

impl typst::World for TypstWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.font_book
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let mut cache = self.io_cache.lock().unwrap();
        let slot = cache.entry(id).or_default();

        if let Some(source) = &slot.source {
            return Ok(source.clone());
        }

        let real = self.resolve_path(id)?;

        let text = std::fs::read_to_string(&real).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileError::NotFound(real.0.clone()),
            std::io::ErrorKind::PermissionDenied => FileError::AccessDenied,
            _ => FileError::Other(Some(ecow::eco_format!("Io Error: {}", e)))
        })?;

        let source = Source::new(id, text);
        slot.source = Some(source.clone());

        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let mut cache = self.io_cache.lock().unwrap();
        let slot = cache.entry(id).or_default();

        if let Some(bytes) = &slot.binary {
            return Ok(bytes.clone());
        }

        let real = self.resolve_path(id)?;

        let data = std::fs::read(&real).map_err(|_| FileError::AccessDenied)?;

        let bytes = Bytes::new(data);
        slot.binary = Some(bytes.clone());

        Ok(bytes)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        match offset {
            Some(offset) => {
                let now_utc = OffsetDateTime::now_utc();
                let adjusted_time = now_utc + Into::<time::Duration>::into(offset);
                Some(typst::foundations::Datetime::Datetime(
                    PrimitiveDateTime::new(adjusted_time.date(), adjusted_time.time())
                ))
            }
            None => match OffsetDateTime::now_local() {
                Ok(now_local) => Some(typst::foundations::Datetime::Datetime(
                    PrimitiveDateTime::new(now_local.date(), now_local.time())
                )),
                Err(e) => {
                    warn!("Unable to get local time: {e}");
                    None
                }
            }
        }
    }
}
