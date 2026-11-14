"""Unit tests for ``tools/gen_golden.py``.

The corpus builder needs the pinned ``ttucs`` package, so those tests skip when it is
not importable. Everything else — reading inputs, resolving sources, and the
``--check`` comparison — runs everywhere.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import gen_golden  # noqa: E402


def test_read_descriptions_accepts_jsonl(tmp_path: Path) -> None:
    path = tmp_path / "in.jsonl"
    path.write_text(
        '{"description": "door creak"}\n{"text": "window slam"}\n', encoding="utf-8"
    )

    assert gen_golden._read_descriptions(path) == ["door creak", "window slam"]


def test_read_descriptions_accepts_plain_lines(tmp_path: Path) -> None:
    path = tmp_path / "in.txt"
    path.write_text("door creak\n\n  window slam  \n", encoding="utf-8")

    assert gen_golden._read_descriptions(path) == ["door creak", "window slam"]


def test_read_descriptions_skips_records_without_text(tmp_path: Path) -> None:
    path = tmp_path / "in.jsonl"
    path.write_text('{"description": ""}\n{"other": 1}\n{"description": "kept"}\n', encoding="utf-8")

    assert gen_golden._read_descriptions(path) == ["kept"]


def test_read_descriptions_of_a_missing_file_is_empty(tmp_path: Path) -> None:
    assert gen_golden._read_descriptions(tmp_path / "absent.jsonl") == []


def test_collect_descriptions_keeps_source_order(tmp_path: Path) -> None:
    first = tmp_path / "a.jsonl"
    second = tmp_path / "b.jsonl"
    first.write_text('{"description": "one"}\n', encoding="utf-8")
    second.write_text('{"description": "two"}\n', encoding="utf-8")

    assert gen_golden.collect_descriptions([first, second]) == ["one", "two"]


def test_vendored_sources_exist_and_are_non_empty() -> None:
    for source in gen_golden.DEFAULT_SOURCES:
        assert source.is_file(), f"missing vendored source {source}"
        assert gen_golden._read_descriptions(source), f"empty vendored source {source}"


def test_vendored_sources_hold_the_expected_row_counts() -> None:
    counts = [len(gen_golden._read_descriptions(s)) for s in gen_golden.DEFAULT_SOURCES]

    assert counts == [332, 80]


def test_main_reports_missing_sources(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    code = gen_golden.main(["--sources", str(tmp_path / "absent.jsonl"), "--allow-editable"])

    assert code == 2
    assert "no descriptions found" in capsys.readouterr().err


def test_describe_difference_is_none_for_identical_text() -> None:
    assert gen_golden.describe_difference('{"a": 1}\n', '{"a": 1}\n') is None


def test_describe_difference_names_the_first_differing_row() -> None:
    existing = '{"a": 1}\n{"a": 2}\n{"a": 3}\n'
    rendered = '{"a": 1}\n{"a": 9}\n{"a": 3}\n'

    detail = gen_golden.describe_difference(existing, rendered)

    assert detail is not None
    assert "row 2 differs" in detail
    assert 'reference: {"a": 9}' in detail
    assert 'committed: {"a": 2}' in detail
    assert "row 3" not in detail


def test_describe_difference_reports_a_row_count_change() -> None:
    detail = gen_golden.describe_difference('{"a": 1}\n', '{"a": 1}\n{"a": 2}\n')

    assert detail == "row count differs: reference has 2, committed has 1"


def test_describe_difference_reports_a_trailing_newline_only() -> None:
    detail = gen_golden.describe_difference('{"a": 1}', '{"a": 1}\n')

    assert detail == "the files differ only in trailing whitespace"


def test_check_reference_accepts_the_pinned_release(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(gen_golden, "reference_version", lambda: "0.3.0")
    monkeypatch.setattr(gen_golden, "reference_is_editable", lambda: False)

    assert gen_golden.check_reference() is None


def test_check_reference_rejects_another_version(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(gen_golden, "reference_version", lambda: "0.3.1")
    monkeypatch.setattr(gen_golden, "reference_is_editable", lambda: False)

    problem = gen_golden.check_reference()

    assert problem is not None
    assert "0.3.1" in problem
    assert 'ttucs==0.3.0' in problem


def test_check_reference_rejects_an_editable_checkout(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(gen_golden, "reference_version", lambda: "0.3.0")
    monkeypatch.setattr(gen_golden, "reference_is_editable", lambda: True)

    problem = gen_golden.check_reference()

    assert problem is not None
    assert "editable" in problem
    assert gen_golden.check_reference(allow_editable=True) is None


def test_main_refuses_an_unusable_reference(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(gen_golden, "reference_version", lambda: "9.9.9")
    monkeypatch.setattr(gen_golden, "reference_is_editable", lambda: False)

    code = gen_golden.main(["--check"])

    assert code == 3
    assert "reference unusable" in capsys.readouterr().err


def test_main_check_compares_against_the_committed_corpus() -> None:
    pytest.importorskip("ttucs")
    if gen_golden.check_reference() is not None:
        pytest.skip(gen_golden.check_reference() or "")

    assert gen_golden.main(["--check"]) == 0


def test_main_check_fails_on_a_stale_corpus(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    pytest.importorskip("ttucs")
    stale = tmp_path / "stale.jsonl"
    stale.write_text("{}\n", encoding="utf-8")

    code = gen_golden.main(["--check", "--output", str(stale), "--allow-editable"])

    assert code == 1
    assert "stale" in capsys.readouterr().err


def test_build_rows_matches_the_reference_for_one_description() -> None:
    pytest.importorskip("ttucs")

    rows = gen_golden.build_rows(["door creak"])

    assert len(rows) == 1
    row = rows[0]
    assert row["description"] == "door creak"
    assert row["cat_id"]
    assert row["filename"].startswith(row["cat_id"] + "_")
    assert json.dumps(row)  # every field is serialisable
