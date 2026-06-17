use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};
use regex::{Captures, Regex};
use serde::Serialize;
use tera::{Context, Tera};
use typst::comemo::Track;
use typst::foundations::{Dict, Smart, Str, Value, target};
use typst::introspection::Introspector;
use typst::model::{Document, LateLinkResolver};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode, html_in_bundle};
use typst_syntax::VirtualPath;
use xxhash_rust::xxh3::xxh3_64;

use crate::config::Config;
use crate::engine::TypstEngine;
use crate::error::{CompilationDiagnostics, DidacticError};
use crate::file_map::{FileMap, LogicalPath, RealPath};

/// The overall didactic execution context
pub struct World {
    pub root_dir: RealPath,
    pub output_path: RealPath,
    pub config: Config,
    pub tera: Tera,
    pub engine: TypstEngine,
    pub minify: bool
}

/// The dynamic build state
#[derive(Default, Debug)]
struct BuildState {
    pub file_map: FileMap,
    // TODO: overhaul cache busting
    // pub asset_hashes: HashMap<String, String>,
    pub document_cache: HashMap<LogicalPath, HtmlDocument>
}

/// Individual page metadata
#[derive(Debug, Hash, Clone, Serialize)]
struct PageMeta {
    pub title: String,
    pub url: String,
    // pub section: String,
    // pub children: Vec<PageMeta>,
    pub date: String,
    pub author: String
}

impl World {
    /// Initialize build tooling
    pub fn new(root_dir: PathBuf, minify: bool) -> Result<Self, DidacticError> {
        debug!("Reading config");
        let root_dir = RealPath(root_dir);
        let config_path = root_dir.join("didactic.toml");
        let config: Config = if config_path.is_file() {
            toml::from_str(&fs::read_to_string(config_path)?)?
        } else {
            return Err(DidacticError::MissingManifest);
        };
        let output_path = RealPath(root_dir.join(config.site.output_path.clone()));

        debug!("Initializing Tera");
        let tera = Tera::new(
            root_dir
                .join("templates/**/*.html")
                .to_str()
                .ok_or(DidacticError::InvalidUtf8Root)?
        )?;

        debug!("Initializing Typst engine");
        let engine = TypstEngine::new(Some(10), root_dir.clone());

        Ok(Self {
            root_dir,
            output_path,
            config,
            tera,
            engine,
            minify
        })
    }

    /// Orchestrates the build
    pub fn build(&mut self) -> Result<(), DidacticError> {
        info!("Building logical map");
        let file_map = self.map_files()?;
        debug!("{}", &file_map);

        fs::create_dir_all(&self.output_path)?;
        self.engine.set_file_map(file_map.clone());
        let mut state = BuildState {
            file_map,
            ..Default::default()
        };

        info!("Copying uncompiled asset files");
        self.copy_asset_files(&state)?;

        info!("Compiling SCSS");
        self.compile_styles(&state)?;

        info!("Compiling typst");
        self.compile_typst(&mut state)?;

        info!("Templating pages");
        self.template_pages(&state)?;

        Ok(())
    }

    fn map_files(&self) -> Result<FileMap, DidacticError> {
        let mut file_map = FileMap::new();
        file_map.add_directory(
            self.root_dir
                .join(self.config.site.root_content_path.clone()),
            None
        )?;
        file_map.add_directory(
            self.root_dir
                .join(self.config.site.static_content_path.clone()),
            None
        )?;
        for link in &self.config.links {
            file_map.add_directory(
                self.root_dir.join(&link.path),
                Some(&LogicalPath::new(&link.slug))
            )?;
        }
        Ok(file_map)
    }

    fn copy_asset_files(&self, state: &BuildState) -> Result<(), DidacticError> {
        for (logical, real) in state.file_map.filter_entries(|_, real| {
            !matches!(
                real.extension().and_then(|s| s.to_str()),
                Some("typ" | "toml" | "scss")
            )
        }) {
            let out_path = logical.with_output_path(&self.output_path);
            debug!(
                "Copying asset file: {} -> {}",
                real.display(),
                out_path.display()
            );
            out_path.make_parent()?;
            fs::copy(&real, &out_path)?;
        }
        Ok(())
    }

    fn compile_styles(&self, state: &BuildState) -> Result<(), DidacticError> {
        for (logical, real) in state
            .file_map
            .filter_entries(|_, real| real.extension().and_then(|s| s.to_str()) == Some("scss"))
        {
            let out_path = logical
                .with_extension("css")
                .with_output_path(&self.output_path);
            debug!(
                "Compiling SCSS: {} -> {}",
                real.display(),
                out_path.display()
            );
            let css = grass::from_path(real, &grass::Options::default())?;
            out_path.make_parent()?;
            fs::write(out_path, css)?;
        }
        Ok(())
    }

    fn compile_typst(&self, state: &mut BuildState) -> Result<(), DidacticError> {
        for (logical, real) in state
            .file_map
            .filter_entries(|_, real| real.extension().and_then(|s| s.to_str()) == Some("typ"))
        {
            debug!("Compiling typst: {}", real.display(),);
            let compilation_output = self.engine.compile_to_html(logical)?;
            for warning in compilation_output.warnings {
                warn!("Typst warning: {}", warning.message);
            }
            let compile_result = compilation_output
                .output
                .map_err(CompilationDiagnostics::from);
            match compile_result {
                Ok(doc) => {
                    state.document_cache.insert(logical.clone(), doc);
                }
                Err(e) => {
                    error!("{e}");
                }
            }
        }
        Ok(())
    }

    fn template_pages(&self, state: &BuildState) -> Result<(), DidacticError> {
        for (logical, doc) in &state.document_cache {
            let out_path = logical
                .with_extension("html")
                .with_output_path(&self.output_path);
            let meta = World::extract_meta(doc, logical)?;
            let body = World::extract_html_body(doc)?;
            let base = VirtualPath::new(logical.to_url_string())?;
            let link_resolver =
                LateLinkResolver::new(Some(&base), &**doc.introspector() as &dyn Introspector);
            let html_string = html_in_bundle(body, link_resolver.track())
                .map_err(CompilationDiagnostics::from)?;
            let mut context = Context::new();
            // context.insert("current_section", );
            // context.insert("menu", &page_metas);
            context.insert("content", &html_string);
            context.insert("site", &self.config.site);
            // TODO: template seleciton
            let rendered = self.tera.render("index.html", &context)?;
            out_path.make_parent()?;
            fs::write(out_path, rendered)?;
        }
        Ok(())
    }

    fn extract_meta(doc: &HtmlDocument, logical: &LogicalPath) -> Result<PageMeta, DidacticError> {
        let date_format = "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] [offset_hour sign:mandatory][offset_minute]";
        let description = time::format_description::parse(date_format).unwrap();
        let date = match doc.info().date {
            Smart::Custom(Some(typst::foundations::Datetime::Datetime(datetime))) => {
                datetime.assume_utc()
            }
            Smart::Custom(Some(typst::foundations::Datetime::Date(date))) => {
                let midnight = time::Time::from_hms(0, 0, 0).unwrap();
                time::PrimitiveDateTime::new(date, midnight).assume_utc()
            }
            Smart::Custom(Some(typst::foundations::Datetime::Time(time))) => {
                let today = time::OffsetDateTime::now_utc().date();
                time::PrimitiveDateTime::new(today, time).assume_utc()
            }
            _ => time::OffsetDateTime::now_utc()
        }
        .format(&description)?;

        Ok(PageMeta {
            title: doc
                .info()
                .title
                .as_ref()
                .ok_or(DidacticError::MissingTitle(logical.display().to_string()))?
                .to_string(),
            url: logical.with_extension("html").to_url_string(),
            date,
            author: doc.info().author.join(", ")
        })
    }

    fn extract_html_body(doc: &HtmlDocument) -> Result<&HtmlElement, DidacticError> {
        match doc.root().children.get(1) {
            None => Err(DidacticError::MissingBody),
            Some(HtmlNode::Element(elem)) => Ok(elem),
            Some(_) => Err(DidacticError::MissingBody)
        }
    }
}
