# Security policy

Slate-To-UCS is a local desktop application. It reads audio files, runs speech-to-text and a
classifier on the same machine, and writes metadata back into those files. It has no server, no
accounts and no user data store. That shape decides most of what follows.

- [Reporting a vulnerability](#reporting-a-vulnerability)
- [Supported versions](#supported-versions)
- [Scope](#scope)
- [Threat model](#threat-model)
- [Verifying a download](#verifying-a-download)
- [Code signing](#code-signing)
- [Telemetry and privacy](#telemetry-and-privacy)
- [Dependencies and supply chain](#dependencies-and-supply-chain)
- [Disclosure policy](#disclosure-policy)

---

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Use GitHub's private vulnerability reporting on this repository:
*Security* → *Advisories* → *Report a vulnerability*
(<https://github.com/SoundsLikeJonny/Slate-To-UCS/security/advisories/new>).

Please include:

- the version (`slate --version`) and platform;
- what you did, what happened, and what you expected;
- the smallest reproduction you can manage — a fixture file, a request, a folder layout;
- the impact you believe it has, and whether it needs the user to open a hostile file.

Do not include real recordings, real transcripts, project names or client names. If a reproduction
needs audio, generate a synthetic file or describe its structure instead.

**Response.** This is a single-maintainer project, so these are best-effort targets, not a service
level agreement:

| stage | target |
|---|---|
| acknowledgement | 5 working days |
| initial assessment and severity | 10 working days |
| fix or mitigation plan | 30 days for High and Critical |
| coordinated disclosure | 90 days from the report, or sooner by agreement |

There is no bug bounty. There is no PGP key: GitHub private advisories are the encrypted channel,
and adding a key that is not actually maintained would be worse than not having one.

---

## Supported versions

Pre-1.0, only the latest minor release receives security fixes. There are no long-term support
branches.

| version | supported |
|---|---|
| latest `0.x` | yes |
| any earlier `0.x` | no — upgrade |
| `1.x` and later | latest minor, plus the previous minor for 90 days after a major release |

The REAPER extension and the Lua scripts ship from the same tag as the app and are supported
together. A mismatched extension and app is not a supported configuration; the extension reports the
version it expects.

---

## Scope

### In scope

| area | examples |
|---|---|
| Engine authentication | bypassing `X-Ttucs-Token`, reaching the engine from a browser page, a non-loopback bind |
| Local privilege | a hostile local process driving the engine, reading `serve.json`, or reading another user's endpoint file |
| File handling | path traversal through a filename, a rename escaping the chosen folder, a symlink followed out of it, a write outside the target directory |
| Untrusted input parsing | a malformed WAV/BWF/RF64/`bext`/`iXML` chunk causing a crash, an out-of-bounds read, or unbounded memory growth |
| Metadata writing | corrupting an existing `bext` or `iXML` chunk, losing a recordist's notes, a partial write leaving a truncated file |
| Update path | a forged or replayed update manifest, a signature check that can be skipped, a downgrade to a vulnerable version |
| Telemetry | any user content — transcript, filename, path, CatID, project or location — leaving the machine |
| REAPER extension | memory-safety bugs in the C++ shim, a main-thread stall that loses a session, an undo block that corrupts project state |
| Dependencies | a vulnerable crate, npm package or bundled Python package that is reachable from our code |

### Out of scope

| area | why |
|---|---|
| Classifier accuracy, wrong CatIDs, odd filenames | a quality issue, not a vulnerability. Use a normal issue |
| UCS taxonomy content | upstream data; report it to the UCS project |
| REAPER itself, or any third-party plug-in | not our code |
| An attacker who already has the user's account or physical access | outside the threat model; local code execution is game over by design |
| Denial of service by the same user against their own engine | the engine is a local process the user owns |
| Missing hardening headers, cookie flags, TLS configuration | there is no web server, no cookies and no TLS |
| SmartScreen or Gatekeeper warnings on a fresh release | expected until reputation accrues; see [Code signing](#code-signing) |
| Reports produced only by a scanner with no demonstrated impact | we will look, but a scanner line is not a finding |

If you are unsure whether something is in scope, report it privately and let us decide.

---

## Threat model

What the design assumes, so a report can be judged against it.

**Trusted.** The user's own account and machine. The audio files the user chooses. The `ttucs`
package installed alongside the app. The REAPER installation.

**Untrusted.** Every audio file, every `bext`/`iXML` chunk inside one, every transcript handed to
the engine, every HTTP request that reaches the engine port, and every update manifest.

**Boundaries.**

1. **The engine is loopback-only and token-gated.** It binds `127.0.0.1`, requires
   `X-Ttucs-Token` on every route except `/health`, compares with `hmac.compare_digest`, sends no
   CORS headers, and caps bodies at 1 MiB and 512 texts. A browser page must not be able to reach
   it even if it learns the token.
2. **The endpoint file is user-private.** `$TTUCS_HOME/serve.json` is mode `0600` and carries the
   port and token. A world-readable endpoint file is a finding.
3. **Writes are confined and reversible.** Metadata writes go to the file the user selected. Renames
   stay inside the chosen folder, never overwrite an existing file, and are journalled so a batch
   can be undone. A rename that escapes the folder is a finding.
4. **The engine never touches the network.** `TTUCS_OFFLINE=1` guarantees zero outbound connections
   from the engine and the sidecar. Only the shell may reach a telemetry endpoint or an update feed,
   and only under the rules below.
5. **Updates are signed and verified.** A failed signature check aborts the update rather than
   installing it. A downgrade requires an explicit user action.

**Explicitly not defended against.** A local attacker who can already run code as the user, read
that user's files, or attach a debugger. At that point the audio files, the transcripts and the
token are all readable, and no design in this shape changes that.

---

## Verifying a download

Every release publishes `SHA256SUMS`, `SHA256SUMS.sig` and a Sigstore bundle. Verify before you
install, especially if you did not download from the GitHub Releases page.

### Checksums

```powershell
# Windows PowerShell 5.1
$expected = (Select-String -Path .\SHA256SUMS -Pattern 'x64\.msi').Line.Split(' ')[0]
$actual = (Get-FileHash .\Slate-To-UCS-0.1.0-x64.msi -Algorithm SHA256).Hash.ToLower()
if ($actual -eq $expected) { "OK" } else { "MISMATCH: $actual" }
```

```bash
# macOS and Linux
sha256sum --check --ignore-missing SHA256SUMS
```

### Sigstore signature

```bash
cosign verify-blob \
  --bundle SHA256SUMS.sigstore.json \
  --certificate-identity-regexp 'https://github.com/SoundsLikeJonny/Slate-To-UCS/.*' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  SHA256SUMS
```

### Build provenance

```bash
gh attestation verify Slate-To-UCS-0.1.0-x64.msi --repo SoundsLikeJonny/Slate-To-UCS
```

A failure at any of these three steps means stop. Do not install, and report it privately.

---

## Code signing

### The chain

```
Microsoft Root CA
   └─ Microsoft ID Verified CS AOC CA / DigiCert Trusted Root
        └─ leaf: "Slate-To-UCS" code signing certificate (short-lived, renewed daily)
             └─ private key in a FIPS 140-2 Level 3 HSM (Microsoft-managed)
                  └─ signtool sign /fd SHA256 /tr <RFC 3161> /td SHA256
                       └─ Authenticode signature in the PE / MSI
```

The private key never exists on a developer machine, in the repository, or in a GitHub Actions
secret. Signing runs in CI through OIDC federation to Azure Artifact Signing, and only from the
`release` environment, which requires a reviewer.

Since June 2023 the CA/Browser Forum requires publicly trusted code-signing keys to live on
FIPS 140-2 Level 2 (or Common Criteria EAL4+) hardware, which is why a `.pfx` on disk is not an
option here.

### What is signed

Signed bottom-up, then bundled, then the bundle is signed. Nothing is rebuilt after signing.

1. `slate.exe`, the Tauri app `.exe`, `slate_reaper.dll`, and the sidecar launcher.
2. The MSI and the NSIS `.exe`.
3. macOS: the `.app` and the `.dmg`, with hardened runtime and a stapled notarisation ticket.

### Fingerprints

These are published so a user can confirm the signer without trusting this file's prose. They are
filled in at the first signed release (phase 7) and never edited afterwards; a change here is itself
a security event.

| item | value |
|---|---|
| Windows signer subject | `<PENDING: CN=Slate-To-UCS, O=..., C=...>` |
| Windows certificate SHA-1 thumbprint | `<PENDING>` |
| Windows certificate SHA-256 thumbprint | `<PENDING>` |
| Timestamp authority | `http://timestamp.acs.microsoft.com` (RFC 3161) |
| macOS signing identity | `<PENDING: Developer ID Application: ... (TEAMID)>` |
| macOS Team ID | `<PENDING>` |
| Sigstore OIDC issuer | `https://token.actions.githubusercontent.com` |
| Sigstore identity | `https://github.com/SoundsLikeJonny/Slate-To-UCS/.github/workflows/build.yml@refs/tags/v*` |

### Verifying the signature

```powershell
# Windows PowerShell 5.1
Get-AuthenticodeSignature .\Slate-To-UCS-0.1.0-x64.msi |
  Format-List Status, StatusMessage, SignerCertificate, TimeStamperCertificate
```

Expected: `Status = Valid`, a signer certificate whose subject matches the table above, and a
timestamp certificate present. A missing timestamp is a failure: the certificate is short-lived, so
an untimestamped signature stops verifying within days.

```bash
# macOS
codesign --verify --deep --strict --verbose=2 /Applications/Slate-To-UCS.app
spctl --assess --type execute --verbose=4 /Applications/Slate-To-UCS.app
```

Expected: `accepted`, with `source=Notarized Developer ID`.

### SmartScreen

The certificate is OV-level, not EV, so SmartScreen reputation accrues per certificate and per file
over time rather than being granted on first sight. Early releases may show "Windows protected your
PC" until enough downloads have been seen. That is expected and is not a sign of a bad download;
verify the signature above instead. If a release ever shows a signature failure rather than a
reputation warning, stop and report it.

---

## Telemetry and privacy

Telemetry is **opt-in on first run, default off**, one switch, revocable at any time, with a visible
"what is sent" panel. `DO_NOT_TRACK=1` and `SLATE_TELEMETRY=0` both disable it regardless of the
setting.

| stays on the machine, always | may leave the machine, only with consent |
|---|---|
| audio files and their paths | app version, OS and arch, locale |
| transcripts and slate text | model name/version, `schema_version` |
| CatIDs, filenames, keywords, exports | counts and timings (files processed, ms per predict) |
| project, location, creator and source IDs | decision mix (`confident` / `fallback` / `unknown`) |
| the user's overrides and corrections | crash type and stack, with paths and text stripped |

Events are a fixed, versioned allowlist. There are no free-form strings, so no field can carry a
transcript, a filename or a path. Adding a field is a schema bump and a changelog entry. A test
asserts the payload against a corpus of transcripts, filenames and paths.

A telemetry payload containing any user content is a **Critical** finding. Report it privately.

The engine and the sidecar never send anything. `TTUCS_OFFLINE=1` means zero network from them
regardless of the telemetry setting.

---

## Dependencies and supply chain

- `cargo-deny` enforces a licence allowlist and denies advisories; `cargo-audit` and `osv-scanner`
  run on every PR.
- Dependabot or Renovate groups weekly updates for cargo, npm and actions.
- Every release carries an SBOM per platform (`cargo-cyclonedx` for Rust, `syft` for the bundled
  Python and Node trees) and SLSA build provenance.
- Workflows default to `permissions: contents: read`; `id-token: write` is granted only to the jobs
  that sign or attest.
- Model weights and ASR models are release assets pinned by SHA-256 in `models/manifest.toml` and
  verified before use. A hash mismatch is a hard failure, not a warning.
- The REAPER SDK is fetched at build time into a gitignored directory, pinned by version and hash,
  and never committed.

A dependency vulnerability that is reachable from our code is in scope. One that is not reachable
is still worth a private note; we will upgrade on the normal schedule.

---

## Disclosure policy

- We follow coordinated disclosure: private report, private fix, then a public advisory once a
  fixed release is available.
- The default embargo is 90 days from the report. We will publish sooner if a fix ships sooner, and
  we will say so if we need longer rather than letting the deadline pass in silence.
- Advisories are published as GitHub Security Advisories with a CVE requested for High and Critical
  findings.
- Reporters are credited by name or handle unless they ask not to be.
- We will tell you what we fixed, what we did not, and why. If a report is declined, you get the
  reasoning, not silence.

### Safe harbour

We will not pursue or support legal action against anyone who reports a vulnerability in good faith
under this policy, who tests only against their own installation and their own files, and who does
not access, modify or exfiltrate another person's data. Do not test against infrastructure you do
not own, and do not run automated scanners against the project's GitHub organisation.
