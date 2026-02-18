#set document(title: "Documentation", date: datetime(year: 2026, month: 2, day: 17))

= Documentation

== Project Structure

#raw(
  block: true,
  lang: "sh",
  "your-site/
├── didactic.toml
├── content/
│   └── index.typ
├── templates/
│   ├── index.html
│   └── main.scss
└── dist/",
)

== Config File

#raw(
  block: true,
  lang: "toml",
  "[site]
title = \"\"
author = \"\"
base_url = \"https://example.com\"",
)

= Typst

All content is written in Typst files placed in the `content/` directory. Each file becomes an HTML
page at the same relative path.

== Document Metadata

Every page needs to declare a title and optionally a date. Pages without a date are not included in
RSS.

#raw(
  block: true,
  lang: "typst",
  "#set document(title: \"\", date: datetime(year: 1970, month: 1, day: 1))",
)

== Math

Math has to be rendered using `html.frame`, which produces inline SVGs. This can be done
automatically by putting this at the top of your document:

#raw(block: true, lang: "typst", "#show math.equation: it => html.frame(it)")

Then write math as normal.

#raw(block: true, lang: "typst", "$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $")

Becomes:

#show math.equation: it => html.frame(it)
$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $

= Templates

Didactic uses #link("https://keats.github.io/tera/")[Tera] for templating.

== Template Variables
/ `site.title`: From `didactic.toml`
/ `site.author`: From `didactic.toml`
/ `site.base_url`: From `didactic.toml`
/ `menu`: List of pages for navigation. Each menu item exposes `title`, `url`, `section`, and
  `children`. Children are only used on index pages.
/ `content`: Rendered content
/ `current_section`: Current directory name
