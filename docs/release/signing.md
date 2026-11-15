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
