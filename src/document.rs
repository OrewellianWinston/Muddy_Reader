use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use url::Url;

use crate::{
    DocumentIndex, WordHeatmap, build_document_index, build_word_heatmap, collect_intercepted_links,
};

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub path: PathBuf,
    pub markdown: String,
    pub index: DocumentIndex,
    pub word_heatmap: WordHeatmap,
    pub image_base_uri: String,
    pub intercepted_links: Vec<String>,
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

/// Narrow document source used by the application layer.
///
/// Keeping file access behind this boundary lets the UI depend on a capability rather than a
/// concrete filesystem implementation.
pub trait DocumentLoader {
    fn load(&self, path: &Path) -> Result<LoadedDocument, DocumentError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileDocumentLoader;

impl DocumentLoader for FileDocumentLoader {
    fn load(&self, requested_path: &Path) -> Result<LoadedDocument, DocumentError> {
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
        let word_heatmap = build_word_heatmap(&markdown);
        let image_base_uri = document_directory_uri(&absolute_path);
        let intercepted_links = collect_intercepted_links(&absolute_path, &index.render_markdown);

        Ok(LoadedDocument {
            path: absolute_path,
            markdown,
            index,
            word_heatmap,
            image_base_uri,
            intercepted_links,
        })
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
    FileDocumentLoader.load(path.as_ref())
}

pub fn document_directory_uri(document_path: &Path) -> String {
    document_path
        .parent()
        .and_then(|directory| Url::from_directory_path(directory).ok())
        .map(|url| url.to_string())
        .unwrap_or_else(|| "file:///".to_owned())
}
