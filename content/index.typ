#import "@local/akari-macros:0.1.0": *
#show: minimal-setup.with(
  title: "Didactic",
  date: datetime(year: 2026, month: 2, day: 17),
)

#title()

A simple static site generator that uses typst instead of markdown. Its how I made this site. There
are a ton of pain points currently.

== Quickstart

Install with cargo or download binary from releases:

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

== Commands
/ `didactic help`: Show help text
/ `didactic build`: Builds a site
  / `-m`: Minify the output
  / `-d`: The root directory to build [default: `./`]
/ `didactic clean`: Clean the build directory byt deleting the `dist` folder
  / `-d`: The root directory of the build to clean [default: `./`]

== Missing Stuff
- Typst html support is brand new and missing a ton of features, math is currently just an svg.
  - Also, your lsp is not going to like html specific typst because its currently feature gated.
- This crate is currently the minimum viable for my personal use. Feel free to open a pr or write an
  issue if you need a feature.
