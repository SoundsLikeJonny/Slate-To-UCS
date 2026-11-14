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
    use std::io::{Read, Write};

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

    /// Serve `requests` connections on a loopback port, answering each with the body
    /// `handler` returns for that request line. Returns the port.
    fn spawn_router<F>(requests: usize, handler: F) -> u16
    where
        F: Fn(&str) -> String + Send + 'static,
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            for _ in 0..requests {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut sink = [0u8; 4096];
                let n = stream.read(&mut sink).unwrap_or(0);
                let request_line = String::from_utf8_lossy(&sink[..n])
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let body = handler(&request_line);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        port
    }

    fn client_on(port: u16) -> Client {
        Client::with_timeout(
            Endpoint {
                schema_version: 1,
                host: "127.0.0.1".to_string(),
                port,
                token: "t".to_string(),
                pid: 1,
                started: String::new(),
                ttucs_version: String::new(),
            },
            Duration::from_secs(5),
        )
    }

    #[test]
    fn health_reads_the_document() {
        let port = spawn_router(1, |line| {
            assert_eq!(line, "GET /health HTTP/1.1");
            r#"{"schema_version":1,"status":"ok","warm":true,"ttucs_version":"0.3.0","ucs_version":"8.2.1","model":{"name":"bow","version":"0.2.0"},"offline":true,"unknown_threshold":0.4,"category_threshold":0.4}"#.to_string()
        });

        let health = client_on(port).health().expect("health");

        assert!(health.warm);
        assert_eq!(health.status, "ok");
    }

    #[test]
    fn predict_reads_the_envelope() {
        let port = spawn_router(1, |line| {
            assert_eq!(line, "POST /predict HTTP/1.1");
            r#"{"schema_version":1,"predictions":[{"cat_id":"DOORCreak","category":"DOOR","subcategory":"Creak","cat_short":"Creak","category_full":"DOOR-Creak","confidence":0.9,"decision":"confident","filename":"DOORCreak_Door Creak_NONE_NONE","fx_name":"Door Creak","keywords":["Door","Creak"]}]}"#.to_string()
        });

        let predictions = client_on(port)
            .predict(&["door creak".to_string()], Some(3))
            .expect("predict");

        assert_eq!(predictions.len(), 1);
        assert_eq!(predictions[0].cat_id, "DOORCreak");
    }

    #[test]
    fn parse_reads_the_envelope() {
        let port = spawn_router(1, |line| {
            assert_eq!(line, "POST /parse HTTP/1.1");
            r#"{"schema_version":1,"slates":[{"description":"door creak","normalized_text":"door creak"}]}"#.to_string()
        });

        let slates = client_on(port)
            .parse(&["door creak".to_string()])
            .expect("parse");

        assert_eq!(slates.len(), 1);
        assert_eq!(slates[0].description, "door creak");
    }

    #[test]
    fn taxonomy_search_encodes_the_query() {
        let port = spawn_router(1, |line| {
            assert_eq!(line, "GET /taxonomy/search?q=door%20creak&limit=5 HTTP/1.1");
            r#"{"schema_version":1,"entries":[]}"#.to_string()
        });

        let entries = client_on(port)
            .taxonomy_search("door creak", 5)
            .expect("search");

        assert!(entries.is_empty());
    }

    #[test]
    fn taxonomy_entry_reads_one_entry() {
        let port = spawn_router(1, |line| {
            assert_eq!(line, "GET /taxonomy/DOORCreak HTTP/1.1");
            r#"{"schema_version":1,"entry":{"cat_id":"DOORCreak","category":"DOOR","subcategory":"Creak","cat_short":"Creak"}}"#
                .to_string()
        });

        let entry = client_on(port).taxonomy_entry("DOORCreak").expect("entry");

        assert_eq!(entry.cat_id, "DOORCreak");
    }

    #[test]
    fn shutdown_posts_an_empty_object() {
        let port = spawn_router(1, |line| {
            assert_eq!(line, "POST /shutdown HTTP/1.1");
            "{}".to_string()
        });

        client_on(port).shutdown().expect("shutdown");
    }

    #[test]
    fn a_schema_mismatch_is_reported() {
        let port = spawn_router(1, |_| {
            r#"{"schema_version":99,"status":"ok","warm":true,"ttucs_version":"0.3.0","ucs_version":"8.2.1","model":{"name":"bow","version":"0.2.0"},"offline":true,"unknown_threshold":0.4,"category_threshold":0.4}"#.to_string()
        });

        let err = client_on(port).health().expect_err("mismatch");

        assert!(
            matches!(err, EngineError::SchemaMismatch { .. }),
            "got {err:?}"
        );
    }
}
