# 0006 — An in-tree HTTP client instead of a full stack

Date: 2026-11-14
Status: accepted

## Context

`slate-engine` talks to a loopback JSON API. It needs `GET` and `POST`, a bearer header, a timeout
and a body. It does not need TLS, redirects, connection pooling, proxies, HTTP/2 or cookies.

`reqwest` with rustls would work, but it pulls a large dependency tree into the core install and
adds build time on three platforms for features that cannot be used against `127.0.0.1`.

## Decision

A minimal HTTP/1.1 client in `crates/slate-engine/src/http.rs`, built on `std::net::TcpStream`.
Requests send `Connection: close`; responses are read to `Content-Length` or to EOF.

## Consequences

- No TLS stack, no async runtime and no certificate store in the core install.
- The client is about 150 lines and fully unit-tested, including malformed responses, a missing
  status code and a truncated body.
- It cannot talk to anything but a plain-HTTP loopback endpoint. That is the only thing it is for,
  and the engine refuses a non-loopback bind.
- If the engine ever needs TLS, this decision is revisited rather than extended.

## Revisit when

- the engine moves off loopback, which would also require revisiting the token model and the
  threat model in `SECURITY.md`.
