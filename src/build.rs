use std::collections::HashMap;
use std::error::Error;
use std::fs::{self};
use std::path::{Path, PathBuf};

use log::{info, warn};
use regex::{Captures, Regex};
use scraper::{Html, Selector};
use tera::{Context, Tera};
use typst_as_lib::TypstEngine;
use typst_html::HtmlDocument;
use xxhash_rust::xxh3::xxh3_64;

use crate::config::Config;
use crate::meta::{PageMeta, collect_page_meta};

pub fn run_build(dir: PathBuf) -> Result<(), Box<dyn Error>> {
    let content_path = dir.join("content");
    let output_path = dir.join("dist");
    fs::create_dir_all(&output_path)?;

    info!("Reading config");
    let config_path = dir.join("didactic.toml");
    let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;

    let scss_path = dir.join("templates/main.scss");
    if scss_path.exists() {
        info!("Compiling SCSS");
        let css = grass::from_path(scss_path, &grass::Options::default())?;
        fs::write(output_path.join("style.css"), css)?;
    } else {
        info!("No SCSS found, skipping");
    }

    info!("Copying static assets");
    let static_path = dir.join("static");
    if static_path.exists() {
        copy_assets(&static_path, &output_path)?;
    }
    copy_assets(&content_path, &output_path)?;
    let asset_hashes = collect_asset_hashes(&output_path, &output_path)?;

    info!("Initializing Tera");
    let tera = Tera::new(
        dir.join("templates/**/*.html")
            .to_str()
            .expect("Non UTF8 valid path????")
    )?;

    info!("Initializing Typst engine");
    let engine = TypstEngine::builder()
        .with_file_system_resolver(".")
        .fonts(typst_assets::fonts())
        .build();

    info!("Compiling content");
    let mut cache: HashMap<PathBuf, HtmlDocument> = HashMap::new();
    let page_metas = collect_page_meta(&content_path, &content_path, &engine, &mut cache, true)?;

    info!("Generating RSS feed");
    generate_rss(&page_metas, &config, &output_path)?;

    info!("Processing templates");
    let process_dirs = ProcessDirs {
        src: &content_path,
        out: &output_path,
        base: &content_path
    };

    process_typst_files(
        process_dirs,
        &tera,
        &page_metas,
        &config,
        &mut cache,
        &asset_hashes
    )?;

    info!("Build complete");
    Ok(())
}

struct ProcessDirs<'a> {
    src: &'a Path,
    out: &'a Path,
    base: &'a Path
}

fn process_typst_files(
    dirs: ProcessDirs,
    tera: &Tera,
    page_metas: &[PageMeta],
    config: &Config,
    cache: &mut HashMap<PathBuf, HtmlDocument>,
    asset_hashes: &HashMap<String, String>
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dirs.src)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dirs = ProcessDirs {
                src: &path,
                out: dirs.out,
                base: dirs.base
            };
            process_typst_files(dirs, tera, page_metas, config, cache, asset_hashes)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("typ") {
            info!("Rendering {}", path.display());

            let doc = cache
                .remove(&path)
                .ok_or_else(|| format!("no cached doc for {}", path.display()))?;

            let typst_html =
                extract_body_content(&typst_html::html(&doc).map_err(|e| format!("{:?}", e))?);

            let rel_path = path.strip_prefix(dirs.base)?;
            let mut out_path = dirs.out.join(rel_path);
            out_path.set_extension("html");
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let current_section = if rel_path.components().count() == 1 {
                String::new()
            } else {
                rel_path
                    .components()
                    .next()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .unwrap_or_default()
            };

            let mut context = Context::new();
            context.insert("asset_hashes", asset_hashes);
            context.insert("current_section", &current_section);
            context.insert("menu", &page_metas);
            context.insert("content", &typst_html);
            context.insert("site", &config.site);

            let rendered = bust_image_urls(&tera.render("index.html", &context)?, asset_hashes);
            fs::write(out_path, rendered)?;
        }
    }
    Ok(())
}

fn generate_rss(pages: &[PageMeta], config: &Config, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let base = config.site.base_url.trim_end_matches('/');

    for page in pages
        .iter()
        .chain(pages.iter().flat_map(|p| p.children.iter()))
    {
        if page.date.is_none() {
            warn!("Page {} has no date, excluded from RSS", page.url);
        }
    }

    let items: String = pages
        .iter()
        .filter(|p| p.date.is_some() && p.children.is_empty())
        .map(|p| {
            format!(
                r#"    <item>
      <title>{}</title>
      <link>{}{}</link>
      <guid>{}{}</guid>
      <pubDate>{}</pubDate>
    </item>"#,
                escape_xml(&p.title),
                base,
                p.url,
                base,
                p.url,
                p.date.as_ref().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let child_items: String = pages
        .iter()
        .flat_map(|p| p.children.iter())
        .filter(|p| p.date.is_some())
        .map(|p| {
            format!(
                r#"    <item>
      <title>{}</title>
      <link>{}{}</link>
      <guid>{}{}</guid>
      <pubDate>{}</pubDate>
    </item>"#,
                escape_xml(&p.title),
                base,
                p.url,
                base,
                p.url,
                p.date.as_ref().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>en-us</language>
{}
{}
  </channel>
</rss>"#,
        escape_xml(&config.site.title),
        base,
        escape_xml(&config.site.title),
        items,
        child_items,
    );

    fs::write(out_dir.join("rss.xml"), rss)?;
    Ok(())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn extract_body_content(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector = Selector::parse("body").unwrap();

    document
        .select(&selector)
        .next()
        .map(|body| body.inner_html())
        .unwrap_or_else(|| html.to_string())
}

fn hash_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let contents = fs::read(path)?;
    Ok(format!("{:x}", xxh3_64(&contents)))
}

fn collect_asset_hashes(
    dir: &Path,
    base: &Path
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut hashes = HashMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            hashes.extend(collect_asset_hashes(&path, base)?);
        } else {
            let ext = path.extension().and_then(|s| s.to_str());
            match ext {
                Some("html") | Some("typ") => {} // skip generated and source files
                _ => {
                    let hash = hash_file(&path)?;
                    let rel = path.strip_prefix(base)?;
                    let url = format!("/{}", rel.to_str().unwrap().replace('\\', "/"));
                    hashes.insert(url, hash);
                }
            }
        }
    }
    Ok(hashes)
}

fn bust_image_urls(html: &str, asset_hashes: &HashMap<String, String>) -> String {
    let re = Regex::new(r#"<img([^>]*?)src="(/[^"?]+)"([^>]*?)>"#).unwrap();
    re.replace_all(html, |caps: &Captures| {
        let before = &caps[1];
        let src = &caps[2];
        let after = &caps[3];
        if let Some(hash) = asset_hashes.get(src) {
            format!(r#"<img{}src="{}?v={}"{}>"#, before, src, hash, after)
        } else {
            caps[0].to_string()
        }
    })
    .to_string()
}

fn copy_assets(src: &Path, dst: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let dest_dir = dst.join(path.file_name().unwrap());
            fs::create_dir_all(&dest_dir)?;
            copy_assets(&path, &dest_dir)?;
        } else {
            let ext = path.extension().and_then(|s| s.to_str());
            match ext {
                Some("typ") | Some("toml") | Some("scss") => {}
                _ => {
                    let dest = dst.join(path.file_name().unwrap());
                    fs::copy(&path, &dest)?;
                }
            }
        }
    }
    Ok(())
}
