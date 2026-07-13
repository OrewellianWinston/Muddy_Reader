# MD Reader

MD Reader is a deliberately minimal native Windows Markdown viewer written in Rust. It opens one `.md` or `.markdown` file at a time and renders CommonMark plus tables, task lists, strikethrough, footnotes, links, and local raster images.

## Use

- Run `md-reader.exe`, then choose **Open…** or drop a Markdown file into the window.
- Run `md-reader.exe path\to\notes.md` to open a document directly.
- Press `Ctrl+O` to open another file and `Alt+Left` to follow the in-app Back history.

Relative Markdown links open in the same window. `http`, `https`, and `mailto` links use the Windows default application. Remote images are never fetched.

## Build and test

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The portable executable is generated at `target\release\md-reader.exe`.
