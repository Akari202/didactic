#set document(title: "Didactic", date: datetime(year: 2026, month: 2, day: 17))

= Didactic

A simple static site generator that uses typst instead of markdown.

== Quickstart

Install with cargo:

#raw(block: true, lang: "sh", "cargo install --git https://github.com/Akari202/didactic")

Build the site to `dist`:

#raw(block: true, lang: "sh", "didactic build")

Serve it locally with:

#raw(block: true, lang: "sh", "python -m http.server --directory dist")

== Missing Stuff
- Typst html support is brand new and missing a ton of features, math is currently just an svg.
  - Also, your lsp is not going to like html specific typst because its currently feature gated.
- This crate is currently the minimum viable for my personal use. Feel free to open a pr or write an
  issue if you need a feature.

