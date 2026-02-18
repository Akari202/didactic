use std::collections::HashMap;
use std::error::Error;
use std::fs::{self};
use std::path::{Path, PathBuf};

use typst_as_lib::TypstEngine;
use typst_html::HtmlDocument;

#[derive(serde::Serialize)]
pub struct PageMeta {
    pub title: String,
    pub url: String,
    pub section: String,
    pub date: Option<String>,
    pub children: Vec<PageMeta>
}

pub fn collect_page_meta(
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
    sort_meta(&mut items);
    Ok(items)
}

fn sort_meta(items: &mut [PageMeta]) {
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
