# Didactic

A simple static site generator that uses typst instead of markdown.

## Quickstart

Install with cargo:

```sh
cargo install didactic
```

Build the site to `dist`:

```sh
didactic build
```

Serve it locally with:

```sh
python -m http.server --directory dist
```

## Missing Stuff
* Typst html support is brand new and missing a ton of features, math is currently just an svg.
  * Also, your lsp is not going to like html specific typst because its currently feature gated.
* This crate is currently the minimum viable for my personal use. Feel free to open a pr or write an
  issue if you need a feature.
