//! Small fuzzy matcher for launcher-style search.
//!
//! A query token matches a candidate when its characters appear in order
//! (a subsequence). Scoring rewards what people expect from a launcher:
//! matches at the start of the text, at word starts, and in contiguous runs,
//! while penalizing gaps. Text is lowercased per character so indices stay
//! aligned with word-boundary information.

/// A candidate string prepared once (at catalog load) for repeated matching.
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    chars: Vec<char>,
    /// `true` where a word starts (index 0, after a separator, or a
    /// lowercase→uppercase transition like "LibreOffice").
    word_start: Vec<bool>,
}

impl Prepared {
    pub fn new(text: &str) -> Self {
        let mut chars = Vec::with_capacity(text.len());
        let mut word_start = Vec::with_capacity(text.len());
        let mut prev: Option<char> = None;
        for c in text.chars() {
            let start = match prev {
                None => true,
                Some(p) => !p.is_alphanumeric() && c.is_alphanumeric() || p.is_lowercase() && c.is_uppercase(),
            };
            chars.push(fold(c));
            word_start.push(start);
            prev = Some(c);
        }
        Prepared { chars, word_start }
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
}

/// Lowercase a single character, keeping a 1:1 mapping with the input.
pub fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Prepare a query token (already split on whitespace).
pub fn fold_token(token: &str) -> Vec<char> {
    token.chars().map(fold).collect()
}

const MATCH: i32 = 16;
const WORD_START: i32 = 24;
const TEXT_START: i32 = 32;
const CONSECUTIVE: i32 = 20;
const GAP: i32 = 3;
const MAX_GAP_PENALTY: i32 = 30;
const LEADING: i32 = 1;
const MAX_LEADING_PENALTY: i32 = 12;
const WHOLE_WORD: i32 = 24;

/// Fuzzy (subsequence) score of `token` in `candidate`, or `None` if the
/// token is not a subsequence.
///
/// Tries every start position whose character matches the token's first
/// character and keeps the best greedy alignment. Candidates are short
/// (names, keywords), so O(n·m) is fine.
pub fn fuzzy(token: &[char], candidate: &Prepared) -> Option<i32> {
    let first = *token.first()?;
    let text = &candidate.chars;
    if token.len() > text.len() {
        return None;
    }
    (0..text.len()).filter(|&start| text[start] == first).filter_map(|start| align(token, candidate, start)).max()
}

fn align(token: &[char], candidate: &Prepared, start: usize) -> Option<i32> {
    let text = &candidate.chars;
    let mut score = -(start as i32 * LEADING).min(MAX_LEADING_PENALTY);
    let mut gap_penalty = 0;
    let mut prev: Option<usize> = None;
    let mut pos = start;
    for &tc in token {
        // Prefer a word-start occurrence ahead over the next plain one
        // when the current run is broken, so "lo" in "LibreOffice" aligns
        // to L…O(ffice) rather than l(ibre)…o.
        let next = (pos..text.len()).find(|&i| text[i] == tc)?;
        let chosen = if prev.is_some_and(|p| next != p + 1) && !candidate.word_start[next] {
            (next..text.len()).find(|&i| text[i] == tc && candidate.word_start[i]).unwrap_or(next)
        } else {
            next
        };
        score += MATCH;
        if candidate.word_start[chosen] {
            score += WORD_START;
        }
        if chosen == 0 {
            score += TEXT_START;
        }
        match prev {
            Some(p) if chosen == p + 1 => score += CONSECUTIVE,
            Some(p) => gap_penalty += (chosen - p - 1) as i32 * GAP,
            None => {}
        }
        prev = Some(chosen);
        pos = chosen + 1;
    }
    // Token covers a whole word (e.g. "code" in "Visual Studio Code").
    let end = pos;
    let contiguous = end - start == token.len();
    if contiguous && candidate.word_start[start] && (end == text.len() || !text[end].is_alphanumeric()) {
        score += WHOLE_WORD;
    }
    Some(score - gap_penalty.min(MAX_GAP_PENALTY))
}

/// Strict match: `token` must be a prefix of some word in `candidate`.
/// Used for long fields (descriptions), where subsequence matching would
/// match nearly anything.
pub fn word_prefix(token: &[char], candidate: &Prepared) -> Option<i32> {
    let text = &candidate.chars;
    let n = token.len();
    if n == 0 || n > text.len() {
        return None;
    }
    (0..=text.len() - n)
        .filter(|&i| candidate.word_start[i] && text[i..i + n] == *token)
        .map(|_| MATCH * n as i32 + WORD_START)
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(token: &str, text: &str) -> Option<i32> {
        fuzzy(&fold_token(token), &Prepared::new(text))
    }

    #[test]
    fn requires_subsequence() {
        assert!(s("frf", "Firefox").is_some());
        assert!(s("xf", "Firefox").is_none());
        assert!(s("firefoxx", "Firefox").is_none());
        assert!(s("", "Firefox").is_none());
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(s("FIRE", "firefox"), s("fire", "Firefox"));
    }

    #[test]
    fn prefix_beats_inner_match() {
        assert!(s("term", "Terminal") > s("term", "Xterm Emulator"));
    }

    #[test]
    fn word_start_beats_mid_word() {
        assert!(s("code", "Visual Studio Code") > s("code", "Barcode Scanner"));
    }

    #[test]
    fn contiguous_beats_scattered() {
        assert!(s("fire", "Firefox") > s("fire", "File Reader"));
    }

    #[test]
    fn camel_case_word_starts() {
        // "lo" aligns to L…O(ffice) (two word starts), beating a mid-word
        // contiguous match; a contiguous prefix still wins over both.
        assert!(s("lo", "LibreOffice") > s("lo", "Hello World"));
        assert!(s("lo", "Lorem") > s("lo", "LibreOffice"));
    }

    #[test]
    fn word_prefix_only_at_word_starts() {
        let d = Prepared::new("Browse the World Wide Web");
        assert!(word_prefix(&fold_token("wor"), &d).is_some());
        assert!(word_prefix(&fold_token("orld"), &d).is_none());
        assert!(word_prefix(&fold_token("bw"), &d).is_none());
    }

    #[test]
    fn unicode_is_safe() {
        assert!(s("é", "Éditeur de texte").is_some());
        // 'İ' lowercases to two chars; folding keeps the first, so indices
        // stay aligned and matching must not panic.
        assert_eq!(Prepared::new("İstanbul").chars.len(), "İstanbul".chars().count());
        assert!(s("stan", "İstanbul").is_some());
    }
}
