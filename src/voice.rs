//! My Voice: learn spelling fixes from the person's own edits, and summarise
//! how they dictate.
//!
//! The learner works on three strings only: the transcript Flowline pasted,
//! the text field right after the paste, and the same field a few seconds
//! later. It keeps the corrected word or phrase and nothing around it.
//! Everything is conservative on purpose, because a wrong rule would rewrite
//! every later dictation:
//!
//! * the edit must sit inside the pasted text and be small (a word or short
//!   phrase), so rewrites and additions are ignored;
//! * the fix must look like a recognition error (similar letters, or only a
//!   change of capitals), so a word swapped for a different word is ignored;
//! * very common words are never learned.

use crate::model::Dictation;

const STOPWORDS: &[&str] = &[
    "a", "about", "after", "again", "all", "also", "am", "an", "and", "any", "are", "as", "at",
    "be", "because", "been", "but", "by", "can", "could", "did", "do", "does", "for", "from",
    "get", "go", "had", "has", "have", "he", "her", "here", "him", "his", "how", "i", "i'm",
    "i'll", "i've", "if", "in", "into", "is", "it", "it's", "its", "just", "know", "like", "me",
    "more", "my", "no", "not", "now", "of", "on", "one", "or", "our", "out", "she", "so", "some",
    "than", "that", "the", "their", "them", "then", "there", "these", "they", "this", "to", "too",
    "up", "us", "very", "was", "we", "were", "what", "when", "which", "who", "will", "with",
    "would", "you", "your", "you're",
];

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’' || c == '-'
}

fn find_chars(hay: &[char], needle: &str) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() || n.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - n.len()).find(|&i| hay[i..i + n.len()] == n[..])
}

fn trim_punct(s: &str) -> &str {
    s.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Letters and digits only, lower-cased, for comparing how two spellings look.
fn skeleton(s: &str) -> Vec<char> {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Should "heard → corrected" become a dictionary rule?
fn accept(heard: &str, corrected: &str) -> Option<(String, String)> {
    let (h, c) = (heard.trim(), corrected.trim());
    let hl = h.chars().count();
    let cl = c.chars().count();
    if hl < 2 || cl < 2 || hl > 40 || cl > 40 || h == c {
        return None;
    }
    if !h.chars().any(|ch| ch.is_alphabetic()) || !c.chars().any(|ch| ch.is_alphabetic()) {
        return None;
    }
    if h.split_whitespace().count() == 1 && STOPWORDS.contains(&h.to_lowercase().as_str()) {
        return None;
    }
    let (a, b) = (skeleton(h), skeleton(c));
    if a.is_empty() || b.is_empty() {
        return None;
    }
    // A change of capitals or punctuation alone is always a fix. Otherwise the
    // two spellings must be close: this is what tells a misheard word from a
    // deliberate rewording.
    if a != b {
        // One spelling inside the other means letters or words were added or
        // dropped ("today" → "today please", "plan" → "plans"), not misheard.
        let (short, long) = if a.len() <= b.len() { (&a, &b) } else { (&b, &a) };
        if long.windows(short.len()).any(|w| w == short.as_slice()) {
            return None;
        }
        let dist = levenshtein(&a, &b) as f32;
        let longest = a.len().max(b.len()) as f32;
        if dist / longest > 0.6 {
            return None;
        }
    }
    Some((h.to_string(), c.to_string()))
}

fn pairs_from(old: &str, new: &str) -> Vec<(String, String)> {
    let ow: Vec<&str> = old.split_whitespace().map(trim_punct).filter(|w| !w.is_empty()).collect();
    let nw: Vec<&str> = new.split_whitespace().map(trim_punct).filter(|w| !w.is_empty()).collect();
    if ow.is_empty() || nw.is_empty() || ow.len() > 3 || nw.len() > 4 {
        return Vec::new();
    }
    if ow.len() == nw.len() && ow.len() > 1 {
        return ow
            .iter()
            .zip(nw.iter())
            .filter(|(o, n)| o != n)
            .filter_map(|(o, n)| accept(o, n))
            .take(3)
            .collect();
    }
    accept(&ow.join(" "), &nw.join(" ")).into_iter().collect()
}

/// The fixes the person made to `expected` (the text Flowline pasted), found
/// by comparing the field right after the paste (`baseline`) with the field
/// later (`current`).
pub fn learn_pairs(expected: &str, baseline: &str, current: &str) -> Vec<(String, String)> {
    let b: Vec<char> = baseline.chars().collect();
    let c: Vec<char> = current.chars().collect();
    let Some(start) = find_chars(&b, expected) else {
        return Vec::new();
    };
    let end = start + expected.chars().count();

    let mut p = 0;
    while p < b.len() && p < c.len() && b[p] == c[p] {
        p += 1;
    }
    let mut s = 0;
    while s < b.len() - p && s < c.len() - p && b[b.len() - 1 - s] == c[c.len() - 1 - s] {
        s += 1;
    }
    if p == b.len() && p == c.len() {
        return Vec::new(); // nothing changed
    }
    let (mut bo, mut co) = (p, p);
    let (mut be, mut ce) = (b.len() - s, c.len() - s);
    // Grow the changed span to whole words.
    while bo > 0 && co > 0 && is_word_char(b[bo - 1]) {
        bo -= 1;
        co -= 1;
    }
    while be < b.len() && ce < c.len() && is_word_char(b[be]) {
        be += 1;
        ce += 1;
    }
    // Only edits inside the pasted text count.
    if bo < start || be > end {
        return Vec::new();
    }
    let old: String = b[bo..be].iter().collect();
    let new: String = c[co..ce].iter().collect();
    if new.contains('\n') || old.contains('\n') {
        return Vec::new();
    }
    pairs_from(&old, &new)
}

#[derive(Default, Debug, PartialEq)]
pub struct VoiceStats {
    pub dictations: usize,
    pub words: usize,
    pub minutes: f32,
    pub wpm: f32,
    /// Words removed by cleanup (fillers, repeats) across the history.
    pub cleaned_words: usize,
    pub top_words: Vec<(String, usize)>,
}

pub fn stats(history: &[Dictation]) -> VoiceStats {
    let mut out = VoiceStats::default();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut seconds = 0.0f32;
    for d in history {
        out.dictations += 1;
        out.words += d.words;
        seconds += d.seconds;
        let raw = d.raw.split_whitespace().count();
        if raw > d.words {
            out.cleaned_words += raw - d.words;
        }
        for w in d.text.split_whitespace() {
            let w = trim_punct(w).to_lowercase();
            if w.chars().count() >= 4 && !STOPWORDS.contains(&w.as_str()) {
                *counts.entry(w).or_insert(0) += 1;
            }
        }
    }
    out.minutes = seconds / 60.0;
    out.wpm = if out.minutes > 0.05 { out.words as f32 / out.minutes } else { 0.0 };
    let mut top: Vec<(String, usize)> = counts.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(8);
    out.top_words = top;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(expected: &str, before: &str, after: &str) -> Vec<(String, String)> {
        learn_pairs(expected, before, after)
    }

    #[test]
    fn learns_a_misheard_word() {
        let text = "Please ask Sequoya about the budget.";
        let got = pairs(text, text, "Please ask Sequoia about the budget.");
        assert_eq!(got, vec![("Sequoya".to_string(), "Sequoia".to_string())]);
    }

    #[test]
    fn learns_a_capitalisation_fix() {
        let text = "we use figma for design";
        let got = pairs(text, text, "we use Figma for design");
        assert_eq!(got, vec![("figma".to_string(), "Figma".to_string())]);
    }

    #[test]
    fn learns_a_phrase_that_should_be_one_word() {
        let text = "deploy it to cooper netties today";
        let got = pairs(text, text, "deploy it to Kubernetes today");
        assert_eq!(got, vec![("cooper netties".to_string(), "Kubernetes".to_string())]);
    }

    #[test]
    fn ignores_a_reworded_sentence_or_a_different_word() {
        let text = "let us have a meeting on friday";
        assert!(pairs(text, text, "let us have a call on friday").is_empty());
        assert!(pairs(text, text, "can we talk friday").is_empty());
    }

    #[test]
    fn never_learns_common_words() {
        let text = "put it over their";
        assert!(pairs(text, text, "put it over there").is_empty());
    }

    #[test]
    fn ignores_edits_outside_the_pasted_text() {
        let pasted = "hello world";
        let before = "Dear Sam, hello world";
        // Fixing the greeting is not a correction of the transcript.
        assert!(pairs(pasted, before, "Dear Sam, hello world and more").is_empty());
        assert!(pairs(pasted, before, "Dear Sarn, hello world").is_empty());
    }

    #[test]
    fn ignores_pure_insertions_and_deletions() {
        let text = "send the report today";
        assert!(pairs(text, text, "send the report today please").is_empty());
        assert!(pairs(text, text, "send report today").is_empty());
    }

    #[test]
    fn plural_and_extension_edits_are_not_learned() {
        let text = "review the plan today";
        assert!(pairs(text, text, "review the plans today").is_empty());
        assert!(pairs(text, text, "review the planning today").is_empty());
    }

    #[test]
    fn no_change_learns_nothing() {
        let text = "all good here";
        assert!(pairs(text, text, text).is_empty());
        assert!(pairs("not present", text, text).is_empty());
    }

    #[test]
    fn stats_count_words_pace_and_top_words() {
        let mk = |text: &str, raw: &str, secs: f32| Dictation {
            text: text.into(),
            raw: raw.into(),
            app: String::new(),
            at: String::new(),
            ts: 0,
            seconds: secs,
            words: text.split_whitespace().count(),
            engine: String::new(),
        };
        let h = vec![
            mk("budget review budget", "um budget review uh budget", 6.0),
            mk("budget planning", "budget planning", 6.0),
        ];
        let s = stats(&h);
        assert_eq!(s.dictations, 2);
        assert_eq!(s.words, 5);
        assert_eq!(s.cleaned_words, 2);
        assert!((s.wpm - 25.0).abs() < 0.01);
        assert_eq!(s.top_words[0], ("budget".to_string(), 3));
    }
}
