#!/usr/bin/env python
"""Generate the golden conformance corpus from the pinned ``ttucs`` package.

The Rust port of ``ttucs.taxonomy`` and ``ttucs.text`` must agree with the Python
reference exactly. This script is the only place that agreement is established: it
runs the installed ``ttucs`` over the dev and realistic description sets and writes
one JSON object per line to ``fixtures/golden/predictions.jsonl``.

Run it with an interpreter that has the pinned release installed, in a clean
virtualenv::

    python -m venv .venv-golden
    .venv-golden/bin/pip install "ttucs==0.3.0"
    .venv-golden/bin/python tools/gen_golden.py

An editable ``ttucs`` checkout tracks ``main`` and is normally ahead of the tag, so the
script refuses it unless ``--allow-editable`` is passed for a local experiment.

``cargo xtask gen-golden --check`` regenerates into a temporary file and fails when
the committed corpus differs, so an upstream change that alters a filename is caught
in CI rather than in a user's library.

The realistic slates are the held-out evaluation set. They appear here only as an
equality fixture: nothing in this repository tunes a threshold, a default or a UI
string against them.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = REPO_ROOT / "fixtures" / "golden" / "predictions.jsonl"
# The corpus is an oracle for a *released* reference, not for whatever checkout happens
# to be importable. A local editable install of `ttucs` is routinely ahead of the tag,
# and its parser output then differs from the wheel CI installs, which is how the corpus
# went stale in CI while looking current on a developer machine.
PINNED_TTUCS_VERSION = "0.3.0"
# The inputs are vendored so the corpus can be regenerated and checked without a
# sibling `ttucs` checkout. They are byte-identical copies of
# `ttucs/examples/dev_descriptions.jsonl` and `ttucs/examples/realistic_slates.jsonl`
# at the pinned version; `--sources` points at a live checkout when one is available.
DEFAULT_SOURCES = (
    REPO_ROOT / "fixtures" / "golden" / "sources" / "dev_descriptions.jsonl",
    REPO_ROOT / "fixtures" / "golden" / "sources" / "realistic_slates.jsonl",
)


def _read_descriptions(path: Path) -> list[str]:
    """Read one description per line from a JSONL file or a plain text file."""
    if not path.is_file():
        return []
    out: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("{"):
            record = json.loads(line)
            text = record.get("description") or record.get("text") or ""
            if text:
                out.append(text)
        else:
            out.append(line)
    return out


def collect_descriptions(sources: tuple[Path, ...] | list[Path]) -> list[str]:
    """Read every description from ``sources``, in order."""
    out: list[str] = []
    for source in sources:
        out.extend(_read_descriptions(Path(source)))
    return out


def build_rows(descriptions: list[str]) -> list[dict]:
    """Run the reference implementation over each description."""
    from ttucs.pipeline import TTUCS
    from ttucs.taxonomy import UCSFilename
    from ttucs.text import make_fx_name, make_keywords

    model = TTUCS.from_pretrained()
    rows: list[dict] = []
    for description in descriptions:
        prediction = model.predict(description)
        fx_name = make_fx_name(prediction.description)
        keywords = make_keywords(prediction.description)
        filename = UCSFilename(
            cat_id=prediction.cat_id,
            fx_name=fx_name,
            creator_id="NONE",
            source_id="NONE",
        ).build()
        rows.append(
            {
                "description": description,
                "parsed_description": prediction.description,
                "cat_id": prediction.cat_id,
                "fx_name": fx_name,
                "keywords": keywords,
                "filename": filename,
            }
        )
    return rows


def describe_difference(existing: str, rendered: str) -> str | None:
    """Return a human-readable account of the first difference, or ``None``.

    ``--check`` compares whole files, which tells a maintainer that something moved
    but not what. CI logs are the only place the failure is visible, so the first
    differing row is reported with both versions side by side.
    """
    if existing == rendered:
        return None
    existing_lines = existing.splitlines()
    rendered_lines = rendered.splitlines()
    for index, (want, got) in enumerate(zip(rendered_lines, existing_lines)):
        if want != got:
            return (
                f"row {index + 1} differs\n"
                f"  reference: {want}\n"
                f"  committed: {got}"
            )
    if len(rendered_lines) != len(existing_lines):
        return (
            "row count differs: "
            f"reference has {len(rendered_lines)}, committed has {len(existing_lines)}"
        )
    return "the files differ only in trailing whitespace"


def reference_version() -> str:
    """Return the version of the importable ``ttucs`` package."""
    import ttucs

    return str(getattr(ttucs, "__version__", "unknown"))


def reference_is_editable() -> bool:
    """Return whether the importable ``ttucs`` is an editable source checkout.

    ``pip`` records an editable install in ``direct_url.json``; a wheel install has no
    ``dir_info`` entry. The distinction matters because an editable checkout tracks
    ``main``, which is normally ahead of the pinned release.
    """
    from importlib.metadata import PackageNotFoundError, distribution

    try:
        raw = distribution("ttucs").read_text("direct_url.json")
    except PackageNotFoundError:
        return False
    if not raw:
        return False
    try:
        return bool(json.loads(raw).get("dir_info", {}).get("editable", False))
    except json.JSONDecodeError:
        return False


def check_reference(allow_editable: bool = False) -> str | None:
    """Return why the importable reference is unusable, or ``None`` when it is fine."""
    version = reference_version()
    if version != PINNED_TTUCS_VERSION:
        return (
            f"ttucs {version} is installed but the corpus is pinned to {PINNED_TTUCS_VERSION}; "
            f'install it with: pip install "ttucs=={PINNED_TTUCS_VERSION}"'
        )
    if reference_is_editable() and not allow_editable:
        return (
            "ttucs is an editable checkout, which tracks main rather than the pinned release; "
            "regenerate in a clean virtualenv, or pass --allow-editable for a local experiment"
        )
    return None


def main(argv: list[str] | None = None) -> int:
    """Write the corpus, or check the committed one."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="fail if the output differs")
    parser.add_argument("--limit", type=int, default=0, help="cap the number of rows")
    parser.add_argument(
        "--allow-editable",
        action="store_true",
        help="accept an editable ttucs checkout (local experiments only; never commit the result)",
    )
    parser.add_argument(
        "--sources",
        type=Path,
        nargs="+",
        default=list(DEFAULT_SOURCES),
        help="description files to read instead of the vendored copies",
    )
    args = parser.parse_args(argv)

    problem = check_reference(allow_editable=args.allow_editable)
    if problem:
        print(f"reference unusable: {problem}", file=sys.stderr)
        return 3

    descriptions = collect_descriptions(args.sources)
    if args.limit:
        descriptions = descriptions[: args.limit]
    if not descriptions:
        print("no descriptions found; pass --limit or check the ttucs checkout", file=sys.stderr)
        return 2

    rows = build_rows(descriptions)
    rendered = "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows)

    if args.check:
        existing = args.output.read_text(encoding="utf-8") if args.output.is_file() else ""
        if existing != rendered:
            print(f"golden corpus is stale: {args.output}", file=sys.stderr)
            detail = describe_difference(existing, rendered)
            if detail:
                print(detail, file=sys.stderr)
            return 1
        print(f"golden corpus is current: {len(rows)} rows")
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(f"wrote {len(rows)} rows to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
