//! The wire types of the `ttucs serve` contract.
//!
//! These mirror `docs/serve-api.md`. Unknown JSON keys are ignored rather than
//! rejected, so a newer engine can add a field without breaking an older client.

use serde::{Deserialize, Serialize};

/// One of the three decision states the engine reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionKind {
    /// The CatID head was confident.
    Confident,
    /// The CatID was unsure but the category was clear.
    CategoryFallback,
    /// Neither head cleared its threshold.
    Unknown,
    /// A value this client does not recognise.
    Unrecognised,
}

impl DecisionKind {
    /// Classify the engine's `decision` string.
    #[must_use]
    pub fn from_wire(value: &str) -> Self {
        match value {
            "confident" => Self::Confident,
            "category_fallback" => Self::CategoryFallback,
            "unknown" => Self::Unknown,
            _ => Self::Unrecognised,
        }
    }
}

/// Model identity reported by `/health`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ModelInfo {
    /// Model family, for example `bow`.
    pub name: String,
    /// Weights version.
    pub version: String,
    /// Where the weights came from: `path`, `package`, `cache` or `release`.
    #[serde(default)]
    pub source: String,
    /// Whether the weights manifest verified.
    #[serde(default)]
    pub manifest_verified: bool,
    /// Verification failure, when there was one.
    #[serde(default)]
    pub manifest_error: Option<String>,
    /// Filesystem path of the weights, when known.
    #[serde(default)]
    pub path: Option<String>,
}

/// Request caps the engine enforces.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Limits {
    /// Maximum texts per request.
    pub max_texts: usize,
    /// Maximum body size in bytes.
    pub max_body_bytes: usize,
    /// Maximum `top_k`.
    pub max_top_k: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_texts: crate::MAX_TEXTS,
            max_body_bytes: crate::MAX_BODY_BYTES,
            max_top_k: 50,
        }
    }
}

/// The `/health` response.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Health {
    /// Contract version.
    pub schema_version: u32,
    /// `ok` when the server is usable.
    pub status: String,
    /// Whether the model has been warmed.
    pub warm: bool,
    /// Version of the `ttucs` package serving.
    pub ttucs_version: String,
    /// UCS list version.
    pub ucs_version: String,
    /// Model identity.
    pub model: ModelInfo,
    /// Whether the engine is in offline mode.
    pub offline: bool,
    /// Compute device, for example `cpu`.
    #[serde(default)]
    pub device: String,
    /// Threshold below which the CatID head is treated as unknown.
    pub unknown_threshold: f64,
    /// Threshold below which the category head is treated as unknown.
    pub category_threshold: f64,
    /// Default number of alternatives.
    #[serde(default)]
    pub top_k: u32,
    /// Request caps.
    #[serde(default)]
    pub limits: Limits,
}

/// One alternative CatID in a prediction.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Alternative {
    /// CatID.
    pub cat_id: String,
    /// Category.
    pub category: String,
    /// Subcategory.
    pub subcategory: String,
    /// Short label.
    pub cat_short: String,
    /// Calibrated confidence.
    pub confidence: f64,
}

/// One category score in a prediction.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CategoryScore {
    /// Category name.
    pub category: String,
    /// Aggregated confidence.
    pub confidence: f64,
}

/// Fields parsed out of a slate.
///
/// The engine returns every `SlateFields` field; this client keeps the ones the UI
/// shows and preserves the rest in `extra` so nothing is silently dropped.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct SlateFields {
    /// The raw transcript.
    #[serde(default)]
    pub raw_text: String,
    /// The description the classifier saw.
    #[serde(default)]
    pub description: String,
    /// Project name.
    #[serde(default)]
    pub project: Option<String>,
    /// Recording location.
    #[serde(default)]
    pub location: Option<String>,
    /// Microphones named in the slate.
    #[serde(default)]
    pub microphones: Vec<String>,
    /// Microphone types.
    #[serde(default)]
    pub mic_types: Vec<String>,
    /// Microphone setup, for example `interior`.
    #[serde(default)]
    pub mic_setup: Vec<String>,
    /// Recorder.
    #[serde(default)]
    pub recorder: Option<String>,
    /// Sample rate in Hz.
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// Bit depth as written in the slate.
    #[serde(default)]
    pub bit_depth: Option<String>,
    /// Take number.
    #[serde(default)]
    pub take: Option<String>,
    /// Scene.
    #[serde(default)]
    pub scene: Option<String>,
    /// Slate number.
    #[serde(default)]
    pub slate: Option<String>,
    /// Perspective.
    #[serde(default)]
    pub perspective: Vec<String>,
    /// Date.
    #[serde(default)]
    pub date: Option<String>,
    /// Creator.
    #[serde(default)]
    pub creator: Option<String>,
    /// Free notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Text the parser could not place.
    #[serde(default)]
    pub unparsed: Vec<String>,
}

/// One prediction, exactly as `Prediction.to_dict()` produces it.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Prediction {
    /// The input text.
    #[serde(default)]
    pub text: String,
    /// The description the classifier saw.
    #[serde(default)]
    pub description: String,
    /// Chosen CatID.
    pub cat_id: String,
    /// Category.
    pub category: String,
    /// Subcategory.
    pub subcategory: String,
    /// Short label.
    pub cat_short: String,
    /// `CATEGORY-SUBCATEGORY`.
    pub category_full: String,
    /// Calibrated CatID confidence.
    pub confidence: f64,
    /// Aggregated category confidence.
    #[serde(default)]
    pub category_confidence: f64,
    /// `confident`, `category_fallback` or `unknown`.
    pub decision: String,
    /// Top-k alternatives.
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
    /// Top-k category scores.
    #[serde(default)]
    pub categories: Vec<CategoryScore>,
    /// Suggested FX name.
    #[serde(default)]
    pub fx_name: String,
    /// Suggested keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Suggested UCS filename.
    pub filename: String,
    /// Whether the decision is `unknown`.
    #[serde(default)]
    pub is_unknown: bool,
    /// Parsed slate fields.
    #[serde(default)]
    pub slate: SlateFields,
}

impl Prediction {
    /// The decision as an enum.
    #[must_use]
    pub fn decision_kind(&self) -> DecisionKind {
        DecisionKind::from_wire(&self.decision)
    }
}

/// One taxonomy entry returned by `/taxonomy/*`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaxonomyEntry {
    /// CatID.
    pub cat_id: String,
    /// Category.
    pub category: String,
    /// Subcategory.
    pub subcategory: String,
    /// Short label.
    pub cat_short: String,
    /// Explanation from the UCS list.
    #[serde(default)]
    pub explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_kind_maps_every_documented_value() {
        assert_eq!(
            DecisionKind::from_wire("confident"),
            DecisionKind::Confident
        );
        assert_eq!(
            DecisionKind::from_wire("category_fallback"),
            DecisionKind::CategoryFallback
        );
        assert_eq!(DecisionKind::from_wire("unknown"), DecisionKind::Unknown);
    }

    #[test]
    fn decision_kind_marks_an_unknown_string_rather_than_failing() {
        assert_eq!(
            DecisionKind::from_wire("something_new"),
            DecisionKind::Unrecognised
        );
    }

    #[test]
    fn health_parses_a_full_document() {
        let json = r#"{
            "schema_version": 1, "status": "ok", "warm": true,
            "ttucs_version": "0.3.0", "ucs_version": "8.2.1",
            "model": {"name": "bow", "version": "0.2.0", "source": "package",
                      "manifest_verified": true, "manifest_error": null, "path": "/w"},
            "offline": true, "device": "cpu",
            "unknown_threshold": 0.12, "category_threshold": 0.5, "top_k": 5,
            "limits": {"max_texts": 512, "max_body_bytes": 1048576, "max_top_k": 50}
        }"#;

        let health: Health = serde_json::from_str(json).expect("valid health");

        assert_eq!(health.schema_version, 1);
        assert!(health.warm);
        assert!(health.offline);
        assert_eq!(health.model.name, "bow");
        assert_eq!(health.limits.max_texts, 512);
    }

    #[test]
    fn health_tolerates_missing_optional_fields() {
        let json = r#"{
            "schema_version": 1, "status": "ok", "warm": false,
            "ttucs_version": "0.3.0", "ucs_version": "8.2.1",
            "model": {"name": "bow", "version": "0.2.0"},
            "offline": true, "unknown_threshold": 0.12, "category_threshold": 0.5
        }"#;

        let health: Health = serde_json::from_str(json).expect("valid health");

        assert_eq!(health.limits, Limits::default());
        assert_eq!(health.device, "");
    }

    #[test]
    fn prediction_parses_and_exposes_the_decision() {
        let json = r#"{
            "text": "door creak", "description": "door creak",
            "cat_id": "DOORCreak", "category": "DOORS", "subcategory": "CREAK",
            "cat_short": "Creak", "category_full": "DOORS-CREAK",
            "confidence": 0.71, "category_confidence": 0.88,
            "decision": "confident",
            "alternatives": [{"cat_id": "DOORWood", "category": "DOORS",
                              "subcategory": "WOOD", "cat_short": "Wood",
                              "confidence": 0.2}],
            "categories": [{"category": "DOORS", "confidence": 0.88}],
            "fx_name": "Door Creak", "keywords": ["Door", "Creak"],
            "filename": "DOORCreak_Door Creak_NONE_NONE",
            "is_unknown": false,
            "slate": {"raw_text": "door creak", "description": "door creak", "take": "3"}
        }"#;

        let prediction: Prediction = serde_json::from_str(json).expect("valid prediction");

        assert_eq!(prediction.decision_kind(), DecisionKind::Confident);
        assert_eq!(prediction.alternatives.len(), 1);
        assert_eq!(prediction.slate.take.as_deref(), Some("3"));
        assert!(!prediction.is_unknown);
    }

    #[test]
    fn prediction_ignores_unknown_keys() {
        let json = r#"{
            "cat_id": "AIRBlow", "category": "AIR", "subcategory": "BLOW",
            "cat_short": "AIR", "category_full": "AIR-BLOW",
            "confidence": 0.5, "decision": "unknown",
            "filename": "AIRBlow_Air_NONE_NONE",
            "a_field_from_the_future": {"nested": true}
        }"#;

        let prediction: Prediction = serde_json::from_str(json).expect("forward compatible");

        assert_eq!(prediction.decision_kind(), DecisionKind::Unknown);
        assert!(prediction.alternatives.is_empty());
    }

    #[test]
    fn slate_fields_default_to_empty() {
        let slate: SlateFields = serde_json::from_str("{}").expect("empty slate");

        assert!(slate.microphones.is_empty());
        assert!(slate.take.is_none());
    }

    #[test]
    fn taxonomy_entry_parses() {
        let json = r#"{"cat_id": "DOORCreak", "category": "DOORS",
                       "subcategory": "CREAK", "cat_short": "Creak"}"#;

        let entry: TaxonomyEntry = serde_json::from_str(json).expect("valid entry");

        assert_eq!(entry.cat_id, "DOORCreak");
        assert_eq!(entry.explanation, "");
    }
}
