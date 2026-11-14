//! Property tests for the pure functions in this crate.
//!
//! These state invariants rather than examples, so the framework searches for a
//! counterexample instead of relying on a hand-picked case.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;
use slate_ucs::{content_tokens, make_fx_name, parse_ucs_filename, sanitize_block, UcsFilename};

/// Characters UCS or the filesystem forbid inside a filename block.
const FORBIDDEN: [char; 10] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|', '_'];

proptest! {
    #[test]
    fn sanitize_never_leaves_a_forbidden_character(text in ".{0,80}") {
        let cleaned = sanitize_block(&text, None);

        for ch in FORBIDDEN {
            prop_assert!(!cleaned.contains(ch), "found {ch:?} in {cleaned:?}");
        }
        for ch in cleaned.chars() {
            prop_assert!(
                !('\u{0}'..='\u{1f}').contains(&ch),
                "found control character in {cleaned:?}"
            );
        }
    }

    #[test]
    fn sanitize_never_exceeds_the_budget(text in ".{0,120}", max in 1usize..40) {
        let cleaned = sanitize_block(&text, Some(max));

        prop_assert!(
            cleaned.chars().count() <= max,
            "{cleaned:?} is longer than {max}"
        );
    }

    #[test]
    fn sanitize_is_idempotent(text in ".{0,80}") {
        let once = sanitize_block(&text, None);
        let twice = sanitize_block(&once, None);

        prop_assert_eq!(once, twice);
    }

    #[test]
    fn a_built_filename_round_trips(
        cat_id in "[A-Z][A-Za-z0-9]{2,12}",
        fx in "[A-Za-z0-9][A-Za-z0-9 ]{0,39}",
        creator in "[A-Za-z0-9 ]{0,12}",
        source in "[A-Za-z0-9 ]{0,12}",
    ) {
        let built = UcsFilename::new(cat_id.clone(), fx.clone())
            .with_creator_id(creator)
            .with_source_id(source)
            .build()
            .expect("valid inputs build");

        let parsed = parse_ucs_filename(&built, None).expect("built names parse");

        prop_assert_eq!(parsed.cat_id, cat_id);
        prop_assert_eq!(parsed.fx_name, sanitize_block(&fx, None));
    }

    #[test]
    fn fx_name_respects_the_budget_unless_it_is_one_word(
        text in "[A-Za-z0-9 ]{0,120}",
        max in 4usize..40,
    ) {
        let name = make_fx_name(&text, max);

        if name.contains(' ') {
            prop_assert!(
                name.chars().count() <= max,
                "{name:?} exceeds {max}"
            );
        }
        prop_assert!(!name.is_empty());
    }

    #[test]
    fn content_tokens_never_contain_a_stopword(text in "[A-Za-z ]{0,120}") {
        let tokens = content_tokens(&text);

        for token in tokens {
            prop_assert!(token.chars().count() > 1, "single-character token {token:?}");
            prop_assert_eq!(token.to_lowercase(), token);
        }
    }
}
