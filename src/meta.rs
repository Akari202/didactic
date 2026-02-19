use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use log::{debug, warn};
use typst::foundations::{Dict, Str, Value};
use typst_as_lib::TypstEngine;
use typst_html::HtmlDocument;

use crate::file_map::FileMap;

#[derive(serde::Serialize, Debug)]
pub struct PageMeta {
    pub title: String,
    pub url: String,
    pub section: String,
    pub date: Option<String>,
    pub children: Vec<PageMeta>
}

pub fn collect_page_meta(
    prefix: &Path,
    file_map: &FileMap,
    engine: &TypstEngine,
    cache: &mut HashMap<PathBuf, HtmlDocument>,
    is_root: bool
) -> Result<Vec<PageMeta>, Box<dyn Error>> {
    let mut items = Vec::new();

    for dir in file_map.subdirs_at(prefix) {
        let index = dir.join("index.typ");
        if file_map.contains(&index) {
            let real = file_map.get_real(&index).unwrap();
            debug!("Compiling index path {}", real.display());
            let mut inputs = Dict::new();
            inputs.insert("target".into(), Value::Str(Str::from("html")));
            let doc: HtmlDocument = engine
                .compile_with_input(real.to_str().unwrap(), inputs)
                .output
                .map_err(|e| format!("Compile failed: {:?}", e))?;
            let stem = dir.file_stem().unwrap().to_string_lossy();
            let url = format!(
                "/{}",
                index
                    .parent()
                    .expect("Unable to get parent")
                    .to_str()
                    .unwrap()
                    .replace('\\', "/")
            );

            let children = collect_page_meta(&dir, file_map, engine, cache, false)?;

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
        } else {
            warn!(
                "Skipping directory {} as it has no index.typ",
                dir.display()
            );
        }
    }

    for logical in file_map.typ_files_at(prefix) {
        if !is_root && logical.file_stem().unwrap() == "index" {
            continue;
        }

        let real = file_map.get_real(logical).unwrap();
        debug!("Compiling path {}", real.display());
        let mut inputs = Dict::new();
        inputs.insert("target".into(), Value::Str(Str::from("html")));
        let doc: HtmlDocument = engine
            .compile_with_input(real.to_str().unwrap(), inputs)
            .output
            .map_err(|e| format!("{:?}", e))?;
        let url = format!(
            "/{}",
            logical
                .with_extension("html")
                .to_str()
                .unwrap()
                .replace('\\', "/")
        );
        let stem = logical.file_stem().unwrap().to_string_lossy();
        let title = extract_title_from_doc(&doc, &stem.to_uppercase());
        let date = extract_date(&doc);
        cache.insert(logical.clone(), doc);
        items.push(PageMeta {
            title,
            url,
            section: String::new(),
            date,
            children: vec![]
        });
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
