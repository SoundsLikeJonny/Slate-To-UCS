//! Human and machine renderers, so every command has one of each.

use slate_engine::Prediction;

/// Render predictions as a table, one row each.
#[must_use]
pub fn predictions_table(predictions: &[Prediction]) -> String {
    let mut out = String::new();
    for prediction in predictions {
        out.push_str(&format!(
            "{:<12} {:<6.3} {:<18} {}\n",
            prediction.cat_id, prediction.confidence, prediction.decision, prediction.filename
        ));
        if !prediction.keywords.is_empty() {
            out.push_str(&format!(
                "             keywords: {}\n",
                prediction.keywords.join(", ")
            ));
        }
    }
    out
}

/// Render predictions as a JSON array.
///
/// Always an array, even for one input, matching the engine's `/predict` shape.
#[must_use]
pub fn predictions_json(predictions: &[Prediction]) -> String {
    serde_json::to_string_pretty(predictions).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slate_engine::{Prediction, SlateFields};

    fn sample() -> Prediction {
        Prediction {
            text: "door creak".to_string(),
            description: "door creak".to_string(),
            cat_id: "DOORCreak".to_string(),
            category: "DOORS".to_string(),
            subcategory: "CREAK".to_string(),
            cat_short: "Creak".to_string(),
            category_full: "DOORS-CREAK".to_string(),
            confidence: 0.71,
            category_confidence: 0.88,
            decision: "confident".to_string(),
            alternatives: Vec::new(),
            categories: Vec::new(),
            fx_name: "Door Creak".to_string(),
            keywords: vec!["Door".to_string(), "Creak".to_string()],
            filename: "DOORCreak_Door Creak_NONE_NONE".to_string(),
            is_unknown: false,
            slate: SlateFields::default(),
        }
    }

    #[test]
    fn table_contains_the_cat_id_and_filename() {
        let rendered = predictions_table(&[sample()]);

        assert!(rendered.contains("DOORCreak"));
        assert!(rendered.contains("DOORCreak_Door Creak_NONE_NONE"));
        assert!(rendered.contains("keywords: Door, Creak"));
    }

    #[test]
    fn table_of_nothing_is_empty() {
        assert!(predictions_table(&[]).is_empty());
    }

    #[test]
    fn json_is_always_an_array() {
        let rendered = predictions_json(&[sample()]);

        assert!(rendered.trim_start().starts_with('['));
        assert!(rendered.contains("DOORCreak"));
    }

    #[test]
    fn json_of_nothing_is_an_empty_array() {
        assert_eq!(predictions_json(&[]), "[]");
    }
}
