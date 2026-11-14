//! UCS filename construction and parsing.
//!
//! Structure, from the UCS Filenaming Convention:
//!
//! ```text
//! CatID(-UserCategory)_(VendorCategory-)FXName_CreatorID_SourceID(_UserData)
//! ```

use std::sync::OnceLock;

use regex::Regex;

use crate::error::UcsError;
use crate::taxonomy::Taxonomy;

/// Characters UCS or the filesystem forbid inside a filename block, plus control
/// characters and the `_` separator itself.
fn forbidden() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"[\\/:*?"<>|_\x00-\x1f]+"#)
            .unwrap_or_else(|e| panic!("invalid forbidden pattern: {e}"))
    })
}

fn whitespace() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\s+").unwrap_or_else(|e| panic!("invalid whitespace pattern: {e}"))
    })
}

/// Make a string safe for a UCS filename block.
///
/// Forbidden characters and `_` become spaces, runs of whitespace collapse to one,
/// and the result is trimmed. When `max_len` is given the value is cut at a word
/// boundary if one exists before the limit, otherwise hard-cut at the limit.
#[must_use]
pub fn sanitize_block(text: &str, max_len: Option<usize>) -> String {
    let replaced = forbidden().replace_all(text, " ");
    let collapsed = whitespace().replace_all(&replaced, " ");
    let trimmed = collapsed.trim();
    match max_len {
        Some(max) if trimmed.chars().count() > max => {
            let prefix: String = trimmed.chars().take(max).collect();
            match prefix.rsplit_once(' ') {
                Some((head, _)) if !head.is_empty() => head.to_string(),
                _ => prefix,
            }
        }
        _ => trimmed.to_string(),
    }
}

/// A UCS filename, either parsed from a string or built block by block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UcsFilename {
    /// The UCS CatID, for example `DOORCreak`.
    pub cat_id: String,
    /// The FX name block.
    pub fx_name: String,
    /// The creator block; rendered as `NONE` when empty.
    pub creator_id: String,
    /// The source block; rendered as `NONE` when empty.
    pub source_id: String,
    /// Optional user category appended to the CatID with a hyphen.
    pub user_category: Option<String>,
    /// Optional vendor category prefixed to the FX name with a hyphen.
    pub vendor_category: Option<String>,
    /// Optional trailing user data block.
    pub user_data: Option<String>,
    /// Optional extension, without the dot.
    pub extension: Option<String>,
}

impl UcsFilename {
    /// Start a filename from a CatID and an FX name.
    #[must_use]
    pub fn new(cat_id: impl Into<String>, fx_name: impl Into<String>) -> Self {
        Self {
            cat_id: cat_id.into(),
            fx_name: fx_name.into(),
            ..Self::default()
        }
    }

    /// Set the creator block.
    #[must_use]
    pub fn with_creator_id(mut self, creator_id: impl Into<String>) -> Self {
        self.creator_id = creator_id.into();
        self
    }

    /// Set the source block.
    #[must_use]
    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = source_id.into();
        self
    }

    /// Set the user category.
    #[must_use]
    pub fn with_user_category(mut self, user_category: impl Into<String>) -> Self {
        self.user_category = Some(user_category.into());
        self
    }

    /// Set the vendor category.
    #[must_use]
    pub fn with_vendor_category(mut self, vendor_category: impl Into<String>) -> Self {
        self.vendor_category = Some(vendor_category.into());
        self
    }

    /// Set the trailing user data block.
    #[must_use]
    pub fn with_user_data(mut self, user_data: impl Into<String>) -> Self {
        self.user_data = Some(user_data.into());
        self
    }

    /// Set the extension, without the dot.
    #[must_use]
    pub fn with_extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = Some(extension.into());
        self
    }

    /// Render the filename.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::EmptyFxName`] when the FX name is empty after sanitising,
    /// and [`UcsError::InvalidCatId`] when the CatID is empty.
    pub fn build(&self) -> Result<String, UcsError> {
        if self.cat_id.trim().is_empty() {
            return Err(UcsError::InvalidCatId(self.cat_id.clone()));
        }
        let fx = sanitize_block(&self.fx_name, None);
        if fx.is_empty() {
            return Err(UcsError::EmptyFxName);
        }

        let mut head = self.cat_id.trim().to_string();
        if let Some(user_category) = &self.user_category {
            let block = sanitize_block(user_category, None);
            if !block.is_empty() {
                head.push('-');
                head.push_str(&block);
            }
        }

        let fx_block = match &self.vendor_category {
            Some(vendor_category) => {
                let vendor = sanitize_block(vendor_category, None);
                if vendor.is_empty() {
                    fx.clone()
                } else {
                    format!("{vendor}-{fx}")
                }
            }
            None => fx,
        };

        let creator = or_none(&sanitize_block(&self.creator_id, None));
        let source = or_none(&sanitize_block(&self.source_id, None));

        let mut blocks = vec![head, fx_block, creator, source];
        if let Some(user_data) = &self.user_data {
            let block = sanitize_block(user_data, None);
            if !block.is_empty() {
                blocks.push(block);
            }
        }

        let name = blocks.join("_");
        Ok(match &self.extension {
            Some(extension) if !extension.is_empty() => format!("{name}.{extension}"),
            _ => name,
        })
    }
}

fn or_none(value: &str) -> String {
    if value.is_empty() {
        "NONE".to_string()
    } else {
        value.to_string()
    }
}

/// Split a trailing extension off a filename, if it looks like one.
fn split_extension(filename: &str) -> (&str, Option<String>) {
    match filename.rsplit_once('.') {
        Some((stem, ext))
            if (2..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            (stem, Some(ext.to_string()))
        }
        _ => (filename, None),
    }
}

/// Parse a UCS filename into its blocks.
///
/// # Errors
///
/// Returns [`UcsError::TooFewBlocks`] when fewer than four `_`-separated blocks are
/// present, and [`UcsError::UnknownCatId`] when a taxonomy is supplied and the CatID
/// is not in it.
pub fn parse_ucs_filename(
    filename: &str,
    taxonomy: Option<&Taxonomy>,
) -> Result<UcsFilename, UcsError> {
    let (stem, extension) = split_extension(filename);
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() < 4 {
        return Err(UcsError::TooFewBlocks(filename.to_string()));
    }

    let head = parts[0];
    let fx_block = parts[1];
    let creator = parts[2];
    let source = parts[3];
    let user_data = if parts.len() > 4 {
        let joined = parts[4..].join("_");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    } else {
        None
    };

    let (cat_id, user_category) = match head.split_once('-') {
        Some((id, user_cat)) => (id, Some(user_cat.to_string()).filter(|s| !s.is_empty())),
        None => (head, None),
    };
    let (vendor_category, fx_name) = match fx_block.split_once('-') {
        Some((vendor, fx)) => (Some(vendor.to_string()), fx.to_string()),
        None => (None, fx_block.to_string()),
    };

    let cat_id = match taxonomy {
        Some(tax) => tax.get(cat_id)?.cat_id.clone(),
        None => cat_id.to_string(),
    };

    Ok(UcsFilename {
        cat_id,
        fx_name,
        creator_id: creator.to_string(),
        source_id: source.to_string(),
        user_category,
        vendor_category,
        user_data,
        extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_forbidden_characters_with_spaces() {
        assert_eq!(
            sanitize_block("a/b:c*d?e\"f<g>h|i\\j", None),
            "a b c d e f g h i j"
        );
    }

    #[test]
    fn sanitize_replaces_underscores() {
        assert_eq!(sanitize_block("door_creak", None), "door creak");
    }

    #[test]
    fn sanitize_collapses_whitespace_and_trims() {
        assert_eq!(sanitize_block("  door   creak  ", None), "door creak");
    }

    #[test]
    fn sanitize_strips_control_characters() {
        assert_eq!(sanitize_block("door\u{0}\u{1f}creak", None), "door creak");
    }

    #[test]
    fn sanitize_cuts_at_a_word_boundary() {
        assert_eq!(
            sanitize_block("wooden door creak slow", Some(12)),
            "wooden door"
        );
    }

    #[test]
    fn sanitize_hard_cuts_when_no_boundary_exists() {
        assert_eq!(sanitize_block("abcdefghij", Some(4)), "abcd");
    }

    #[test]
    fn sanitize_keeps_short_values_intact() {
        assert_eq!(sanitize_block("door", Some(25)), "door");
    }

    #[test]
    fn builds_the_four_block_form() {
        let name = UcsFilename::new("DOORCreak", "Wooden Door Creak Slow")
            .with_creator_id("NONE")
            .with_source_id("ThickAsThieves")
            .build()
            .expect("built");

        assert_eq!(name, "DOORCreak_Wooden Door Creak Slow_NONE_ThickAsThieves");
    }

    #[test]
    fn empty_creator_and_source_render_as_none() {
        let name = UcsFilename::new("AIRBlow", "Steady Air")
            .build()
            .expect("built");

        assert_eq!(name, "AIRBlow_Steady Air_NONE_NONE");
    }

    #[test]
    fn appends_user_category_to_the_cat_id() {
        let name = UcsFilename::new("DOORCreak", "Creak")
            .with_user_category("MYCAT")
            .build()
            .expect("built");

        assert!(name.starts_with("DOORCreak-MYCAT_"), "got {name}");
    }

    #[test]
    fn prefixes_vendor_category_to_the_fx_name() {
        let name = UcsFilename::new("DOORCreak", "Creak")
            .with_vendor_category("VENDOR")
            .build()
            .expect("built");

        assert_eq!(name, "DOORCreak_VENDOR-Creak_NONE_NONE");
    }

    #[test]
    fn appends_user_data_and_extension() {
        let name = UcsFilename::new("DOORCreak", "Creak")
            .with_user_data("extra")
            .with_extension("wav")
            .build()
            .expect("built");

        assert_eq!(name, "DOORCreak_Creak_NONE_NONE_extra.wav");
    }

    #[test]
    fn rejects_an_empty_fx_name() {
        let err = UcsFilename::new("DOORCreak", "   ")
            .build()
            .expect_err("empty");

        assert_eq!(err, UcsError::EmptyFxName);
    }

    #[test]
    fn rejects_an_empty_cat_id() {
        let err = UcsFilename::new("", "Creak").build().expect_err("empty");

        assert!(matches!(err, UcsError::InvalidCatId(_)), "got {err:?}");
    }

    #[test]
    fn round_trips_a_built_name() {
        let built = UcsFilename::new("DOORCreak", "Wooden Door Creak Slow")
            .with_creator_id("JE")
            .with_source_id("ThickAsThieves")
            .with_user_data("take3")
            .with_extension("wav")
            .build()
            .expect("built");

        let parsed = parse_ucs_filename(&built, None).expect("parsed");

        assert_eq!(parsed.cat_id, "DOORCreak");
        assert_eq!(parsed.fx_name, "Wooden Door Creak Slow");
        assert_eq!(parsed.creator_id, "JE");
        assert_eq!(parsed.source_id, "ThickAsThieves");
        assert_eq!(parsed.user_data.as_deref(), Some("take3"));
        assert_eq!(parsed.extension.as_deref(), Some("wav"));
    }

    #[test]
    fn parses_user_and_vendor_categories() {
        let parsed =
            parse_ucs_filename("DOORCreak-MYCAT_VENDOR-Creak_JE_SRC", None).expect("parsed");

        assert_eq!(parsed.cat_id, "DOORCreak");
        assert_eq!(parsed.user_category.as_deref(), Some("MYCAT"));
        assert_eq!(parsed.vendor_category.as_deref(), Some("VENDOR"));
        assert_eq!(parsed.fx_name, "Creak");
    }

    #[test]
    fn parse_rejects_too_few_blocks() {
        let err = parse_ucs_filename("DOORCreak_Creak_JE", None).expect_err("too few");

        assert!(matches!(err, UcsError::TooFewBlocks(_)), "got {err:?}");
    }

    #[test]
    fn parse_canonicalises_the_cat_id_against_the_taxonomy() {
        let tax = Taxonomy::bundled().expect("bundled");

        let parsed = parse_ucs_filename("doorcreak_Creak_JE_SRC", Some(&tax)).expect("parsed");

        assert_eq!(parsed.cat_id, "DOORCreak");
    }

    #[test]
    fn parse_rejects_an_unknown_cat_id_when_a_taxonomy_is_given() {
        let tax = Taxonomy::bundled().expect("bundled");

        let err = parse_ucs_filename("NOPE_Creak_JE_SRC", Some(&tax)).expect_err("unknown");

        assert!(matches!(err, UcsError::UnknownCatId(_)), "got {err:?}");
    }

    #[test]
    fn parse_does_not_treat_a_short_suffix_as_an_extension() {
        let parsed = parse_ucs_filename("DOORCreak_Creak_JE_SRC_x", None).expect("parsed");

        assert!(parsed.extension.is_none());
        assert_eq!(parsed.user_data.as_deref(), Some("x"));
    }
}
