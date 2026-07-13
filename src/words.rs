use std::collections::BTreeMap;

use pulldown_cmark::{Event, Options, Parser};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WordHeatmap {
    pub total_words: usize,
    pub entries: Vec<WordFrequency>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordFrequency {
    pub word: String,
    pub count: usize,
    pub share: f32,
    /// Log-normalized intensity in the inclusive range 0..=1.
    pub heat: f32,
}

/// Counts visible Markdown words and maps their frequencies onto a logarithmic heat scale.
/// Markdown punctuation and link destinations are excluded because only text/code parser events
/// enter the token stream. Entries are sorted by descending count, then alphabetically.
pub fn build_word_heatmap(markdown: &str) -> WordHeatmap {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut total_words = 0usize;

    for event in Parser::new_ext(markdown, Options::all()) {
        if let Event::Text(text) | Event::Code(text) = event {
            collect_words(&text, &mut counts, &mut total_words);
        }
    }

    let maximum = counts.values().copied().max().unwrap_or_default() as f32;
    let heat_denominator = maximum.ln_1p();
    let mut entries = counts
        .into_iter()
        .map(|(word, count)| WordFrequency {
            word,
            count,
            share: count as f32 / total_words.max(1) as f32,
            heat: if heat_denominator > 0.0 {
                (count as f32).ln_1p() / heat_denominator
            } else {
                0.0
            },
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.word.cmp(&right.word))
    });

    WordHeatmap {
        total_words,
        entries,
    }
}

fn collect_words(text: &str, counts: &mut BTreeMap<String, usize>, total_words: &mut usize) {
    let mut token = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if character.is_alphanumeric() || matches!(character, '\'' | '’') {
            token.extend(character.to_lowercase());
            continue;
        }

        let normalized = token.trim_matches(['\'', '’']);
        if normalized.chars().any(char::is_alphabetic) {
            *counts.entry(normalized.to_owned()).or_default() += 1;
            *total_words += 1;
        }
        token.clear();
    }
}
