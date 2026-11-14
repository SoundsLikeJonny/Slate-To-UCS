//! A minimal HTTP/1.1 client for a loopback JSON API.
//!
//! No TLS, no connection pooling, no redirects: the engine is on `127.0.0.1` and
//! speaks plain HTTP with `Connection: close`. Keeping this in-tree avoids pulling a
//! full HTTP stack and its dependency tree into the core install.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::error::EngineError;

/// A parsed HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Body as UTF-8, lossily decoded.
    pub body: String,
}

/// Send one request and read the whole response.
///
/// # Errors
///
/// Returns [`EngineError::Transport`] for any socket failure and
/// [`EngineError::Malformed`] when the response cannot be parsed.
pub(crate) fn request(
    host: &str,
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
    timeout: Duration,
) -> Result<HttpResponse, EngineError> {
    let address = (host, port)
        .to_socket_addrs()
        .map_err(|e| EngineError::Transport(format!("resolve {host}:{port}: {e}")))?
        .next()
        .ok_or_else(|| EngineError::Transport(format!("no address for {host}:{port}")))?;

    let mut stream = TcpStream::connect_timeout(&address, timeout)
        .map_err(|e| EngineError::Transport(format!("connect {host}:{port}: {e}")))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let mut head =
        format!("{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n");
    if let Some(token) = token {
        head.push_str("X-Ttucs-Token: ");
        head.push_str(token);
        head.push_str("\r\n");
    }
    if let Some(body) = body {
        head.push_str("Content-Type: application/json\r\n");
        head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    head.push_str("\r\n");
    if let Some(body) = body {
        head.push_str(body);
    }

    stream
        .write_all(head.as_bytes())
        .map_err(|e| EngineError::Transport(format!("write: {e}")))?;
    let _ = stream.flush();

    let mut raw: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buffer[..n]);
                if let Some(total) = expected_total(&raw) {
                    if raw.len() >= total {
                        break;
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(EngineError::Transport(format!("read: {e}"))),
        }
    }

    parse(&raw)
}

/// Total bytes expected, when the headers carry a `Content-Length`.
fn expected_total(raw: &[u8]) -> Option<usize> {
    let split = find_header_end(raw)?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    for line in head.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            let len: usize = value.trim().parse().ok()?;
            return Some(split + 4 + len);
        }
    }
    None
}

/// Index of the `\r\n\r\n` that ends the headers.
fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Split a raw response into status and body.
fn parse(raw: &[u8]) -> Result<HttpResponse, EngineError> {
    let split = find_header_end(raw)
        .ok_or_else(|| EngineError::Malformed("response has no header terminator".to_string()))?;
    let head = std::str::from_utf8(&raw[..split])
        .map_err(|e| EngineError::Malformed(format!("headers are not UTF-8: {e}")))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| EngineError::Malformed("response has no status line".to_string()))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| EngineError::Malformed(format!("bad status line: {status_line:?}")))?
        .parse::<u16>()
        .map_err(|e| EngineError::Malformed(format!("bad status code: {e}")))?;
    let body = String::from_utf8_lossy(&raw[split + 4..]).to_string();
    Ok(HttpResponse { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_status_and_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";

        let response = parse(raw).expect("parsed");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "{}");
    }

    #[test]
    fn parses_a_non_200_status() {
        let raw = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

        assert_eq!(parse(raw).expect("parsed").status, 401);
    }

    #[test]
    fn rejects_a_response_without_a_header_terminator() {
        let err = parse(b"HTTP/1.1 200 OK").expect_err("no terminator");

        assert!(matches!(err, EngineError::Malformed(_)), "got {err:?}");
    }

    #[test]
    fn rejects_a_response_without_a_status_code() {
        let err = parse(b"HTTP/1.1\r\n\r\n").expect_err("no code");

        assert!(matches!(err, EngineError::Malformed(_)), "got {err:?}");
    }

    #[test]
    fn rejects_a_non_numeric_status_code() {
        let err = parse(b"HTTP/1.1 OK OK\r\n\r\n").expect_err("bad code");

        assert!(matches!(err, EngineError::Malformed(_)), "got {err:?}");
    }

    #[test]
    fn finds_the_expected_total_from_content_length() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";

        assert_eq!(expected_total(raw), Some(raw.len()));
    }

    #[test]
    fn content_length_is_matched_case_insensitively() {
        let raw = b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}";

        assert_eq!(expected_total(raw), Some(raw.len()));
    }

    #[test]
    fn no_content_length_means_read_to_eof() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";

        assert_eq!(expected_total(raw), None);
    }

    #[test]
    fn a_truncated_body_is_still_parsed() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\npartial";

        assert_eq!(parse(raw).expect("parsed").body, "partial");
    }

    /// Serve one canned response on a loopback socket and return its port.
    fn serve_once(response: &'static str) -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut sink = [0u8; 1024];
                let _ = stream.read(&mut sink);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        port
    }

    #[test]
    fn request_round_trips_a_get() {
        let port = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}");

        let response = request(
            "127.0.0.1",
            port,
            "GET",
            "/health",
            None,
            None,
            Duration::from_secs(5),
        )
        .expect("response");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "{\"ok\":true}");
    }

    #[test]
    fn request_sends_the_token_and_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut sink = [0u8; 2048];
                let n = stream.read(&mut sink).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&sink[..n]).to_string());
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
            }
        });

        request(
            "127.0.0.1",
            port,
            "POST",
            "/predict",
            Some("secret"),
            Some("{\"texts\":[]}"),
            Duration::from_secs(5),
        )
        .expect("response");

        let sent = rx.recv_timeout(Duration::from_secs(5)).expect("request");
        assert!(
            sent.starts_with("POST /predict HTTP/1.1\r\n"),
            "got {sent:?}"
        );
        assert!(sent.contains("X-Ttucs-Token: secret\r\n"), "got {sent:?}");
        assert!(sent.contains("Content-Length: 12\r\n"), "got {sent:?}");
        assert!(sent.ends_with("{\"texts\":[]}"), "got {sent:?}");
    }

    #[test]
    fn request_reports_a_closed_port_as_transport() {
        // Bind and drop so the port is almost certainly free.
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
            listener.local_addr().expect("addr").port()
        };

        let err = request(
            "127.0.0.1",
            port,
            "GET",
            "/health",
            None,
            None,
            Duration::from_millis(250),
        )
        .expect_err("refused");

        assert!(matches!(err, EngineError::Transport(_)), "got {err:?}");
    }

    #[test]
    fn request_reports_an_unresolvable_host_as_transport() {
        let err = request(
            "no-such-host.invalid",
            1,
            "GET",
            "/health",
            None,
            None,
            Duration::from_millis(250),
        )
        .expect_err("unresolvable");

        assert!(matches!(err, EngineError::Transport(_)), "got {err:?}");
    }
}
