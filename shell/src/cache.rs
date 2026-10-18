//! Small weighted LRU cache. Owners choose budgets; dropping the owner releases entries.
use std::collections::VecDeque;

pub(crate) struct Cache<K, V> {
    entries: VecDeque<(K, V, usize)>,
    bytes: usize,
    max_bytes: usize,
    max_entries: usize,
}
impl<K: PartialEq, V: Clone> Cache<K, V> {
    pub(crate) fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self { entries: VecDeque::new(), bytes: 0, max_bytes, max_entries }
    }
    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        let index = self.entries.iter().position(|entry| &entry.0 == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.1.clone();
        self.entries.push_back(entry);
        Some(value)
    }
    pub(crate) fn insert(&mut self, key: K, value: V, bytes: usize) {
        if let Some(index) = self.entries.iter().position(|entry| entry.0 == key) {
            self.bytes -= self.entries.remove(index).unwrap().2;
        }
        if bytes > self.max_bytes || self.max_entries == 0 {
            return;
        }
        while self.entries.len() >= self.max_entries || self.bytes > self.max_bytes - bytes {
            let Some(entry) = self.entries.pop_front() else { break };
            self.bytes -= entry.2;
        }
        self.bytes += bytes;
        self.entries.push_back((key, value, bytes));
    }
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evicts_by_bytes_and_recency_and_rejects_oversized_replacements() {
        let mut cache = Cache::new(3, 10);
        cache.insert("a", "first", 4);
        cache.insert("b", "second", 4);
        assert_eq!(cache.get(&"a"), Some("first"));
        cache.insert("c", "third", 4);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"a"), Some("first"));
        cache.insert("a", "too big", 11);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.bytes, 4);
        cache.insert("d", "fourth", 1);
        cache.insert("e", "fifth", 1);
        cache.insert("f", "sixth", 1);
        assert_eq!(cache.get(&"c"), None);
        cache.clear();
        assert_eq!(cache.bytes, 0);
        assert!(cache.entries.is_empty());
    }
}
