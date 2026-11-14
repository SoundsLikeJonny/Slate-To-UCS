# Code style

Status: draft, 2026-11-14. Applies to every language in this repository. `AGENTS.md` rule 13 makes
this document binding.

The goal is not uniformity for its own sake. It is that a reader moving from `slate-audio` to the
Tauri shell to the REAPER shim does not have to relearn how names and comments work at each
boundary, and that a reviewer can tell a mistake from a preference.

- [Principles](#principles)
- [Comments and documentation](#comments-and-documentation)
- [Rust](#rust)
- [TypeScript and React](#typescript-and-react)
- [C++](#c)
- [Lua](#lua)
- [Python](#python)
- [Data and configuration files](#data-and-configuration-files)
- [Domain vocabulary](#domain-vocabulary)
- [How this is enforced](#how-this-is-enforced)

---

## Principles

1. **Names carry the meaning; comments carry the reason.** A comment that restates the code is
   noise. A comment that explains why the obvious approach was wrong is the most valuable line in
   the file.
2. **One concept, one name, everywhere.** If the engine calls it `decision`, the UI does not call it
   `status` and the C++ shim does not call it `result_kind`. The contract's field names are the
   vocabulary.
3. **Public surface is documented; internals are documented where non-obvious.** Every exported
   function, type and module gets a doc comment. A private helper gets one only when the reason is
   not visible from the code.
4. **Prefer the language's own idiom over a house style.** A Rust reviewer should not have to
   translate Go habits, and a C++ reviewer should not have to translate Rust ones.
5. **Formatting is a tool's job, never a review comment.** If a formatter can decide it, the
   formatter decides it and no one argues about it in a PR.

---

## Comments and documentation

### What a comment must contain

| kind | required when | shape |
|---|---|---|
| Module / file header | every source file | one sentence on what the file is for, then anything a reader must know before reading it |
| Public API doc | every exported item | what it does, its parameters, its errors, and any invariant the caller must hold |
| Why-comment | the code is non-obvious, surprising, or works around a bug | the reason, and a link to the issue or upstream report |
| Safety comment | every `unsafe` block, every raw pointer, every FFI boundary | `// SAFETY:` followed by the invariant that makes it sound |
| TODO | never without an owner and a reason | `TODO(#123): ...` — an issue number, not a name |

### What a comment must never be

- **No commented-out code.** Git remembers it. Delete it.
- **No restatement.** `// increment i` above `i += 1` is a defect in review.
- **No banner art.** No rows of `*****` or `=====`. A blank line and a doc comment do the job.
- **No attribution comments.** No `// added by`, no dates, no initials. `git blame` is the record.
- **No secrets, paths or transcripts** in a comment, a log line or an example. See `SECURITY.md`.

### Doc comment style per language

| language | public API | module header |
|---|---|---|
| Rust | `///` with `# Errors`, `# Panics`, `# Safety` sections where they apply | `//!` at the top of `lib.rs` and each module |
| TypeScript | TSDoc `/** ... */` with `@param`, `@returns`, `@throws` | file-level `/** ... */` above the imports |
| C++ | `///` Doxygen above the declaration in the header, not the definition | `//` block at the top of the `.cpp` |
| Lua | `---` LuaDoc above each public function | `--` block at the top of the file |
| Python | PEP 257 docstring, Google sections (`Args:`, `Returns:`, `Raises:`) | module docstring on line 1 |

---

## Rust

**Tooling.** `rustfmt` with the repository `rustfmt.toml`; `clippy --all-targets -- -D warnings`.
No `#[allow(...)]` without a comment saying why the lint is wrong here.

**Naming.**

| item | convention | example |
|---|---|---|
| crate | `kebab-case`, `slate-` prefix | `slate-audio` |
| module, file | `snake_case` | `decode/rf64.rs` |
| function, method, local, parameter | `snake_case` | `detect_speech`, `sample_rate` |
| type, trait, enum | `UpperCamelCase` | `AudioBuffer`, `VoiceDetector` |
| enum variant | `UpperCamelCase` | `AudioError::UnsupportedFormat` |
| constant, static | `SCREAMING_SNAKE_CASE` | `MAX_BODY_BYTES` |
| type parameter | single uppercase letter or short `UpperCamelCase` | `T`, `Item` |
| lifetime | short lowercase | `'a`, `'buf` |
| feature flag | `kebab-case` | `ffmpeg-sidecar` |

**Idiom.**

- Return `Result<T, E>` with a crate-local error enum built on `thiserror`. No `anyhow` in a
  library crate; `anyhow` is acceptable only in `slate-cli` and `xtask`.
- Take `&str` and `&[T]` in parameters, return `String` and `Vec<T>`. No `&String` or `&Vec<T>`.
- Accept `impl AsRef<Path>` for paths.
- No `unwrap()` or `expect()` outside tests and `main`. `expect()` in `main` must carry a message a
  user can act on.
- `unsafe` is forbidden workspace-wide except in the FFI crates, each with a `// SAFETY:` comment.
- Prefer iterators over index loops; prefer `?` over nested `match`.
- Keep `pub` minimal. A `pub` item is an API promise; use `pub(crate)` by default.

**Tests.** `#[cfg(test)] mod tests` in-file for unit tests, `tests/` for integration. Test names are
sentences: `fn rejects_rf64_without_ds64_chunk()`. One behaviour per test.

---

## TypeScript and React

**Tooling.** ESLint (`@typescript-eslint`, `react-hooks`, `jsx-a11y`) and Prettier. `tsc --noEmit`
in CI. Line length 100.

**Naming.**

| item | convention | example |
|---|---|---|
| variable, function, hook | `camelCase`; hooks start with `use` | `queueRows`, `useQueue` |
| component, type, interface, enum | `PascalCase` | `QueueTable`, `Prediction` |
| component file | `PascalCase.tsx`, one component per file | `DecisionBadge.tsx` |
| non-component file | `camelCase.ts` | `engine.ts`, `schema.ts` |
| constant | `SCREAMING_SNAKE_CASE` | `MAX_BATCH_SIZE` |
| boolean | `is` / `has` / `can` / `should` prefix | `isWarm`, `hasOverrides` |
| event handler prop | `on` prefix; handler `handle` prefix | `onSelect` / `handleSelect` |
| zod schema | `PascalCase` + `Schema` suffix | `PredictionSchema` |
| store | `use` + `PascalCase` + `Store` | `useQueueStore` |

**Idiom.**

- `strict: true`. No `any`; use `unknown` and narrow. No non-null `!` except on a value the same
  function just checked.
- Every engine response is parsed through its zod schema at the boundary. A shape mismatch is a
  visible error, never a silently empty cell.
- Types come from `packages/contract/gen/ts`, not hand-written duplicates.
- Function components only. No class components. No `default export` for components except route
  entries.
- Props are a named `interface` with `Props` suffix, declared above the component.
- `useEffect` needs a comment when its dependency array is deliberately incomplete.
- No `console.log` in committed code; use the logger in `lib/log.ts`.

**Accessibility.** Every interactive element is reachable by keyboard and has an accessible name.
`jsx-a11y` is an error, not a warning.

---

## C++

The REAPER shim is the only C++ in the repository. It sits between our conventions and the REAPER
SDK, so the rule is explicit: **our code follows Google C++ Style; SDK symbols keep their names
verbatim and are wrapped in thin adapters.**

**Tooling.** `clang-format` with the repository `.clang-format` (Google base, 100 columns, 4-space
indent), `clang-tidy` with `bugprone-*`, `cppcoreguidelines-*`, `modernize-*`, `performance-*`.
Warnings as errors on GCC and Clang; `/W4 /WX` on MSVC.

**Naming.**

| item | convention | example |
|---|---|---|
| file | `snake_case.cpp` / `.h` | `engine_client.cpp` |
| type, class, struct, enum | `PascalCase` | `EngineClient`, `TagResult` |
| function, method, local, parameter | `snake_case` | `tag_selected_items`, `item_count` |
| member variable | `snake_case_` with trailing underscore | `port_`, `token_` |
| constant, enumerator | `k` + `PascalCase` | `kMaxRetries`, `kTimerHz` |
| macro | `SLATE_` + `SCREAMING_SNAKE_CASE` | `SLATE_LOG_WARN` |
| namespace | `slate` | `slate::reaper` |

**SDK boundary.** `GetSetMediaItemInfo_String`, `P_NOTES`, `Undo_BeginBlock2`,
`plugin_register` and every other SDK symbol keep their exact spelling. They appear only inside
`src/notes.cpp`, `src/undo.cpp`, `src/plugin.cpp` and `src/marshal.cpp`; the rest of the shim calls
our wrappers. A reviewer should be able to see, from the file list, every place the SDK is touched.

**Idiom.**

- C++17. No exceptions across the SDK boundary; every entry point is `noexcept` and returns a status.
- RAII for every resource. No naked `new`/`delete`. `std::unique_ptr` by default.
- `const` on everything that can be. `const&` for non-trivial parameters.
- No `using namespace` in a header. No `#include` of a `.cpp`.
- No blocking call on the main thread. HTTP runs on a worker; results cross through the queue in
  `marshal.cpp` and are drained from the timer callback.
- Every `reinterpret_cast` and every raw pointer from the SDK carries a `// SAFETY:` comment naming
  the invariant that makes it valid.

---

## Lua

ReaScripts only. Lua 5.4 as embedded by REAPER.

**Tooling.** `stylua` with the repository `stylua.toml` (2-space indent, 100 columns). `luacheck`
in CI.

**Naming.**

| item | convention | example |
|---|---|---|
| file | `snake_case.lua`, `slate_` prefix for entry scripts | `slate_tag_selected.lua` |
| local variable, function | `snake_case` | `local item_count`, `function tag_item(item)` |
| module table | `M` | `local M = {}` |
| constant | `SCREAMING_SNAKE_CASE` | `MAX_ITEMS` |
| global | **none** — every global is a bug | – |

**Idiom.**

- `local` for everything. A global leaks into REAPER's shared Lua state and can collide with another
  script.
- One entry script per action, each with a header block: what it does, what it needs, what it
  changes.
- Shared helpers live in `lib/slate_api.lua` and are required, never copy-pasted.
- Every `reaper.` call that can fail is checked; a nil return is handled, not indexed.
- Wrap project mutations in an undo block so one action is one undo step.
- No `os.execute` with an interpolated path. Use `reaper.ExecProcess` with an argument list.

---

## Python

The sidecar spec, `tools/`, and any scripting around the engine.

**Tooling.** `ruff` for lint and format (line length 100, `E`, `F`, `I`, `UP`, `B`, `SIM` rules),
`mypy --strict` on `tools/`.

**Naming.** PEP 8 without deviation: `snake_case` functions, variables and modules; `PascalCase`
classes; `SCREAMING_SNAKE_CASE` constants; `_leading_underscore` for internal.

**Idiom.**

- Type hints on every public function. `from __future__ import annotations` at the top.
- Dataclasses for data, not dicts, except at a JSON boundary.
- No bare `except:`. Catch the specific exception and either handle it or re-raise with context.
- `pathlib.Path`, never string path concatenation.
- `subprocess.run([...], check=True)`, never `shell=True` with an interpolated string.

---

## Data and configuration files

| file | convention |
|---|---|
| TOML | `snake_case` keys, tables grouped by concern, no deep nesting |
| JSON | `camelCase` keys where the consumer is TypeScript, `snake_case` where it mirrors the engine contract; never mix within one file |
| YAML (workflows) | 2-space indent, one job per concern, `permissions:` stated explicitly in every workflow |
| Markdown | one sentence per line, ATX headings (`##`), tables for tabular facts, no trailing whitespace |
| CSV fixtures | header row, `snake_case` columns, UTF-8 without BOM, LF endings |

Line endings: LF in the repository for every text file, except `*.ps1` which stays CRLF. A
`.gitattributes` pins this so it does not depend on a contributor's `core.autocrlf`.

---

## Domain vocabulary

These spellings are fixed. They appear in code, JSON, docs and UI copy identically.

| term | spelling | never |
|---|---|---|
| Universal Category System | `UCS` | `Ucs`, `ucs` in prose |
| CatID | `cat_id` in JSON and code, `CatID` in prose and UI | `catid`, `catId`, `category_id` |
| category full form | `category_full` | `categoryFull`, `full_category` |
| FX name | `fx_name` | `fxName`, `fxname` |
| Broadcast Wave metadata chunk | `bext` | `BEXT`, `BeXT` |
| iXML chunk | `iXML` | `ixml`, `IXML` |
| decision values | `confident`, `category_fallback`, `unknown` | `ok`, `fallback`, `none` |
| engine | `ttucs serve` | `the server`, `the daemon` |
| sidecar | the PyInstaller Python tree | `the backend` |
| slate | the spoken announcement | `clapper`, `announcement` |

---

## How this is enforced

| check | tool | gate |
|---|---|---|
| Rust formatting | `cargo fmt --check` | CI, blocking |
| Rust lints | `cargo clippy --all-targets -- -D warnings` | CI, blocking |
| Rust docs | `cargo doc` with `#![deny(missing_docs)]` on public crates | CI, blocking |
| TypeScript types | `tsc --noEmit` | CI, blocking |
| TypeScript lint and format | `eslint`, `prettier --check` | CI, blocking |
| C++ format and lint | `clang-format --dry-run --Werror`, `clang-tidy` | CI, blocking |
| Lua format and lint | `stylua --check`, `luacheck` | CI, blocking |
| Python lint and format | `ruff check`, `ruff format --check` | CI, blocking |
| Markdown links | `cargo xtask docs-check` | CI, blocking |
| Naming and comment quality | review | human |

Anything a tool can decide is a tool's decision and is not argued in review. Anything left to
review is listed in the tables above, so a reviewer can point at a rule rather than a preference.
