#import "@local/akari-macros:0.1.0": *
#show: minimal-setup.with(
  title: "Documentation",
  date: datetime(year: 2026, month: 2, day: 17),
)

#title()

= Project Structure

```sh
your-site/
├── didactic.toml
├── content/
│   └── index.typ
├── templates/
│   ├── packages/
│   ├── index.html
│   └── main.scss
└── dist/
```

== Config File

Generally self explanatory. The links will be compiled in at the slug.

```toml
[site]
title = ""
author = ""
base_url = "https://example.com"
description = "optional field"

[[links]]
slug = "name"
path = "path/to/content/dir"
```

= Templates

Didactic uses #link("https://keats.github.io/tera/")[Tera] for templating.

== Template Variables
/ `site.title`: From `didactic.toml`
/ `site.author`: From `didactic.toml`
/ `site.base_url`: From `didactic.toml`
/ `site.description`: From `didactic.toml`
/ `menu`: List of pages for navigation. Each menu item exposes `title`, `url`, `section`, and
  `children`. Children are only used on index pages.
/ `content`: Rendered content
/ `current_section`: Current directory name

= Typst

All content is written in Typst files placed in the `content/` directory. Each file becomes an HTML
page at the same relative path.

== Document Metadata

Every page needs to declare a title and optionally a date. Pages without a date are not included in
RSS.

```typst
#set document(title: "", date: datetime(year: 1970, month: 1, day: 1))
```

== Packages

Didactic adds `templates/packages` to the file resolver and local packages should be placed there
for convenience. Unfortunately this utilizes typst's local resolver which expects the import
statement:
```typst
#import "@local/package-name:0.1.0": *
```
This searches the path:
```sh
templates/packages/local/package-name/0.1.0/
```
For a `typst.toml` file.

== Math

Math has to be rendered using `html.frame`, which produces inline SVGs. This can be done
automatically by putting this at the top of your document:

```typst
#show math.equation: it => if it.block {
  html.frame(it)
} else {
  box(html.frame(it))
}
```

Then write math as normal.
```typst
$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $
```
Becomes:

$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $

== Images

Typst will by default include images as a base64 blob inline. Didactic does not detect this
happening and will copy all assets to `dist` regardless. Images should be inserted using an `img`
block.

== Suggested Show Rules

```typst
#let compile-host = sys.inputs.at("compile-host", default: "cli")

#show math.equation: it => if compile-host == "didactic" {
  if it.block {
    html.frame(it)
  } else {
    box(html.frame(it))
  }
} else {
  it
}

#show line.where(length: 100%): it => if compile-host == "didactic" {
  html.elem("hr")
} else {
  it
}

#show align: it => if compile-host == "didactic" {
  html.elem("div", attrs: (style: "text-align: center;"), it.body)
} else {
  it
}

#show image: it => if compile-host == "didactic" {
  let px_width = if it.width != auto {
    str(it.width.pt()) + "px"
  } else {
    none
  }

  html.elem("img", attrs: (
    src: it.path,
    alt: it.alt,
    title: it.alt,
    width: px_width,
  ))
} else {
  it
}

```
Also of note, Didactic passes the input `compile-host` as `didactic` to the typst compiler. Typst's
builtin `#target()` requires context and ive found it to give unexpected results.
