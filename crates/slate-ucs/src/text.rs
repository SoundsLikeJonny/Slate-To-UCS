//! Text utilities: description splitting, tokenising, FX names and abbreviations.
//!
//! A port of `ttucs.text`. Every function here is pure, so the property tests in
//! `tests/properties.rs` cover the invariants rather than examples.

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;

/// Words that carry no descriptive content and are dropped from FX names and
/// keywords.
const STOPWORDS: [&str; 80] = [
    "a",
    "an",
    "the",
    "and",
    "or",
    "of",
    "to",
    "in",
    "on",
    "at",
    "for",
    "with",
    "by",
    "from",
    "into",
    "onto",
    "over",
    "under",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "it",
    "its",
    "this",
    "that",
    "these",
    "those",
    "there",
    "here",
    "some",
    "any",
    "very",
    "really",
    "just",
    "then",
    "than",
    "as",
    "up",
    "down",
    "out",
    "off",
    "we",
    "i",
    "you",
    "he",
    "she",
    "they",
    "my",
    "our",
    "your",
    "his",
    "her",
    "their",
    "me",
    "us",
    "them",
    "so",
    "um",
    "uh",
    "like",
    "okay",
    "ok",
    "also",
    "plus",
    "recording",
    "recorded",
    "record",
    "take",
    "sound",
    "sounds",
    "noise",
    "being",
    "going",
    "gonna",
    "get",
    "getting",
    "got",
];

fn stopwords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| STOPWORDS.into_iter().collect())
}

fn keyword_tail() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:^|[.!?])\s*((?:[A-Z0-9][\w'&/-]*(?:\s[\w'&/-]+){0,3},\s*){2,}[A-Z0-9][\w'&/-]*(?:\s[\w'&/-]+){0,3}\.?)\s*$",
        )
        .unwrap_or_else(|e| panic!("invalid keyword-tail pattern: {e}"))
    })
}

fn token_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[a-z0-9][a-z0-9'&-]*").unwrap_or_else(|e| panic!("invalid token pattern: {e}"))
    })
}

fn word_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9]+").unwrap_or_else(|e| panic!("invalid word pattern: {e}"))
    })
}

fn take_suffix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?:\s+\d{1,3})+$").unwrap_or_else(|e| panic!("invalid take pattern: {e}"))
    })
}

/// Split a library description into its prose part and trailing keyword list.
///
/// Professional libraries often write `"<sentences>. Kw1, Kw2, Kw3."`.
///
/// ```
/// use slate_ucs::split_description;
///
/// let (prose, keywords) = split_description("Wooden door creaks open. Door, Wood, Creak, Open.");
/// assert_eq!(prose, "Wooden door creaks open.");
/// assert_eq!(keywords, vec!["Door", "Wood", "Creak", "Open"]);
/// ```
#[must_use]
pub fn split_description(description: &str) -> (String, Vec<String>) {
    let description = description.trim();
    let Some(captures) = keyword_tail().captures(description) else {
        return (description.to_string(), Vec::new());
    };
    let Some(list) = captures.get(1) else {
        return (description.to_string(), Vec::new());
    };

    let prose = description[..list.start()].trim();
    let keywords: Vec<String> = list
        .as_str()
        .split(',')
        .map(|k| k.trim_matches(|c: char| c == ' ' || c == '.').to_string())
        .filter(|k| !k.is_empty())
        .collect();

    if prose.is_empty() {
        return (description.to_string(), keywords);
    }
    (prose.to_string(), keywords)
}

/// Remove a trailing take number: `"Augusta A109 Fly Away 01"` becomes
/// `"Augusta A109 Fly Away"`.
#[must_use]
pub fn strip_take_numbers(fx_name: &str) -> String {
    take_suffix().replace(fx_name.trim(), "").trim().to_string()
}

/// Lower-cased content tokens: stopwords and single characters removed, order kept,
/// duplicates kept.
#[must_use]
pub fn content_tokens(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    token_pattern()
        .find_iter(&lower)
        .map(|m| m.as_str().to_string())
        .filter(|t| t.chars().count() > 1 && !stopwords().contains(t.as_str()))
        .collect()
}

/// Build a short, title-cased UCS FX name from a description.
///
/// UCS recommends about 25 characters. Content words are kept in order until the
/// budget is reached, always keeping at least one word. Duplicates are dropped. A
/// word of three characters or fewer that contains a digit is upper-cased, so model
/// numbers survive.
#[must_use]
pub fn make_fx_name(description: &str, max_chars: usize) -> String {
    let mut out: Vec<String> = Vec::new();
    for word in content_tokens(description) {
        if out.contains(&word) {
            continue;
        }
        let candidate = if out.is_empty() {
            word.clone()
        } else {
            format!("{} {word}", out.join(" "))
        };
        if !out.is_empty() && candidate.chars().count() > max_chars {
            break;
        }
        out.push(word);
    }

    if out.is_empty() {
        return "Untitled".to_string();
    }

    out.iter()
        .map(|w| {
            let short_with_digit = w.chars().count() <= 3 && w.chars().any(|c| c.is_ascii_digit());
            if short_with_digit {
                w.to_uppercase()
            } else {
                capitalize(w)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Unique, title-cased keywords from the description plus optional extra terms.
#[must_use]
pub fn make_keywords(description: &str, extra: &[String], limit: usize) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let extras = extra.iter().map(|e| e.to_lowercase());
    for word in content_tokens(description).into_iter().chain(extras) {
        if !seen.contains(&word) {
            seen.push(word);
        }
    }
    seen.into_iter()
        .take(limit)
        .map(|w| title_case(&w))
        .collect()
}

/// Initials of a name: `"Jon Evans"` becomes `"JE"`, at most six characters.
#[must_use]
pub fn initials(name: &str) -> String {
    word_pattern()
        .find_iter(name)
        .filter_map(|m| m.as_str().chars().next())
        .map(|c| c.to_ascii_uppercase())
        .take(6)
        .collect()
}

/// Short SourceID from a project name: `"Thick As Thieves"` becomes
/// `"ThickAsThieves"`. Falls back to initials when the result exceeds `max_len`.
#[must_use]
pub fn abbreviate(name: &str, max_len: usize) -> String {
    let joined: String = word_pattern()
        .find_iter(name)
        .map(|m| {
            let word = m.as_str();
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    if joined.chars().count() <= max_len {
        joined
    } else {
        initials(name)
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// Title-case a token the way Python's `str.title` does: every run of letters starts
/// with an upper-case letter and continues in lower case, so `t-shirt` becomes
/// `T-Shirt` and `don't` becomes `Don'T`.
fn title_case(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut previous_was_letter = false;
    for ch in word.chars() {
        if ch.is_alphabetic() {
            if previous_was_letter {
                out.extend(ch.to_lowercase());
            } else {
                out.extend(ch.to_uppercase());
            }
            previous_was_letter = true;
        } else {
            out.push(ch);
            previous_was_letter = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_prose_from_a_trailing_keyword_list() {
        let (prose, keywords) =
            split_description("Wooden door creaks open. Door, Wood, Creak, Open.");

        assert_eq!(prose, "Wooden door creaks open.");
        assert_eq!(keywords, vec!["Door", "Wood", "Creak", "Open"]);
    }

    #[test]
    fn returns_the_whole_description_when_there_is_no_keyword_list() {
        let (prose, keywords) = split_description("A wooden door creaks slowly");

        assert_eq!(prose, "A wooden door creaks slowly");
        assert!(keywords.is_empty());
    }

    #[test]
    fn keeps_a_keyword_only_description_intact() {
        let (prose, keywords) = split_description("Door, Wood, Creak");

        assert_eq!(prose, "Door, Wood, Creak");
        assert_eq!(keywords, vec!["Door", "Wood", "Creak"]);
    }

    #[test]
    fn strips_trailing_take_numbers() {
        assert_eq!(
            strip_take_numbers("Augusta A109 Fly Away 01"),
            "Augusta A109 Fly Away"
        );
        assert_eq!(strip_take_numbers("Door Creak 1 2 3"), "Door Creak");
    }

    #[test]
    fn leaves_a_name_without_take_numbers_alone() {
        assert_eq!(strip_take_numbers("Door Creak"), "Door Creak");
    }

    #[test]
    fn content_tokens_drop_stopwords_and_single_characters() {
        assert_eq!(
            content_tokens("A wooden door creaks open"),
            vec!["wooden", "door", "creaks", "open"]
        );
    }

    #[test]
    fn content_tokens_keep_digits_and_hyphens() {
        assert_eq!(content_tokens("MKH-416 boom 3"), vec!["mkh-416", "boom"]);
    }

    #[test]
    fn fx_name_is_title_cased_and_budgeted() {
        assert_eq!(
            make_fx_name("wooden door creak slow interior", 25),
            "Wooden Door Creak Slow"
        );
    }

    #[test]
    fn fx_name_keeps_at_least_one_word() {
        assert_eq!(
            make_fx_name("extraordinarily long single word", 5),
            "Extraordinarily"
        );
    }

    #[test]
    fn fx_name_upper_cases_short_tokens_with_digits() {
        assert_eq!(
            make_fx_name("augusta a109 fly away", 25),
            "Augusta A109 Fly Away"
        );
    }

    #[test]
    fn fx_name_falls_back_to_untitled() {
        assert_eq!(make_fx_name("the a of", 25), "Untitled");
        assert_eq!(make_fx_name("", 25), "Untitled");
    }

    #[test]
    fn fx_name_drops_duplicate_words() {
        assert_eq!(make_fx_name("door door creak", 25), "Door Creak");
    }

    #[test]
    fn keywords_are_unique_title_cased_and_limited() {
        let keywords = make_keywords("door door creak slow", &[], 15);

        assert_eq!(keywords, vec!["Door", "Creak", "Slow"]);
    }

    #[test]
    fn keywords_include_extra_terms_after_description_terms() {
        let keywords = make_keywords("door creak", &["Interior".to_string()], 15);

        assert_eq!(keywords, vec!["Door", "Creak", "Interior"]);
    }

    #[test]
    fn keywords_title_case_hyphenated_tokens_like_the_reference() {
        let keywords = make_keywords("tearing a cotton t-shirt apart", &[], 15);

        assert_eq!(keywords, vec!["Tearing", "Cotton", "T-Shirt", "Apart"]);
    }

    #[test]
    fn keywords_title_case_apostrophes_like_the_reference() {
        assert_eq!(make_keywords("don't stop", &[], 15), vec!["Don'T", "Stop"]);
    }

    #[test]
    fn content_tokens_drop_the_extended_stopword_list() {
        assert_eq!(
            content_tokens("recording of a door being opened while going out"),
            vec!["door", "opened", "while"]
        );
    }

    #[test]
    fn keywords_respect_the_limit() {
        assert_eq!(make_keywords("door creak slow heavy wood", &[], 2).len(), 2);
    }

    #[test]
    fn initials_take_the_first_letter_of_each_word() {
        assert_eq!(initials("Jon Evans"), "JE");
        assert_eq!(initials("thick as thieves"), "TAT");
    }

    #[test]
    fn initials_are_capped_at_six() {
        assert_eq!(initials("a b c d e f g h"), "ABCDEF");
    }

    #[test]
    fn initials_of_empty_input_are_empty() {
        assert_eq!(initials(""), "");
    }

    #[test]
    fn abbreviate_camel_cases_a_project_name() {
        assert_eq!(abbreviate("Thick As Thieves", 16), "ThickAsThieves");
    }

    #[test]
    fn abbreviate_falls_back_to_initials_when_too_long() {
        assert_eq!(abbreviate("Thick As Thieves", 5), "TAT");
    }

    #[test]
    fn abbreviate_keeps_the_inner_capitalisation_of_each_word() {
        assert_eq!(abbreviate("THICK AS", 16), "THICKAS");
        assert_eq!(abbreviate("thick as", 16), "ThickAs");
    }

    #[test]
    fn abbreviate_of_empty_input_is_empty() {
        assert_eq!(abbreviate("", 16), "");
    }
}
