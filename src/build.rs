use std::collections::HashMap;
use std::error::Error;
use std::fs::{self};
use std::path::{Path, PathBuf};

use log::{debug, info, warn};
use regex::{Captures, Regex};
use scraper::{Html, Selector};
use tera::{Context, Tera};
use typst_as_lib::TypstEngine;
use typst_html::HtmlDocument;
use xxhash_rust::xxh3::xxh3_64;

use crate::config::Config;
use crate::file_map::FileMap;
use crate::meta::{PageMeta, collect_page_meta};

pub fn run_build(dir: PathBuf, minify: bool) -> Result<(), Box<dyn Error>> {
    let content_path = dir.join("content");
    let output_path = dir.join("dist");
    fs::create_dir_all(&output_path)?;

    info!("Reading config");
    let config_path = dir.join("didactic.toml");
    let config: Config = if config_path.exists() {
        Ok::<Config, Box<dyn Error>>(toml::from_str(&fs::read_to_string(config_path)?)?)
    } else {
        Err("No manifest file found".into())
    }?;

    info!("Building logical map");
    let mut file_map = FileMap::with_resolver_base(&dir);
    file_map.add_directory(&content_path, None)?;
    config
        .links
        .iter()
        .try_for_each(|i| file_map.add_directory(&dir.join(&i.path), Some(Path::new(&i.slug))))?;
    debug!("{:?}", &file_map);

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
    config.links.iter().try_for_each(|i| {
        let out = output_path.join(&i.slug);
        fs::create_dir_all(&out)?;
        copy_assets(&dir.join(&i.path), &out)
    })?;
    let asset_hashes = collect_asset_hashes(&output_path, &output_path)?;
    debug!("{:?}", &asset_hashes);

    info!("Initializing Tera");
    let tera = Tera::new(
        dir.join("templates/**/*.html")
            .to_str()
            .expect("Non UTF8 valid path????")
    )?;

    info!("Initializing Typst engine");
    let engine = TypstEngine::builder()
        .with_file_system_resolver(dir)
        .fonts(typst_assets::fonts())
        .build();

    info!("Compiling content");
    let mut cache: HashMap<PathBuf, HtmlDocument> = HashMap::new();
    let page_metas = collect_page_meta(Path::new(""), &file_map, &engine, &mut cache, true)?;
    debug!("{:?}", &page_metas);

    info!("Generating RSS feed");
    generate_rss(&page_metas, &config, &output_path)?;

    info!("Processing templates");
    process_typst_files(
        Path::new(""),
        &file_map,
        &output_path,
        &tera,
        &page_metas,
        &config,
        &mut cache,
        &asset_hashes,
        minify
    )?;

    info!("Build complete");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_typst_files(
    prefix: &Path,
    file_map: &FileMap,
    out_dir: &Path,
    tera: &Tera,
    page_metas: &[PageMeta],
    config: &Config,
    cache: &mut HashMap<PathBuf, HtmlDocument>,
    asset_hashes: &HashMap<String, String>,
    minify: bool
) -> Result<(), Box<dyn Error>> {
    for dir in file_map.subdirs_at(prefix) {
        process_typst_files(
            &dir,
            file_map,
            out_dir,
            tera,
            page_metas,
            config,
            cache,
            asset_hashes,
            minify
        )?;
    }

    for logical in file_map.typ_files_at(prefix) {
        info!("Rendering {}", logical.display());

        let doc = cache
            .remove(logical)
            .ok_or_else(|| format!("no cached doc for {}", logical.display()))?;

        let typst_html =
            extract_body_content(&typst_html::html(&doc).map_err(|e| format!("{:?}", e))?);

        let mut out_path = out_dir.join(logical);
        out_path.set_extension("html");
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let current_section = if logical.components().count() == 1 {
            String::new()
        } else {
            logical
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

        let rendered =
            bust_image_urls(&tera.render("index.html", &context)?, asset_hashes).into_bytes();

        debug!("Minifying");
        let minified = if minify {
            let cfg = minify_html::Cfg::new();
            minify_html::minify(&rendered, &cfg)
        } else {
            rendered
        };

        debug!("Writing file {}", out_dir.display());
        fs::write(out_path, minified)?;
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
                Some("html") | Some("typ") => {}
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
