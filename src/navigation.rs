use std::path::PathBuf;

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
