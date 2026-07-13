mod document;
mod index;
mod links;
mod navigation;
mod words;

pub use document::{
    DocumentError, DocumentLoader, FileDocumentLoader, LoadedDocument, decode_markdown,
    document_directory_uri, is_markdown_path, load_document,
};
pub use index::{
    DocumentIndex, Heading, SearchResult, SearchSummary, build_document_index, search_sections,
};
pub use links::{LinkKind, classify_link, collect_intercepted_links};
pub use navigation::{NavigationEntry, NavigationHistory};
pub use words::{WordFrequency, WordHeatmap, build_word_heatmap};

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn accepts_supported_extensions_case_insensitively() {
        assert!(is_markdown_path(Path::new("notes.md")));
        assert!(is_markdown_path(Path::new("notes.MARKDOWN")));
        assert!(!is_markdown_path(Path::new("notes.txt")));
    }

    #[test]
    fn decodes_utf8_and_strips_a_bom() {
        let bytes = [[0xEF, 0xBB, 0xBF].as_slice(), "# Heading".as_bytes()].concat();
        assert_eq!(
            decode_markdown(Path::new("notes.md"), bytes).unwrap(),
            "# Heading"
        );
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

    #[test]
    fn word_heatmap_counts_visible_text_and_normalizes_case() {
        let heatmap = build_word_heatmap(
            "# Rust rust RUST\n\n[Reader](https://example.com/rust) uses `Rust`.\n\n42 42 C3PO",
        );

        assert_eq!(heatmap.total_words, 7);
        assert_eq!(heatmap.entries[0].word, "rust");
        assert_eq!(heatmap.entries[0].count, 4);
        assert_eq!(heatmap.entries[0].heat, 1.0);
        assert!(heatmap.entries.iter().any(|entry| entry.word == "c3po"));
        assert!(!heatmap.entries.iter().any(|entry| entry.word == "https"));
        assert!(!heatmap.entries.iter().any(|entry| entry.word == "42"));
    }

    #[test]
    fn word_heatmap_supports_unicode_and_log_scaled_intensity() {
        let heatmap = build_word_heatmap("Италия италия ИТАЛИЯ art Art design");
        let italy = heatmap
            .entries
            .iter()
            .find(|entry| entry.word == "италия")
            .expect("Unicode token should be present");
        let design = heatmap
            .entries
            .iter()
            .find(|entry| entry.word == "design")
            .expect("single token should be present");

        assert_eq!(italy.count, 3);
        assert_eq!(italy.heat, 1.0);
        assert!(design.heat > 0.0 && design.heat < italy.heat);
        assert_eq!(build_word_heatmap("").total_words, 0);
    }
}
