#set document(title: "Documentation", date: datetime(year: 2026, month: 2, day: 17))

#title()

= Project Structure

```sh
your-site/
├── didactic.toml
├── content/
│   └── index.typ
├── templates/
│   ├── index.html
│   └── main.scss
└── dist/",
```

== Config File

Generally self explanatory. The links will be compiled in at the slug.

```toml
[site]
title = \"\"
author = \"\"
base_url = \"https://example.com\"
description = \"optional field\"

[[links]]
slug = \"name\"
path = \"path/to/content/dir\"
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
#set document(title: \"\", date: datetime(year: 1970, month: 1, day: 1))
```

== Math

Math has to be rendered using `html.frame`, which produces inline SVGs. This can be done
automatically by putting this at the top of your document:

```typst
#show math.equation: it => html.frame(it)
```

Then write math as normal.
```typst
$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $
```
Becomes:

#show math.equation: it => html.frame(it)
$ x = (-b plus.minus sqrt(b^2 - 4a c)) / (2a) $

== Images

Typst will by default include images as a base64 blob inline. Didactic does not detect this
happening and will copy all assets to `dist` regardless. Images can be inserted using an `img` block
like so:

```typst
#let _target = sys.inputs.at("target", default: "paged")

#show math.equation: it => if _target == "html" {
  html.frame(it)
} else {
  it
}

#let image(src, alt: "", width: auto, class: "") = {
  if _target == "html" {
    let attrs = if width != auto {
      let px_width = str(width.pt()) + "px"
      (
        src: src,
        alt: alt,
        title: alt,
        class: class,
        width: px_width,
      )
    } else {
      (
        src: src,
        alt: alt,
        title: alt,
        class: class,
      )
    }
    html.elem("img", attrs: attrs)
  } else {
    std.image(src, width: width, alt: alt)
  }
}
```
Also of note, Didactic passes the input `target` as `html` to the typst compiler. Typst's builtin
`#target()` requires context and ive found it to give unexpected results.
