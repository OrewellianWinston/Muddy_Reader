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
