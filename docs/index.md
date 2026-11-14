# Slate-To-UCS

Turn the slates from your recordings into UCS filenames and metadata.

This site is the documentation for the product family: the desktop application, the REAPER
extension and the REAPER scripts. All three are clients of one engine, `ttucs serve`, which owns
the only loaded model.

## Start here

| document | what it answers |
|---|---|
| [PLAN.md](PLAN.md) | how the product is built: architecture, CI/CD, packaging, code signing, phases, risks |
| [LAYOUT.md](LAYOUT.md) | where every file goes and what it does |
| [MILESTONES.md](MILESTONES.md) | what is being built in the next eight weeks, with exit criteria |
| [CODE_STYLE.md](CODE_STYLE.md) | naming, comments and formatting per language |
| [TESTING.md](TESTING.md) | what must be tested, with which tool, to which coverage floor |
| [../SECURITY.md](../SECURITY.md) | reporting, threat model, verifying a download |
| [../AGENTS.md](../AGENTS.md) | the hard rules, for contributors and agents |

## The shape of the thing

```
audio file
   │
   ▼
slate-audio      decode WAV/BWF/RF64/FLAC/MP3/OGG/AAC → 16 kHz mono → VAD → segment
   │
   ▼
whisper.cpp      native binary, shared by the app and the extension
   │
   ▼  transcript
ttucs serve      /parse → slate fields, /predict → CatID + filename + keywords
   │
   ▼
slate-tagger     bext, iXML, .slate.json sidecar, rename journal
   │
   ▼
REAPER           P_NOTES, take name, render pattern
```

The engine is a separate repository, [ttucs](https://github.com/SoundsLikeJonny/ttucs). It is text
in, metadata out, and it never sees audio. Everything above the engine is this repository.

## Decisions

Architecture decisions are recorded one per page in [adr/](adr/). Each states the context, the
decision, the consequences and the trigger that would make us revisit it.

| # | decision |
|---|---|
| [0001](adr/0001-one-consumer-repo.md) | one consumer repository, `ttucs` separate |
| [0002](adr/0002-tauri-and-react.md) | Tauri 2 with React for the desktop shell |
| [0003](adr/0003-ttucs-serve-owns-inference.md) | `ttucs serve` is the only inference owner |
| [0004](adr/0004-schema-version-and-openapi.md) | `schema_version` at runtime, OpenAPI at build time |
| [0005](adr/0005-reaper-shim-over-http.md) | C++ REAPER shim over HTTP, no C ABI in v1 |
| [0006](adr/0006-in-tree-http-client.md) | an in-tree HTTP client instead of a full stack |
