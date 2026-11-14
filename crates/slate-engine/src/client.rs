//! Typed calls against the `ttucs serve` routes.

use std::time::Duration;

use serde::Deserialize;

use crate::endpoint::Endpoint;
use crate::error::EngineError;
use crate::http;
use crate::schema::check_schema_version;
use crate::types::{Health, Prediction, SlateFields, TaxonomyEntry};
use crate::{MAX_BODY_BYTES, MAX_TEXTS};

/// Default per-request timeout. Inference on a cold model can take a few seconds.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// A client bound to one endpoint.
#[derive(Debug, Clone)]
pub struct Client {
    endpoint: Endpoint,
    timeout: Duration,
}

#[derive(Debug, Deserialize)]
struct PredictEnvelope {
    schema_version: u32,
    predictions: Vec<Prediction>,
}

#[derive(Debug, Deserialize)]
struct ParseEnvelope {
    schema_version: u32,
    slates: Vec<SlateFields>,
}

#[derive(Debug, Deserialize)]
struct SearchEnvelope {
    schema_version: u32,
    entries: Vec<TaxonomyEntry>,
}

#[derive(Debug, Deserialize)]
struct EntryEnvelope {
    schema_version: u32,
    entry: TaxonomyEntry,
}

impl Client {
    /// Bind a client to an endpoint.
    #[must_use]
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Bind a client with an explicit timeout.
    #[must_use]
    pub fn with_timeout(endpoint: Endpoint, timeout: Duration) -> Self {
        Self { endpoint, timeout }
    }

    /// The endpoint this client talks to.
    #[must_use]
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// `GET /health`. No token required.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Status`] for a non-200 answer,
    /// [`EngineError::SchemaMismatch`] for an unsupported contract version, and
    /// transport or parse errors as they occur.
    pub fn health(&self) -> Result<Health, EngineError> {
        let response = http::request(
            &self.endpoint.host,
            self.endpoint.port,
            "GET",
            "/health",
            None,
            None,
            self.timeout,
        )?;
        let body = expect_status(response.status, "/health", &response.body)?;
        let health: Health = serde_json::from_str(&body)
            .map_err(|e| EngineError::Malformed(format!("/health: {e}")))?;
        check_schema_version(health.schema_version)?;
        Ok(health)
    }

    /// `POST /predict`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::RequestTooLarge`] when the batch or body exceeds the
    /// engine's caps, [`EngineError::Unauthorised`] on 401, and
    /// [`EngineError::Status`] for any other non-200 answer.
    pub fn predict(
        &self,
        texts: &[String],
        top_k: Option<u32>,
    ) -> Result<Vec<Prediction>, EngineError> {
        let body = self.post_json("/predict", texts, top_k)?;
        let envelope: PredictEnvelope = serde_json::from_str(&body)
            .map_err(|e| EngineError::Malformed(format!("/predict: {e}")))?;
        check_schema_version(envelope.schema_version)?;
        Ok(envelope.predictions)
    }

    /// `POST /parse`.
    ///
    /// # Errors
    ///
    /// As [`Client::predict`].
    pub fn parse(&self, texts: &[String]) -> Result<Vec<SlateFields>, EngineError> {
        let body = self.post_json("/parse", texts, None)?;
        let envelope: ParseEnvelope = serde_json::from_str(&body)
            .map_err(|e| EngineError::Malformed(format!("/parse: {e}")))?;
        check_schema_version(envelope.schema_version)?;
        Ok(envelope.slates)
    }

    /// `GET /taxonomy/search`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Status`] for a non-200 answer.
    pub fn taxonomy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<TaxonomyEntry>, EngineError> {
        let path = format!("/taxonomy/search?q={}&limit={limit}", encode_query(query));
        let response = http::request(
            &self.endpoint.host,
            self.endpoint.port,
            "GET",
            &path,
            Some(self.endpoint.token.as_str()),
            None,
            self.timeout,
        )?;
        let body = expect_status(response.status, "/taxonomy/search", &response.body)?;
        let envelope: SearchEnvelope = serde_json::from_str(&body)
            .map_err(|e| EngineError::Malformed(format!("/taxonomy/search: {e}")))?;
        check_schema_version(envelope.schema_version)?;
        Ok(envelope.entries)
    }

    /// `GET /taxonomy/{cat_id}`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Status`] with 404 for an unknown CatID.
    pub fn taxonomy_entry(&self, cat_id: &str) -> Result<TaxonomyEntry, EngineError> {
        let path = format!("/taxonomy/{}", encode_query(cat_id));
        let response = http::request(
            &self.endpoint.host,
            self.endpoint.port,
            "GET",
            &path,
            Some(self.endpoint.token.as_str()),
            None,
            self.timeout,
        )?;
        let body = expect_status(response.status, "/taxonomy", &response.body)?;
        let envelope: EntryEnvelope = serde_json::from_str(&body)
            .map_err(|e| EngineError::Malformed(format!("/taxonomy: {e}")))?;
        check_schema_version(envelope.schema_version)?;
        Ok(envelope.entry)
    }

    /// `POST /shutdown`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Unauthorised`] on 401 and
    /// [`EngineError::Status`] for any other non-200 answer.
    pub fn shutdown(&self) -> Result<(), EngineError> {
        let response = http::request(
            &self.endpoint.host,
            self.endpoint.port,
            "POST",
            "/shutdown",
            Some(self.endpoint.token.as_str()),
            Some("{}"),
            self.timeout,
        )?;
        expect_status(response.status, "/shutdown", &response.body)?;
        Ok(())
    }

    /// Build and send a `{texts, top_k}` body, enforcing the caps before sending.
    fn post_json(
        &self,
        path: &str,
        texts: &[String],
        top_k: Option<u32>,
    ) -> Result<String, EngineError> {
        if texts.is_empty() {
            return Err(EngineError::RequestTooLarge(
                "at least one text is required".to_string(),
            ));
        }
        if texts.len() > MAX_TEXTS {
            return Err(EngineError::RequestTooLarge(format!(
                "{} texts exceeds the cap of {MAX_TEXTS}",
                texts.len()
            )));
        }
        let mut payload = serde_json::json!({ "texts": texts });
        if let Some(top_k) = top_k {
            payload["top_k"] = serde_json::json!(top_k);
        }
        let body = payload.to_string();
        if body.len() > MAX_BODY_BYTES {
            return Err(EngineError::RequestTooLarge(format!(
                "body of {} bytes exceeds the cap of {MAX_BODY_BYTES}",
                body.len()
            )));
        }
        let response = http::request(
            &self.endpoint.host,
            self.endpoint.port,
            "POST",
            path,
            Some(self.endpoint.token.as_str()),
            Some(&body),
            self.timeout,
        )?;
        expect_status(response.status, path, &response.body)
    }
}

/// Map a status code to a body or a typed error.
fn expect_status(status: u16, path: &str, body: &str) -> Result<String, EngineError> {
    match status {
        200 => Ok(body.to_string()),
        401 => Err(EngineError::Unauthorised),
        _ => Err(EngineError::Status {
            status,
            path: path.to_string(),
            body: truncate(body, 200),
        }),
    }
}

/// Percent-encode a query value, keeping only unreserved characters.
fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Cut a body to `limit` characters for an error message.
fn truncate(body: &str, limit: usize) -> String {
    if body.chars().count() <= limit {
        return body.to_string();
    }
    let cut: String = body.chars().take(limit).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_spaces_and_reserved_characters() {
        assert_eq!(encode_query("door creak"), "door%20creak");
        assert_eq!(encode_query("a&b=c"), "a%26b%3Dc");
        assert_eq!(encode_query("DOORCreak"), "DOORCreak");
    }

    #[test]
    fn leaves_unreserved_characters_alone() {
        assert_eq!(encode_query("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn truncates_a_long_body_with_an_ellipsis() {
        let body = "x".repeat(300);

        let cut = truncate(&body, 200);

        assert_eq!(cut.chars().count(), 201);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn leaves_a_short_body_alone() {
        assert_eq!(truncate("short", 200), "short");
    }

    #[test]
    fn maps_401_to_unauthorised() {
        assert!(matches!(
            expect_status(401, "/predict", ""),
            Err(EngineError::Unauthorised)
        ));
    }

    #[test]
    fn maps_other_statuses_to_status_errors() {
        let err = expect_status(413, "/predict", "too big").expect_err("413");

        match err {
            EngineError::Status { status, path, body } => {
                assert_eq!(status, 413);
                assert_eq!(path, "/predict");
                assert_eq!(body, "too big");
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn maps_200_to_the_body() {
        assert_eq!(expect_status(200, "/health", "{}").expect("ok"), "{}");
    }

    #[test]
    fn rejects_an_empty_batch_before_sending() {
        let client = Client::new(Endpoint {
            schema_version: 1,
            host: "127.0.0.1".to_string(),
            port: 1,
            token: "t".to_string(),
            pid: 1,
            started: String::new(),
            ttucs_version: String::new(),
        });

        let err = client.predict(&[], None).expect_err("empty batch");

        assert!(
            matches!(err, EngineError::RequestTooLarge(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_an_oversized_batch_before_sending() {
        let client = Client::new(Endpoint {
            schema_version: 1,
            host: "127.0.0.1".to_string(),
            port: 1,
            token: "t".to_string(),
            pid: 1,
            started: String::new(),
            ttucs_version: String::new(),
        });
        let texts = vec!["x".to_string(); MAX_TEXTS + 1];

        let err = client.predict(&texts, None).expect_err("too many");

        assert!(
            matches!(err, EngineError::RequestTooLarge(_)),
            "got {err:?}"
        );
    }
}
