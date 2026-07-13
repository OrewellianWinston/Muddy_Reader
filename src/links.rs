use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::is_markdown_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    Anchor,
    LocalMarkdown(PathBuf),
    External,
    Inactive,
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
