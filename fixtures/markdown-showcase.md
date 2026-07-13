# MD Reader Showcase {#top}

This fixture verifies **bold text**, *emphasis*, ~~strikethrough~~, `inline code`, and a [same-document anchor](#tables).

## Lists

- A bullet item
  - A nested item
- [x] A completed task
- [ ] An incomplete task

1. First numbered item
2. Second numbered item

> A blockquote should be visually distinct without becoming decorative.

---

## Tables {#tables}

| Feature | Status | Notes |
|:--|:--:|--:|
| Headings | Ready | H1–H6 |
| Tables | Ready | Striped rows |
| Code | Ready | Plain monospace |

| Wide column one | Wide column two | Wide column three | Wide column four | Wide column five | Wide column six |
|---|---|---|---|---|---|
| This table intentionally tests narrow windows | Alpha | Bravo | Charlie | Delta | Echo |

## Code

```rust
fn main() {
    println!("boring, functional, and reliable");
}
```

## Links

- [External web link](https://example.com)
- [Inactive non-Markdown file](notes.txt)
- [Missing local Markdown file](next.md#start)

Footnotes are supported.[^1]

[^1]: This is a footnote rendered by the CommonMark viewer.

