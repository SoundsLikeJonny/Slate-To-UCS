# Testing standard

Status: draft, 2026-11-14. Applies to every language in this repository. `AGENTS.md` rule 14 makes
this document binding, and rule 15 covers assertions in production code.

The rule in one line: **every function that can be called gets a test, every error path gets a test,
and a test that cannot fail is not a test.**

- [Principles](#principles)
- [What must be tested](#what-must-be-tested)
- [Test structure and naming](#test-structure-and-naming)
- [Per-language tooling](#per-language-tooling)
- [Assertions in production code](#assertions-in-production-code)
- [Property-based and fuzz testing](#property-based-and-fuzz-testing)
- [Golden and conformance tests](#golden-and-conformance-tests)
- [Fixtures and determinism](#fixtures-and-determinism)
- [Coverage](#coverage)
- [Flaky tests](#flaky-tests)
- [CI gates](#ci-gates)

---

## Principles

1. **Test behaviour, not implementation.** A test that breaks when a private helper is renamed was
   testing the wrong thing. Assert on what a caller can observe.
2. **One behaviour per test.** A test with three unrelated assertions reports one failure and hides
   two. Split it.
3. **Every test must be able to fail.** If you cannot describe the change that would make it red,
   it is decoration. Mutation testing is how this is checked, not intuition.
4. **Deterministic or excluded.** No wall-clock dependence, no network, no reliance on filesystem
   ordering, no unseeded randomness. A test that passes 95% of the time is a broken test.
5. **Fast by default, slow behind a flag.** The unit suite must run in seconds so it is run on every
   save. Anything slower is an integration test with its own command.
6. **A bug fix ships with the test that would have caught it.** No exceptions. The test is named
   after the behaviour, not the issue number.

---

## What must be tested

### Coverage of the code itself

| target | requirement |
|---|---|
| Every `pub` function and method | at least one test exercising the documented behaviour |
| Every error variant of every error enum | at least one test that produces it and asserts the variant, not just "is_err" |
| Every branch | both sides of every `if`, `match` arm, `?`, and early return |
| Every boundary | empty input, one element, maximum, maximum + 1, zero, negative, `None`/`null`/`nil` |
| Every parser | valid input, truncated input, wrong magic bytes, oversized field, and a fuzz corpus |
| Every writer | round-trip: write, re-read, compare field by field |
| Every public type | construction, serialisation, deserialisation, and an unknown-field case |

### Coverage of the seams

These are where the product actually breaks, and they are easy to forget:

| seam | test |
|---|---|
| Engine contract | every route, every status code in `docs/serve-api.md` §4, with and without a token |
| `schema_version` gate | accepts 1, refuses 2 with a message the UI can display |
| Launcher | attach to a healthy server, spawn when absent, give up after the timeout, shut down only if owned |
| Filename builder | every forbidden character, empty FX name, over-long name, `NONE` creator, `UserData` present |
| Rename | dry-run diff, collision, invalid name, apply, undo, undo after restart |
| Metadata chunks | file with no `bext`, file with an existing `bext`, file with an unknown chunk, file with a truncated chunk |
| Telemetry payload | asserts no transcript, filename, path or CatID appears in any field |
| Update check | valid signature installs, bad signature refuses, downgrade requires explicit action |

### What does not need a test

- Generated code under `packages/contract/gen/`. It is covered by the drift check, which is stronger.
- Trivial getters and setters with no logic.
- `main()` argument wiring, beyond one smoke test that the binary starts and prints its version.
- Third-party behaviour. Test our use of it, not the library.

---

## Test structure and naming

**Arrange, Act, Assert**, separated by a blank line, in that order. No assertions in the arrange
block, no setup in the assert block.

**Names are sentences describing the behaviour**, not the function under test:

| good | bad |
|---|---|
| `rejects_rf64_without_ds64_chunk` | `test_rf64` |
| `returns_unknown_when_confidence_below_threshold` | `test_predict_2` |
| `preserves_existing_notes_when_tagging` | `test_notes` |
| `undo_restores_names_after_restart` | `test_undo_works` |

**One file per unit**, mirroring the source path: `src/decode/rf64.rs` → tests in the same file under
`#[cfg(test)] mod tests`, or `tests/rf64.rs` for integration-level cases.

**Test data is built by a helper, not copy-pasted.** A `fn slate_text() -> &'static str` beats the
same string literal in nine tests. Helpers live in `tests/common/mod.rs` (Rust), `test/helpers.ts`
(TypeScript), `tests/helpers.h` (C++), `tests/helpers.lua`, `tests/conftest.py` (Python).

---

## Per-language tooling

### Rust

| purpose | tool |
|---|---|
| unit and integration | built-in `#[test]`, `tests/` directory |
| assertions | `assert!`, `assert_eq!`, `assert_ne!`, `matches!` |
| error assertions | `matches!(err, AudioError::UnsupportedFormat { .. })`, not `is_err()` |
| property testing | `proptest` |
| fuzzing | `cargo-fuzz` on every parser |
| coverage | `cargo-llvm-cov`, `--fail-under-lines` |
| mutation | `cargo-mutants` on `slate-ucs` and `slate-tagger` |
| async | `tokio::test` only where the code is async; the crates are sync by design |

```rust
#[test]
fn rejects_rf64_without_ds64_chunk() {
    let bytes = include_bytes!("../../fixtures/audio/rf64_no_ds64.wav");

    let err = decode_wav(bytes).unwrap_err();

    assert!(matches!(err, AudioError::MissingDs64 { .. }), "got {err:?}");
}
```

### TypeScript

| purpose | tool |
|---|---|
| runner | Vitest |
| component tests | `@testing-library/react` with `user-event`, never `fireEvent` |
| assertions | `expect(...).toBe/toEqual/toStrictEqual/toThrow` |
| DOM assertions | `toBeVisible`, `toHaveAccessibleName`, `toHaveTextContent` |
| network | `msw` handlers, so no test ever reaches a real port |
| coverage | `vitest --coverage` with `v8` |

Component tests assert what a user sees and does. No snapshot tests: they pass when the output is
wrong and fail when it is merely different.

### C++

| purpose | tool |
|---|---|
| framework | Catch2 v3, vendored as a single header |
| assertions | `REQUIRE`, `CHECK`, `REQUIRE_THROWS_AS` |
| memory | AddressSanitizer and UndefinedBehaviorSanitizer on the CI test job |
| fuzzing | libFuzzer on the chunk parsers |

`REQUIRE` aborts the test case, `CHECK` continues. Use `REQUIRE` for anything the rest of the test
depends on.

### Lua

| purpose | tool |
|---|---|
| framework | `busted` |
| assertions | `assert.are.equal`, `assert.has_error`, `assert.is_nil` |
| REAPER API | a stub module in `tests/reaper_stub.lua`; tests never load a real REAPER |

### Python

| purpose | tool |
|---|---|
| framework | `pytest` |
| assertions | plain `assert`, with `pytest.raises(ValueError, match="...")` for errors |
| fixtures | `conftest.py`, function-scoped by default |
| coverage | `pytest --cov --cov-fail-under` |

---

## Assertions in production code

Assertions are for invariants that must never hold false. They are not error handling, and they are
not a substitute for a `Result` or a returned error.

| language | mechanism | when to use | when never to use |
|---|---|---|---|
| Rust | `debug_assert!` / `debug_assert_eq!` | internal invariants: an index just bounds-checked, a state machine transition that cannot happen | anything a caller can trigger; anything with a side effect; anything whose cost matters in release |
| Rust | `assert!` | only where the invariant must hold in release too, and the cost is trivial | input validation — return an error instead |
| TypeScript | `node:assert` / a `invariant(cond, msg)` helper that throws | narrowing a value the type system cannot express, at a module boundary | user input; anything the UI must recover from |
| C++ | `assert()` | internal invariants in debug builds | anything reachable from a file or the SDK; note `assert` compiles out under `NDEBUG`, so never put a side effect in it |
| C++ | `SLATE_CHECK` (our macro, always on) | invariants that must hold in release, with a log and a clean return | control flow |
| Lua | `assert(cond, msg)` | argument validation at a public entry point, since Lua has no exceptions | inside a loop over thousands of items |
| Python | `assert` | tests only | production code — `python -O` strips it. Raise `ValueError`/`TypeError` instead |

Two rules that apply everywhere:

1. **An assertion message names the invariant and the values.** `assert!(i < len, "index {i} out of
   bounds for len {len}")`, not `assert!(i < len)`.
2. **No assertion has a side effect.** `assert!(queue.pop().is_some())` disappears in release and
   silently changes behaviour.

---

## Property-based and fuzz testing

Property tests state an invariant and let the framework search for a counterexample. They are
required for the pure functions where the input space is large:

| target | property |
|---|---|
| `UcsFilename` parse → render | round-trips for every valid name |
| filename validator | never accepts a name containing a forbidden character |
| taxonomy lookup | every CatID in the CSV resolves, and every resolved entry is in the CSV |
| resampler | output length is within one sample of `input_len * target / source` |
| `bext` / `iXML` writer → reader | round-trips for arbitrary field values |
| JSON contract types | serialise → deserialise is identity for arbitrary valid instances |

Fuzzing is required for every parser that reads untrusted bytes: WAV/BWF/RF64 chunk walking, `bext`,
`iXML`, and the engine's request body. A crash, a hang over one second, or an allocation over 64 MiB
is a failure. The corpus lives in `fixtures/fuzz/` and grows with each find.

---

## Golden and conformance tests

`fixtures/golden/predictions.jsonl` is generated from the pinned `ttucs` over
`examples/dev_descriptions.jsonl` and `examples/realistic_slates.jsonl`. It is the oracle for the
Rust port of `ttucs.text` and `ttucs.taxonomy`.

- `crates/slate-ucs/tests/conformance.rs` asserts equality on `cat_id`, `filename`, `keywords` and
  `decision` for every row.
- CI regenerates the corpus against the pinned `ttucs` and fails on drift, so an upstream change that
  alters filenames is caught before release rather than after.
- The corpus is pinned to the **released** `ttucs` wheel (`PINNED_TTUCS_VERSION` in
  `tools/gen_golden.py`, `ttucs==0.3.0` in `ci.yml`), never to a working checkout. An editable
  `ttucs` install tracks `main` and is normally ahead of the tag, so its parser output differs from
  the wheel; `tools/gen_golden.py` refuses an editable reference unless `--allow-editable` is passed,
  and the corpus test skips with that reason. Regenerate in a clean virtualenv.
- A stale corpus is reported with the first differing row, both versions side by side, so a CI log
  says what moved rather than only that something did.
- The realistic slates are used here **only** as an equality fixture. They are the held-out set and
  are never used to tune a threshold, a default or a UI string.

A conformance failure is never fixed by loosening the assertion. Either the port is wrong or the
pinned version moved; both are real findings.

---

## Fixtures and determinism

- `fixtures/` stays under 2 MB total. Anything larger is generated by a script at test time.
- Audio fixtures are synthetic or owner-recorded. No third-party library material, ever.
- No test reads the wall clock. Where time matters, inject a clock or use a fixed timestamp.
- No test depends on directory iteration order; sort before asserting.
- No test opens a socket on a fixed port. Use port 0 and read the bound port back.
- No test writes outside a temp directory it created and removes.
- Every test that spawns a process asserts the process is gone afterwards, so a failing test cannot
  leave an orphan holding a port or a file.

---

## Coverage

Coverage is a floor, not a goal. A number is enforced so a regression is visible; it is never used to
argue that untested behaviour is fine.

| scope | line floor | branch floor |
|---|---|---|
| `crates/slate-ucs`, `crates/slate-tagger` | 90% | 85% |
| `crates/slate-audio` | 85% | 80% |
| `crates/slate-engine` | 85% | 80% |
| `apps/desktop/src` | 80% | 75% |
| `apps/reaper/extension` | 70% | 65% |
| `tools/` | 70% | – |
| workspace total | 85% | 80% |

Exclusions are declared in the config with a reason, never inline in the source. A file excluded
because it is FFI glue says so in `codecov.yml`.

---

## Flaky tests

A flaky test is a defect with the same severity as a wrong assertion.

1. **Quarantine within one day.** Move it to the `quarantine` job, which runs but does not gate, and
   open an issue naming the test and the observed failure rate.
2. **Fix or delete within two weeks.** A quarantined test that survives the window is deleted. A test
   nobody trusts is worse than no test.
3. **Never retry to green.** `--retries` is forbidden in CI. A retry hides the race that will reach a
   user.
4. **Never `sleep` to fix a race.** Poll a condition with a deadline, or inject a synchronisation
   point.

---

## CI gates

| check | command | blocking |
|---|---|---|
| Rust unit and integration | `cargo test --workspace` | yes |
| Rust coverage | `cargo llvm-cov --fail-under-lines 85` | yes |
| Rust mutation (nightly) | `cargo mutants` on `slate-ucs`, `slate-tagger` | nightly, reported |
| TypeScript | `pnpm -r test --coverage` | yes |
| C++ | `ctest` under ASan and UBSan | yes |
| Lua | `busted` | yes |
| Python | `pytest --cov --cov-fail-under=70` | yes |
| Fuzz smoke | 60 s per parser target | nightly |
| Golden drift | `cargo xtask gen-golden --check` | yes |
| Contract drift | regenerate, `git diff --exit-code` | yes |

A PR that lowers coverage below a floor fails. A PR that adds a `pub` function without a test fails
review, and the reviewer points at this document rather than at a preference.
