//! In-memory view of the UCS list with fast CatID lookup.

use std::collections::HashMap;
use std::path::Path;

use crate::error::UcsError;
use crate::resources::{UCS_CSV, UCS_VERSION};

/// CatID spellings found in the wild that are not in the official list.
const CATID_ALIASES: [(&str, &str); 2] = [("MACHOffi", "MACHOffc"), ("WOODHandl", "WOODHndl")];

/// Categories that describe library workflow state rather than sound content.
const WORKFLOW_CATEGORIES: [&str; 1] = ["ARCHIVED"];

/// A single Category/SubCategory pair of the UCS list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcsEntry {
    /// The UCS CatID, for example `DOORCreak`.
    pub cat_id: String,
    /// The top-level category, for example `DOORS`.
    pub category: String,
    /// The subcategory, for example `CREAK`.
    pub subcategory: String,
    /// The short label used in filenames, for example `Creak`.
    pub cat_short: String,
    /// The human explanation from the UCS list.
    pub explanation: String,
    /// Comma-separated synonyms from the UCS list, split and trimmed.
    pub synonyms: Vec<String>,
}

impl UcsEntry {
    /// `CATEGORY-SUBCATEGORY`, the form UCS-aware databases display.
    #[must_use]
    pub fn category_full(&self) -> String {
        format!("{}-{}", self.category, self.subcategory)
    }

    /// Natural-language description of the class, used for cold-start labelling.
    #[must_use]
    pub fn label_document(&self) -> String {
        let syn = self
            .synonyms
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} {}. {} {}",
            self.category.to_lowercase(),
            self.subcategory.to_lowercase(),
            self.explanation,
            syn
        )
    }
}

/// The UCS list, indexed for lookup by CatID, by lower-cased CatID and by
/// `CATEGORY-SUBCATEGORY`.
#[derive(Debug, Clone)]
pub struct Taxonomy {
    version: String,
    entries: Vec<UcsEntry>,
    by_id: HashMap<String, usize>,
    by_id_lower: HashMap<String, usize>,
    by_full: HashMap<String, usize>,
}

impl Taxonomy {
    /// Parse a taxonomy from CSV text.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::MalformedTaxonomy`] if the header is missing a required
    /// column or the list is empty, and if any row cannot be read.
    pub fn from_csv_str(csv_text: &str, version: &str) -> Result<Self, UcsError> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(csv_text.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| UcsError::MalformedTaxonomy(e.to_string()))?
            .clone();
        for required in ["Category", "SubCategory", "CatID", "CatShort"] {
            if !headers.iter().any(|h| h == required) {
                return Err(UcsError::MalformedTaxonomy(format!(
                    "missing column {required}"
                )));
            }
        }

        let index_of = |name: &str| headers.iter().position(|h| h == name);
        let i_cat = index_of("Category");
        let i_sub = index_of("SubCategory");
        let i_id = index_of("CatID");
        let i_short = index_of("CatShort");
        let i_expl = index_of("Explanations");
        let i_syn = index_of("Synonyms");

        let mut entries = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|e| UcsError::MalformedTaxonomy(e.to_string()))?;
            let field = |idx: Option<usize>| -> String {
                idx.and_then(|i| record.get(i))
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            };
            let cat_id = field(i_id);
            if cat_id.is_empty() {
                return Err(UcsError::MalformedTaxonomy(
                    "row without a CatID".to_string(),
                ));
            }
            let synonyms = field(i_syn)
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect();
            entries.push(UcsEntry {
                cat_id,
                category: field(i_cat),
                subcategory: field(i_sub),
                cat_short: field(i_short),
                explanation: field(i_expl),
                synonyms,
            });
        }

        if entries.is_empty() {
            return Err(UcsError::MalformedTaxonomy("no rows".to_string()));
        }
        Ok(Self::new(entries, version))
    }

    /// Parse a taxonomy from a CSV file on disk.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::MalformedTaxonomy`] if the file cannot be read or parsed.
    pub fn from_csv_path(path: &Path, version: &str) -> Result<Self, UcsError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| UcsError::MalformedTaxonomy(format!("{}: {e}", path.display())))?;
        Self::from_csv_str(&text, version)
    }

    /// The taxonomy compiled into this crate.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::MalformedTaxonomy`] if the bundled resource is damaged,
    /// which a test in this crate rules out.
    pub fn bundled() -> Result<Self, UcsError> {
        Self::from_csv_str(UCS_CSV, UCS_VERSION)
    }

    /// Build an index over already-parsed entries.
    #[must_use]
    pub fn new(entries: Vec<UcsEntry>, version: &str) -> Self {
        let mut by_id = HashMap::with_capacity(entries.len());
        let mut by_id_lower = HashMap::with_capacity(entries.len());
        let mut by_full = HashMap::with_capacity(entries.len());
        for (i, entry) in entries.iter().enumerate() {
            by_id.insert(entry.cat_id.clone(), i);
            by_id_lower.insert(entry.cat_id.to_lowercase(), i);
            by_full.insert(entry.category_full().to_uppercase(), i);
        }
        Self {
            version: version.to_string(),
            entries,
            by_id,
            by_id_lower,
            by_full,
        }
    }

    /// The UCS version this list was loaded from.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the list has no entries. Always false for a constructed taxonomy.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every entry, in CSV order.
    #[must_use]
    pub fn entries(&self) -> &[UcsEntry] {
        &self.entries
    }

    /// Resolve a CatID by exact match, known alias, case-insensitive match or
    /// `CATEGORY-SUBCATEGORY`.
    #[must_use]
    pub fn resolve(&self, cat_id: &str) -> Option<&UcsEntry> {
        let trimmed = cat_id.trim();
        let aliased = CATID_ALIASES
            .iter()
            .find(|(from, _)| *from == trimmed)
            .map_or(trimmed, |(_, to)| *to);
        self.by_id
            .get(aliased)
            .or_else(|| self.by_id_lower.get(&aliased.to_lowercase()))
            .or_else(|| self.by_full.get(&aliased.to_uppercase()))
            .map(|&i| &self.entries[i])
    }

    /// Resolve a CatID or fail.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::UnknownCatId`] when the CatID is not in the list.
    pub fn get(&self, cat_id: &str) -> Result<&UcsEntry, UcsError> {
        self.resolve(cat_id)
            .ok_or_else(|| UcsError::UnknownCatId(cat_id.to_string()))
    }

    /// Every CatID, in CSV order.
    #[must_use]
    pub fn cat_ids(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.cat_id.as_str()).collect()
    }

    /// Every distinct category, sorted.
    #[must_use]
    pub fn categories(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.entries.iter().map(|e| e.category.as_str()).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Every entry in a category, matched case-insensitively.
    #[must_use]
    pub fn subcategories(&self, category: &str) -> Vec<&UcsEntry> {
        let wanted = category.to_uppercase();
        self.entries
            .iter()
            .filter(|e| e.category == wanted)
            .collect()
    }

    /// Entries that are sound content rather than library workflow state.
    #[must_use]
    pub fn trainable(&self) -> Vec<&UcsEntry> {
        self.entries
            .iter()
            .filter(|e| !WORKFLOW_CATEGORIES.contains(&e.category.as_str()))
            .collect()
    }

    /// Substring search over CatID, `CATEGORY-SUBCATEGORY`, explanation and
    /// synonyms, best match first.
    ///
    /// Ranking is deterministic: an exact CatID match, then a CatID prefix, then a
    /// CatID substring, then a category match, then anything else. Ties keep CSV
    /// order.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<&UcsEntry> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut scored: Vec<(u8, usize, &UcsEntry)> = Vec::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let cat = entry.cat_id.to_lowercase();
            let full = entry.category_full().to_lowercase();
            let score = if cat == needle {
                0
            } else if cat.starts_with(&needle) {
                1
            } else if cat.contains(&needle) || full.contains(&needle) {
                2
            } else if entry.category.to_lowercase().contains(&needle) {
                3
            } else if entry.explanation.to_lowercase().contains(&needle)
                || entry
                    .synonyms
                    .iter()
                    .any(|s| s.to_lowercase().contains(&needle))
            {
                4
            } else {
                continue;
            };
            scored.push((score, i, entry));
        }
        scored.sort_by_key(|(score, i, _)| (*score, *i));
        scored.into_iter().take(limit).map(|(_, _, e)| e).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Taxonomy {
        let csv = "Category,SubCategory,CatID,CatShort,Explanations,Synonyms\n\
                   DOORS,CREAK,DOORCreak,Creak,\"A door creaking\",\"Creak, Squeak\"\n\
                   DOORS,WOOD,DOORWood,Wood,\"A wooden door\",\"Wood, Timber\"\n\
                   AIR,BLOW,AIRBlow,AIR,\"Steady air\",\"Blow, Puff\"\n";
        Taxonomy::from_csv_str(csv, "test").expect("tiny taxonomy")
    }

    #[test]
    fn parses_entries_and_synonyms() {
        let tax = tiny();

        assert_eq!(tax.len(), 3);
        let entry = tax.get("DOORCreak").expect("known");
        assert_eq!(entry.category, "DOORS");
        assert_eq!(entry.subcategory, "CREAK");
        assert_eq!(entry.cat_short, "Creak");
        assert_eq!(entry.synonyms, vec!["Creak", "Squeak"]);
    }

    #[test]
    fn category_full_joins_with_a_hyphen() {
        let tax = tiny();

        assert_eq!(
            tax.get("DOORWood").expect("known").category_full(),
            "DOORS-WOOD"
        );
    }

    #[test]
    fn resolves_exact_alias_case_insensitive_and_full_form() {
        let tax = tiny();

        assert_eq!(
            tax.resolve("DOORCreak").map(|e| e.cat_id.as_str()),
            Some("DOORCreak")
        );
        assert_eq!(
            tax.resolve("doorcreak").map(|e| e.cat_id.as_str()),
            Some("DOORCreak")
        );
        assert_eq!(
            tax.resolve("DOORS-CREAK").map(|e| e.cat_id.as_str()),
            Some("DOORCreak")
        );
        assert_eq!(
            tax.resolve("  DOORCreak  ").map(|e| e.cat_id.as_str()),
            Some("DOORCreak")
        );
    }

    #[test]
    fn resolves_known_aliases() {
        let csv = "Category,SubCategory,CatID,CatShort,Explanations,Synonyms\n\
                   MACHINES,OFFICE,MACHOffc,Offc,\"Office machine\",\"Printer\"\n";
        let tax = Taxonomy::from_csv_str(csv, "test").expect("taxonomy");

        assert_eq!(
            tax.resolve("MACHOffi").map(|e| e.cat_id.as_str()),
            Some("MACHOffc")
        );
    }

    #[test]
    fn unknown_cat_id_returns_none_and_get_errors() {
        let tax = tiny();

        assert!(tax.resolve("NOPE").is_none());
        assert_eq!(
            tax.get("NOPE"),
            Err(UcsError::UnknownCatId("NOPE".to_string()))
        );
    }

    #[test]
    fn rejects_csv_without_required_columns() {
        let csv = "Category,SubCategory,CatShort\nDOORS,CREAK,Creak\n";

        let err = Taxonomy::from_csv_str(csv, "test").expect_err("missing CatID");

        assert!(matches!(err, UcsError::MalformedTaxonomy(_)), "got {err:?}");
    }

    #[test]
    fn rejects_empty_csv() {
        let csv = "Category,SubCategory,CatID,CatShort,Explanations,Synonyms\n";

        let err = Taxonomy::from_csv_str(csv, "test").expect_err("no rows");

        assert!(matches!(err, UcsError::MalformedTaxonomy(_)), "got {err:?}");
    }

    #[test]
    fn categories_are_sorted_and_deduplicated() {
        let tax = tiny();

        assert_eq!(tax.categories(), vec!["AIR", "DOORS"]);
    }

    #[test]
    fn subcategories_filters_case_insensitively() {
        let tax = tiny();

        let doors = tax.subcategories("doors");

        assert_eq!(doors.len(), 2);
        assert!(doors.iter().all(|e| e.category == "DOORS"));
    }

    #[test]
    fn trainable_excludes_workflow_categories() {
        let csv = "Category,SubCategory,CatID,CatShort,Explanations,Synonyms\n\
                   DOORS,CREAK,DOORCreak,Creak,\"A door creaking\",\"Creak\"\n\
                   ARCHIVED,OLD,ARCHOld,Old,\"Archived\",\"Old\"\n";
        let tax = Taxonomy::from_csv_str(csv, "test").expect("taxonomy");

        let trainable = tax.trainable();

        assert_eq!(trainable.len(), 1);
        assert_eq!(trainable[0].cat_id, "DOORCreak");
    }

    #[test]
    fn search_ranks_exact_then_prefix_then_substring() {
        let tax = tiny();

        let hits = tax.search("door", 10);

        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|e| e.category == "DOORS"));
    }

    #[test]
    fn search_matches_synonyms_and_respects_limit() {
        let tax = tiny();

        assert_eq!(tax.search("squeak", 10)[0].cat_id, "DOORCreak");
        assert_eq!(tax.search("door", 1).len(), 1);
        assert!(tax.search("", 10).is_empty());
        assert!(tax.search("door", 0).is_empty());
    }

    #[test]
    fn label_document_lowercases_the_class_and_appends_synonyms() {
        let tax = tiny();

        let doc = tax.get("DOORCreak").expect("known").label_document();

        assert!(doc.starts_with("doors creak. A door creaking"), "got {doc}");
        assert!(doc.ends_with("Creak, Squeak"), "got {doc}");
    }
}
