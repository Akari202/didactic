use std::collections::HashMap;
use std::error::Error;
use std::fs::{self};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info, warn};
use regex::{Captures, Regex};
use scraper::{Html, Selector};
use serde::Deserialize;
use tera::{Context, Tera};
use typst_as_lib::TypstEngine;
use typst_html::HtmlDocument;
use xxhash_rust::xxh3::xxh3_64;

#[derive(Parser)]
#[command(name = "didactic", about = "Simple typst SSG")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Build
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            if let Err(e) = run_build() {
                error!("Build failed: {}", e);
            }
        }
    }
}

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

#[derive(serde::Serialize)]
struct PageMeta {
    title: String,
    url: String,
    section: String,
    date: Option<String>,
    children: Vec<PageMeta>
}

fn collect_page_meta(
    src_dir: &Path,
    base_content: &Path,
    engine: &TypstEngine,
    cache: &mut HashMap<PathBuf, HtmlDocument>,
    is_root: bool
) -> Result<Vec<PageMeta>, Box<dyn Error>> {
    let mut items = Vec::new();
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let index = path.join("index.typ");
            if index.exists() {
                let doc: HtmlDocument = engine
                    .compile(index.to_str().unwrap())
                    .output
                    .map_err(|e| format!("{:?}", e))?;
                let stem = path.file_stem().unwrap().to_string_lossy();
                let rel = index.strip_prefix(base_content)?;
                let url = format!(
                    "/{}",
                    rel.with_extension("html")
                        .to_str()
                        .unwrap()
                        .replace('\\', "/")
                );

                let children = collect_page_meta(&path, base_content, engine, cache, false)?;

                let title = extract_title_from_doc(&doc, &stem.to_uppercase());
                let date = extract_date(&doc);
                cache.insert(index, doc);

                items.push(PageMeta {
                    title,
                    url,
                    section: stem.to_string(),
                    date,
                    children
                });
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("typ")
            && (is_root || path.file_stem().unwrap() != "index")
        {
            let doc: HtmlDocument = engine
                .compile(path.to_str().unwrap())
                .output
                .map_err(|e| format!("{:?}", e))?;
            let rel = path.strip_prefix(base_content)?;
            let url = format!(
                "/{}",
                rel.with_extension("html")
                    .to_str()
                    .unwrap()
                    .replace('\\', "/")
            );
            let stem = path.file_stem().unwrap().to_string_lossy();
            let title = extract_title_from_doc(&doc, &stem.to_uppercase());
            let date = extract_date(&doc);
            cache.insert(path, doc);
            items.push(PageMeta {
                title,
                url,
                section: String::new(),
                date,
                children: vec![]
            });
        }
    }
    Ok(items)
}

fn sort_meta(items: &mut Vec<PageMeta>) {
    items.sort_by(|a, b| match (a.url.as_str(), b.url.as_str()) {
        (u, _) if u.ends_with("/index.html") => std::cmp::Ordering::Less,
        (_, u) if u.ends_with("/index.html") => std::cmp::Ordering::Greater,
        _ => a.title.cmp(&b.title)
    });
    for item in items.iter_mut() {
        if !item.children.is_empty() {
            sort_meta(&mut item.children);
        }
    }
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

fn extract_date(doc: &HtmlDocument) -> Option<String> {
    doc.info.date.custom().flatten().map(|d| {
        let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
        ];
        let weekday = d.weekday().unwrap() as usize;
        format!(
            "{}, {:02} {} {} 00:00:00 +0000",
            days[weekday],
            d.day().unwrap(),
            months[d.month().unwrap() as usize - 1],
            d.year().unwrap()
        )
    })
}

fn extract_title_from_doc(doc: &HtmlDocument, default: &str) -> String {
    doc.info
        .title
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| default.to_string())
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

fn run_build() -> Result<(), Box<dyn Error>> {
    info!("Reading config");
    let config: Config = toml::from_str(&fs::read_to_string("didactic.toml")?)?;

    let scss_path = Path::new("templates/main.scss");
    let content_path = Path::new("content");
    fs::create_dir_all("dist")?;

    if scss_path.exists() {
        info!("Compiling SCSS");
        let css = grass::from_path(scss_path, &grass::Options::default())?;
        fs::write("dist/style.css", css)?;
    } else {
        info!("No SCSS found, skipping");
    }

    info!("Copying static assets");
    if Path::new("static").exists() {
        copy_assets(Path::new("static"), Path::new("dist"))?;
    }
    copy_assets(content_path, Path::new("dist"))?;
    let asset_hashes = collect_asset_hashes(Path::new("dist"), Path::new("dist"))?;

    info!("Initializing Tera");
    let tera = Tera::new("templates/**/*.html")?;

    let engine = TypstEngine::builder()
        .with_file_system_resolver(".")
        .fonts(typst_assets::fonts())
        .build();

    info!("Compiling content");
    let mut cache: HashMap<PathBuf, HtmlDocument> = HashMap::new();
    let mut page_metas = collect_page_meta(content_path, content_path, &engine, &mut cache, true)?;
    sort_meta(&mut page_metas);

    info!("Generating RSS feed");
    generate_rss(&page_metas, &config)?;

    process_typst_files(
        content_path,
        content_path,
        &tera,
        &page_metas,
        &config,
        &mut cache,
        &asset_hashes
    )?;

    info!("Build complete");
    Ok(())
}

fn process_typst_files(
    src_dir: &Path,
    base_content: &Path,
    tera: &Tera,
    page_metas: &[PageMeta],
    config: &Config,
    cache: &mut HashMap<PathBuf, HtmlDocument>,
    asset_hashes: &HashMap<String, String>
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            process_typst_files(
                &path,
                base_content,
                tera,
                page_metas,
                config,
                cache,
                asset_hashes
            )?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("typ") {
            info!("Rendering {}", path.display());

            let doc = cache
                .remove(&path)
                .ok_or_else(|| format!("no cached doc for {}", path.display()))?;

            let typst_html =
                extract_body_content(&typst_html::html(&doc).map_err(|e| format!("{:?}", e))?);

            let rel_path = path.strip_prefix(base_content)?;
            let mut out_path = PathBuf::from("dist").join(rel_path);
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

fn generate_rss(pages: &[PageMeta], config: &Config) -> Result<(), Box<dyn Error>> {
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

    fs::write("dist/rss.xml", rss)?;
    Ok(())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
