# Task entry points. One recipe per task, each a thin wrapper over `cargo xtask`
# or a package manager, so there is exactly one way to do each thing.
#
# Run `just` with no arguments to list the recipes.

set shell := ["bash", "-uc"]

default:
    @just --list

# ---------------------------------------------------------------- setup

setup:
    cargo fetch
    cargo xtask verify-versions

# ---------------------------------------------------------------- checks

check: check-fmt check-clippy check-docs check-versions check-changelog

check-fmt:
    cargo fmt --all --check

check-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

check-docs:
    cargo xtask docs-check

check-versions:
    cargo xtask verify-versions

check-changelog:
    cargo xtask changelog --check

# ---------------------------------------------------------------- tests

test:
    cargo test --workspace

test-unit:
    cargo test --workspace --lib

coverage:
    cargo xtask coverage

fuzz:
    cargo xtask fuzz

# ---------------------------------------------------------------- build

build:
    cargo build --workspace --release

# ---------------------------------------------------------------- release

bump version:
    cargo xtask bump {{version}}

changelog version:
    cargo xtask changelog --release {{version}}

verify-versions:
    cargo xtask verify-versions

# ---------------------------------------------------------------- data

gen-golden:
    cargo xtask gen-golden

sync-taxonomy:
    cargo xtask sync-taxonomy
