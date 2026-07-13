use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Options, Parser, Tag};
use url::Url;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub path: PathBuf,
    pub markdown: String,
    pub index: DocumentIndex,
    pub image_base_uri: String,
    pub intercepted_links: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub anchor: String,
}

#[derive(Debug, Clone)]
pub struct DocumentIndex {
    /// Markdown augmented in memory with stable heading identifiers. The source file is never
    /// changed, which also keeps hand-authored identifiers intact.
    pub render_markdown: String,
    pub headings: Vec<Heading>,
    sections: Vec<DocumentSection>,
}

#[derive(Debug, Clone)]
struct DocumentSection {
    title: String,
    anchor: Option<String>,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub section_title: String,
    pub anchor: Option<String>,
    pub snippet: String,
    pub matches: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchSummary {
    pub total_matches: usize,
    pub results: Vec<SearchResult>,
}

#[derive(Debug)]
pub enum DocumentError {
    UnsupportedExtension(PathBuf),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidUtf8(PathBuf),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedExtension(path) => {
                write!(f, "{} is not a .md or .markdown file.", path.display())
            }
            Self::Io { path, source } => {
                write!(f, "Could not open {}: {source}", path.display())
            }
            Self::InvalidUtf8(path) => write!(f, "{} is not valid UTF-8 Markdown.", path.display()),
        }
    }
}

impl std::error::Error for DocumentError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    Anchor,
    LocalMarkdown(PathBuf),
    External,
    Inactive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationEntry {
    pub path: PathBuf,
    pub scroll_offset: f32,
}

#[derive(Debug, Default)]
pub struct NavigationHistory {
    entries: Vec<NavigationEntry>,
}

impl NavigationHistory {
    pub fn push(&mut self, entry: NavigationEntry) {
        self.entries.push(entry);
    }

    pub fn pop(&mut self) -> Option<NavigationEntry> {
        self.entries.pop()
    }

    pub fn last(&self) -> Option<&NavigationEntry> {
        self.entries.last()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

pub fn decode_markdown(path: &Path, bytes: Vec<u8>) -> Result<String, DocumentError> {
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(&bytes);
    String::from_utf8(bytes.to_vec()).map_err(|_| DocumentError::InvalidUtf8(path.to_path_buf()))
}

pub fn load_document(path: impl AsRef<Path>) -> Result<LoadedDocument, DocumentError> {
    let requested_path = path.as_ref();
    if !is_markdown_path(requested_path) {
        return Err(DocumentError::UnsupportedExtension(
            requested_path.to_path_buf(),
        ));
    }

    let absolute_path =
        std::path::absolute(requested_path).map_err(|source| DocumentError::Io {
            path: requested_path.to_path_buf(),
            source,
        })?;
    let bytes = fs::read(&absolute_path).map_err(|source| DocumentError::Io {
        path: absolute_path.clone(),
        source,
    })?;
    let markdown = decode_markdown(&absolute_path, bytes)?;
    let index = build_document_index(&markdown);
    let image_base_uri = document_directory_uri(&absolute_path);
    let intercepted_links = collect_intercepted_links(&absolute_path, &index.render_markdown);

    Ok(LoadedDocument {
        path: absolute_path,
        markdown,
        index,
        image_base_uri,
        intercepted_links,
    })
}

/// Builds a navigable, in-memory heading index for ATX and Setext headings.
pub fn build_document_index(markdown: &str) -> DocumentIndex {
    let lines: Vec<&str> = markdown.split_inclusive('\n').collect();
    let mut rendered = String::with_capacity(markdown.len() + 64);
    let mut indexed_headings = Vec::new();
    let mut source_offset = 0usize;
    let mut generated_id = 0usize;
    let mut fenced_marker: Option<char> = None;
    let mut line_index = 0usize;

    while line_index < lines.len() {
        let line = lines[line_index];
        let (content, newline) = split_line_ending(line);

        if let Some(marker) = fenced_marker {
            rendered.push_str(line);
            if is_fence_closer(content, marker) {
                fenced_marker = None;
            }
            source_offset += line.len();
            line_index += 1;
            continue;
        }

        if let Some(marker) = fence_opener(content) {
            fenced_marker = Some(marker);
            rendered.push_str(line);
            source_offset += line.len();
            line_index += 1;
            continue;
        }

        if let Some((level, heading_content)) = parse_atx_heading(content) {
            let (title, explicit_id) = split_heading_id(heading_content);
            let has_explicit_id = explicit_id.is_some();
            let anchor = explicit_id.unwrap_or_else(|| next_generated_anchor(&mut generated_id));
            indexed_headings.push(IndexedHeading {
                heading: Heading {
                    level,
                    title: outline_title(title),
                    anchor: anchor.clone(),
                },
                source_offset,
            });
            if has_explicit_id {
                rendered.push_str(line);
            } else {
                rendered.push_str(&format!("{} {{#{anchor}}}{newline}", content.trim_end()));
            }
            source_offset += line.len();
            line_index += 1;
            continue;
        }

        if let Some(underline) = lines.get(line_index + 1)
            && let Some(level) = parse_setext_underline(split_line_ending(underline).0)
            && !content.trim().is_empty()
        {
            let (title, explicit_id) = split_heading_id(content.trim());
            let anchor = explicit_id.unwrap_or_else(|| next_generated_anchor(&mut generated_id));
            indexed_headings.push(IndexedHeading {
                heading: Heading {
                    level,
                    title: outline_title(title),
                    anchor: anchor.clone(),
                },
                source_offset,
            });
            let hashes = if level == 1 { "#" } else { "##" };
            rendered.push_str(&format!("{hashes} {title} {{#{anchor}}}{newline}"));
            source_offset += line.len() + underline.len();
            line_index += 2;
            continue;
        }

        rendered.push_str(line);
        source_offset += line.len();
        line_index += 1;
    }

    let headings = indexed_headings
        .iter()
        .map(|indexed| indexed.heading.clone())
        .collect();
    let sections = build_sections(markdown, &indexed_headings);
    DocumentIndex {
        render_markdown: rendered,
        headings,
        sections,
    }
}

/// Searches the original Markdown case-insensitively and returns at most one snippet per
/// matching section. The total includes every occurrence, even after the 100-result display cap.
pub fn search_sections(index: &DocumentIndex, query: &str) -> SearchSummary {
    let query = query.trim();
    if query.is_empty() {
        return SearchSummary::default();
    }

    let needle = query.to_lowercase();
    let mut summary = SearchSummary::default();

    for section in &index.sections {
        let mut section_matches = 0usize;
        let mut first_matching_line = None;
        for line in section.source.lines() {
            let count = count_case_insensitive(line, &needle);
            if count > 0 {
                section_matches += count;
                first_matching_line.get_or_insert_with(|| compact_snippet(line));
            }
        }

        summary.total_matches += section_matches;
        if section_matches > 0 && summary.results.len() < 100 {
            summary.results.push(SearchResult {
                section_title: section.title.clone(),
                anchor: section.anchor.clone(),
                snippet: first_matching_line.unwrap_or_default(),
                matches: section_matches,
            });
        }
    }

    summary
}

#[derive(Debug)]
struct IndexedHeading {
    heading: Heading,
    source_offset: usize,
}

fn build_sections(markdown: &str, headings: &[IndexedHeading]) -> Vec<DocumentSection> {
    if headings.is_empty() {
        return vec![DocumentSection {
            title: "Document".to_owned(),
            anchor: None,
            source: markdown.to_owned(),
        }];
    }

    headings
        .iter()
        .enumerate()
        .map(|(position, heading)| {
            let end = headings
                .get(position + 1)
                .map_or(markdown.len(), |next| next.source_offset);
            let start = if position == 0 {
                0
            } else {
                heading.source_offset
            };
            DocumentSection {
                title: heading.heading.title.clone(),
                anchor: Some(heading.heading.anchor.clone()),
                source: markdown[start..end].to_owned(),
            }
        })
        .collect()
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(content) = line.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "\n")
    } else {
        (line, "")
    }
}

fn fence_opener(line: &str) -> Option<char> {
    let trimmed = line.trim_start_matches(&[' ', '\t'][..]);
    let marker = trimmed.chars().next()?;
    if matches!(marker, '`' | '~')
        && trimmed
            .chars()
            .take_while(|character| *character == marker)
            .count()
            >= 3
    {
        Some(marker)
    } else {
        None
    }
}

fn is_fence_closer(line: &str, marker: char) -> bool {
    let trimmed = line.trim_start_matches(&[' ', '\t'][..]);
    trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count()
        >= 3
}

fn parse_atx_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let hash_count = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&hash_count) {
        return None;
    }
    let after_hashes = &trimmed[hash_count..];
    if after_hashes
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace())
    {
        return None;
    }
    let title = after_hashes.trim();
    let title = title.trim_end_matches('#').trim_end();
    Some((hash_count as u8, title))
}

fn parse_setext_underline(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '=' | '-')
        || trimmed.chars().count() < 3
        || !trimmed.chars().all(|character| character == marker)
    {
        return None;
    }
    Some(if marker == '=' { 1 } else { 2 })
}

fn split_heading_id(title: &str) -> (&str, Option<String>) {
    let trimmed = title.trim_end();
    let Some(open) = trimmed.rfind("{#") else {
        return (trimmed, None);
    };
    let Some(candidate) = trimmed[open..]
        .strip_prefix("{#")
        .and_then(|value| value.strip_suffix('}'))
    else {
        return (trimmed, None);
    };
    if candidate.is_empty() || candidate.chars().any(char::is_whitespace) {
        return (trimmed, None);
    }
    let before = &trimmed[..open];
    if before
        .chars()
        .next_back()
        .is_some_and(|character| !character.is_whitespace())
    {
        return (trimmed, None);
    }
    (before.trim_end(), Some(candidate.to_owned()))
}

fn next_generated_anchor(generated_id: &mut usize) -> String {
    *generated_id += 1;
    format!("md-reader-heading-{generated_id}")
}

fn outline_title(title: &str) -> String {
    let mut result = String::with_capacity(title.len());
    let mut in_link_destination = false;
    for character in title.chars() {
        match character {
            '*' | '_' | '`' | '~' => {}
            '(' if result.ends_with(']') => in_link_destination = true,
            ')' if in_link_destination => in_link_destination = false,
            _ if !in_link_destination => result.push(character),
            _ => {}
        }
    }
    result.replace(['[', ']'], "").trim().to_owned()
}

fn count_case_insensitive(haystack: &str, needle: &str) -> usize {
    let haystack = haystack.to_lowercase();
    let mut count = 0usize;
    let mut search_start = 0usize;
    while let Some(found) = haystack[search_start..].find(needle) {
        count += 1;
        search_start += found + needle.len();
    }
    count
}

fn compact_snippet(line: &str) -> String {
    let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_CHARS: usize = 120;
    let mut characters = compact.chars();
    let shortened: String = characters.by_ref().take(MAX_CHARS).collect();
    if characters.next().is_some() {
        format!("{shortened}…")
    } else {
        shortened
    }
}

pub fn document_directory_uri(document_path: &Path) -> String {
    document_path
        .parent()
        .and_then(|directory| Url::from_directory_path(directory).ok())
        .map(|url| url.to_string())
        .unwrap_or_else(|| "file:///".to_owned())
}

pub fn classify_link(document_path: &Path, destination: &str) -> LinkKind {
    let destination = destination.trim();
    if destination.is_empty() {
        return LinkKind::Inactive;
    }

    if destination.starts_with('#') {
        return LinkKind::Anchor;
    }

    let lowercase = destination.to_ascii_lowercase();
    if lowercase.starts_with("http://")
        || lowercase.starts_with("https://")
        || lowercase.starts_with("mailto:")
    {
        return LinkKind::External;
    }

    if destination.contains("://") || looks_like_uri_scheme(destination) {
        return LinkKind::Inactive;
    }

    let path_part = destination
        .split_once('#')
        .map_or(destination, |(path, _)| path)
        .split_once('?')
        .map_or_else(
            || {
                destination
                    .split_once('#')
                    .map_or(destination, |(path, _)| path)
            },
            |(path, _)| path,
        );
    let decoded = percent_decode_str(path_part).decode_utf8_lossy();
    let linked_path = PathBuf::from(decoded.as_ref());

    if !is_markdown_path(&linked_path) {
        return LinkKind::Inactive;
    }

    let resolved = if linked_path.is_absolute() {
        linked_path
    } else {
        document_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(linked_path)
    };

    LinkKind::LocalMarkdown(resolved)
}

pub fn collect_intercepted_links(document_path: &Path, markdown: &str) -> Vec<String> {
    let mut intercepted = BTreeSet::new();
    let parser = Parser::new_ext(markdown, Options::all());

    for event in parser {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            let destination = dest_url.to_string();
            if matches!(
                classify_link(document_path, &destination),
                LinkKind::LocalMarkdown(_) | LinkKind::Inactive
            ) {
                intercepted.insert(destination);
            }
        }
    }

    intercepted.into_iter().collect()
}

fn looks_like_uri_scheme(destination: &str) -> bool {
    let Some((scheme, _)) = destination.split_once(':') else {
        return false;
    };

    !scheme.is_empty()
        && scheme.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic()
            } else {
                character.is_ascii_alphanumeric()
                    || character == '+'
                    || character == '-'
                    || character == '.'
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_extensions_case_insensitively() {
        assert!(is_markdown_path(Path::new("notes.md")));
        assert!(is_markdown_path(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown_path(Path::new("notes.txt")));
    }

    #[test]
    fn decodes_utf8_and_strips_a_bom() {
        let path = Path::new("notes.md");
        let bytes = [UTF8_BOM, "# Heading".as_bytes()].concat();
        assert_eq!(decode_markdown(path, bytes).unwrap(), "# Heading");
    }

    #[test]
    fn rejects_invalid_utf8() {
        let result = decode_markdown(Path::new("notes.md"), vec![0xFF, 0xFE]);
        assert!(matches!(result, Err(DocumentError::InvalidUtf8(_))));
    }

    #[test]
    fn classifies_supported_links() {
        let document = Path::new("C:/docs/index.md");
        assert_eq!(classify_link(document, "#intro"), LinkKind::Anchor);
        assert_eq!(
            classify_link(document, "https://example.com"),
            LinkKind::External
        );
        assert_eq!(
            classify_link(document, "mailto:reader@example.com"),
            LinkKind::External
        );
        assert_eq!(
            classify_link(document, "chapter%20one.md#start"),
            LinkKind::LocalMarkdown(PathBuf::from("C:/docs/chapter one.md"))
        );
        assert_eq!(classify_link(document, "notes.txt"), LinkKind::Inactive);
        assert_eq!(
            classify_link(document, "javascript:alert(1)"),
            LinkKind::Inactive
        );
    }

    #[test]
    fn intercepts_local_and_unsupported_links_only() {
        let document = Path::new("C:/docs/index.md");
        let markdown =
            "[local](next.md) [web](https://example.com) [other](file.txt) [anchor](#top)";
        assert_eq!(
            collect_intercepted_links(document, markdown),
            vec!["file.txt".to_owned(), "next.md".to_owned()]
        );
    }

    #[test]
    fn navigation_history_is_last_in_first_out() {
        let mut history = NavigationHistory::default();
        history.push(NavigationEntry {
            path: PathBuf::from("first.md"),
            scroll_offset: 42.0,
        });
        history.push(NavigationEntry {
            path: PathBuf::from("second.md"),
            scroll_offset: 90.0,
        });

        assert_eq!(history.len(), 2);
        assert_eq!(history.pop().unwrap().path, PathBuf::from("second.md"));
        assert_eq!(history.pop().unwrap().scroll_offset, 42.0);
        assert!(history.is_empty());
    }

    #[test]
    fn directory_uri_ends_with_a_separator() {
        let uri = document_directory_uri(Path::new("C:/docs/index.md"));
        assert!(uri.starts_with("file:///"));
        assert!(uri.ends_with('/'));
    }

    #[test]
    fn loads_the_bundled_markdown_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/markdown-showcase.md");
        let document = load_document(path).expect("fixture should be readable");
        assert!(document.markdown.contains("# MD Reader Showcase"));
        assert!(
            document
                .intercepted_links
                .contains(&"next.md#start".to_owned())
        );
    }

    #[test]
    fn indexes_atx_and_setext_headings_and_preserves_explicit_ids() {
        let markdown =
            "# Top\n\nSetext section {#kept}\n===\n\n```md\n# Not a heading\n```\n\n### End\n";
        let index = build_document_index(markdown);

        assert_eq!(
            index.headings,
            vec![
                Heading {
                    level: 1,
                    title: "Top".to_owned(),
                    anchor: "md-reader-heading-1".to_owned(),
                },
                Heading {
                    level: 1,
                    title: "Setext section".to_owned(),
                    anchor: "kept".to_owned(),
                },
                Heading {
                    level: 3,
                    title: "End".to_owned(),
                    anchor: "md-reader-heading-2".to_owned(),
                },
            ]
        );
        assert!(
            index
                .render_markdown
                .contains("# Top {#md-reader-heading-1}")
        );
        assert!(index.render_markdown.contains("# Setext section {#kept}"));
        assert!(index.render_markdown.contains("# Not a heading"));
    }

    #[test]
    fn search_returns_section_snippets_and_full_match_count() {
        let index = build_document_index(
            "Prelude needle\n\n# One\nNeedle once. needle twice.\n\n## Two\nNo match\n\n# Three\nneedle here\n",
        );
        let results = search_sections(&index, "NEEDLE");

        assert_eq!(results.total_matches, 4);
        assert_eq!(results.results.len(), 2);
        assert_eq!(results.results[0].section_title, "One");
        assert_eq!(results.results[0].matches, 3);
        assert_eq!(results.results[1].section_title, "Three");
        assert!(results.results[0].anchor.is_some());
    }

    #[test]
    fn search_without_headings_has_snippets_but_no_navigation_anchor() {
        let index = build_document_index("A plain document has a Needle in it.");
        let results = search_sections(&index, "needle");

        assert_eq!(results.total_matches, 1);
        assert_eq!(results.results[0].section_title, "Document");
        assert_eq!(results.results[0].anchor, None);
        assert!(search_sections(&index, " ").results.is_empty());
    }
}
