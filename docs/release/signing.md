# Signing

How a Slate-To-UCS installer is signed, how a user verifies it, and what happens when the
identity is compromised. The policy lives in `SECURITY.md`; this is the procedure.

## What is signed, and in what order

Bottom-up, because a signature covers the bytes it wraps:

1. `slate.exe` and `slate_reaper.dll`, before the installers are built.
2. The MSI and the NSIS `.exe`, after they are built and never rebuilt afterwards.
3. The release manifest, which lists the SHA-256 of every artefact.

A rebuild after signing produces a different hash, so the hash that was signed would no
longer be the hash that ships. `build.yml` therefore signs in a step that runs after both
installers exist and before anything is uploaded.

## The identity

Azure Artifact Signing (formerly Trusted Signing), OV profile, short-lived certificates
issued daily. Nothing is on disk: the signing step obtains a token through a GitHub
federated credential scoped to this repository and the `release` environment.

| setting | value |
|---|---|
| endpoint | `https://eus.codesigning.azure.net` (recorded in the `AZURE_SIGNING_ENDPOINT` variable) |
| account | `AZURE_SIGNING_ACCOUNT` repository variable |
| profile | `AZURE_SIGNING_PROFILE` repository variable |
| timestamp | `http://timestamp.acs.microsoft.com`, RFC 3161, SHA-256 |
| digest | SHA-256 |

`AZURE_SIGNING_ENABLED` must be `true` for the signing step to run. When it is absent the
build still completes, the artefacts are renamed with a `-unsigned` suffix, and
`sign-verify.yml` fails on them. An unsigned artefact can therefore never be published by
accident, and packaging breakage is still visible before the account exists.

## Why the timestamp is not optional

An OV certificate is valid for a short window. Without an RFC 3161 timestamp the signature
stops verifying the moment the certificate expires, so every release would rot. With one,
the signature remains valid indefinitely because the timestamp proves the signing happened
while the certificate was live. `sign-verify.yml` and `release.yml` both refuse an artefact
whose `TimeStamperCertificate` is empty.

## What the user runs

```powershell
Get-AuthenticodeSignature .\Slate-To-UCS-0.1.0-x64.msi |
  Format-List Status, SignerCertificate, TimeStamperCertificate
```

Expected: `Status = Valid`, a signer certificate, and a timestamp present. The stricter
check, which also validates the chain against the machine's trust store:

```powershell
signtool verify /pa /v .\Slate-To-UCS-0.1.0-x64.msi
```

SmartScreen reputation accrues with download volume on an OV certificate; it is not
instant and it is not a defect. An EV certificate would skip the accrual and is not worth
the cost for this project.

## Cost and the free alternatives

| item | cost | what it buys |
|---|---|---|
| Azure Artifact Signing | $9.99/month, about $120/year | Authenticode on the MSI and the NSIS exe, trusted publisher name in UAC and SmartScreen |
| Apple Developer Program | $99/year | the only route to notarisation and a stapled DMG |
| total | about $220/year | |

The Azure tier's signature quota is far above this project's volume, so the fee is a flat
platform charge rather than a per-signature cost. The real gate is identity validation: an
individual must be in the US or Canada, and an organisation needs a verifiable
registration.

Options that cost nothing, in the order worth trying:

1. **SignPath Foundation.** Free code signing for open-source projects. Apply, they verify
   the licence, and they issue and custody the certificate; CI submits the artefact and
   gets it back signed. This is the only zero-cost route to a *trusted* Authenticode
   signature. Trade-offs: discretionary approval taking weeks, and the key is theirs, so
   offline signing is impossible.
2. **Certum open-source certificate.** Historically free to OSS developers. Since the 2023
   CA/Browser Forum rules every OV/EV key must live in an HSM or a vendor cloud service, so
   the certificate may be free while the signing token or subscription is not. Worth a
   quote, not worth assuming zero.
3. **Self-signed certificate.** Free and buys nothing public: the root is not in the trust
   store, so Windows shows "Unknown publisher" exactly as it does unsigned. Useful only for
   a GPO-managed fleet.
4. **Stay unsigned, prove integrity instead.** The release already carries an SBOM, a
   cosign signature and GitHub build provenance. Adding a published SHA-256 per artefact and
   a `cosign verify` line in the README lets a user prove the download is the build that was
   made. What it does not give is a publisher name in the UAC prompt or SmartScreen
   reputation.
5. **MSIX through the Microsoft Store.** A $19 one-time individual developer account.
   Store-distributed packages are signed by Microsoft and do not trigger SmartScreen, at the
   cost of MSIX packaging, Store review, and losing the plain `.exe` download.

macOS notarisation has no free path. Gatekeeper refuses a quarantined unsigned app and the
user must right-click-Open every time, so the $99/year is unavoidable if macOS distribution
matters.

The decision recorded here is: apply to SignPath Foundation, and until it is approved ship
unsigned with cosign, SBOM, provenance and a published checksum. Keep the
`AZURE_SIGNING_ENABLED` gate as it is, so moving to Azure later is a secret and a variable
rather than a rewrite. Revisit when downloads reach the hundreds, which is roughly when
SmartScreen reputation starts to matter.

## Key custody

- No key material in the repository, in Actions secrets, or on a developer machine.
- The signing account is scoped to a dedicated resource group.
- The federated credential is limited to this repository and the `release` environment.
- The `release` environment requires a reviewer, so only an approved run can obtain a token.
- Rotation is automatic: the certificates are re-issued daily by the CA.

## If the identity is compromised

1. Disable the signing account, which stops new signatures immediately.
2. Publish an advisory in `SECURITY.md` naming the affected versions.
3. Re-sign the current release with a new identity and publish it as a new version, never
   by replacing the artefact under an existing tag.
4. Older timestamped releases remain verifiable, which is the reason for timestamping.
