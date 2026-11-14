#!/usr/bin/env python
"""Generate the golden conformance corpus from the pinned ``ttucs`` package.

The Rust port of ``ttucs.taxonomy`` and ``ttucs.text`` must agree with the Python
reference exactly. This script is the only place that agreement is established: it
runs the installed ``ttucs`` over the dev and realistic description sets and writes
one JSON object per line to ``fixtures/golden/predictions.jsonl``.

Run it with the interpreter that has ``ttucs`` installed::

    python tools/gen_golden.py

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
DEFAULT_SOURCES = (
    REPO_ROOT.parent / "ttucs" / "examples" / "dev_descriptions.jsonl",
    REPO_ROOT.parent / "ttucs" / "examples" / "realistic_slates.jsonl",
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


def main(argv: list[str] | None = None) -> int:
    """Write the corpus, or check the committed one."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="fail if the output differs")
    parser.add_argument("--limit", type=int, default=0, help="cap the number of rows")
    args = parser.parse_args(argv)

    descriptions: list[str] = []
    for source in DEFAULT_SOURCES:
        descriptions.extend(_read_descriptions(source))
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
            return 1
        print(f"golden corpus is current: {len(rows)} rows")
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(f"wrote {len(rows)} rows to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
