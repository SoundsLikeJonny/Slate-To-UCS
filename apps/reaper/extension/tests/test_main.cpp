// Unit tests for the extension's pure logic: notes, marshalling and the queue.
//
// No framework, so the extension has no dependency a REAPER user has to install. Each
// check names the invariant and prints both values on failure.

#include "slate_reaper.h"

#include <cstdio>
#include <string>
#include <vector>

namespace {

int g_failures = 0;
int g_checks = 0;

void check(bool condition, const char* what, const std::string& detail) {
  ++g_checks;
  if (!condition) {
    ++g_failures;
    std::printf("FAIL %s: %s\n", what, detail.c_str());
  }
}

void check_eq(const std::string& actual, const std::string& expected, const char* what) {
  check(actual == expected, what, "expected [" + expected + "] got [" + actual + "]");
}

void check_eq_int(int actual, int expected, const char* what) {
  check(actual == expected, what,
        "expected " + std::to_string(expected) + " got " + std::to_string(actual));
}

// ---------------------------------------------------------------- notes

void test_notes_appends_when_absent() {
  const std::string merged = slate::merge_notes("take 3", "slate: DOORWood 0.912");
  check_eq(merged, "take 3\nslate: DOORWood 0.912", "append to existing notes");
}

void test_notes_preserves_every_other_line() {
  const std::string existing = "mic: MKH 416\nslate: OLD 0.100\nnotes: keep me";
  const std::string merged = slate::merge_notes(existing, "slate: DOORWood 0.912");
  check_eq(merged, "mic: MKH 416\nslate: DOORWood 0.912\nnotes: keep me",
           "replace the slate line in place");
}

void test_notes_keeps_crlf() {
  const std::string existing = "a\r\nslate: OLD 0.1\r\nb";
  const std::string merged = slate::merge_notes(existing, "slate: NEW 0.9");
  check_eq(merged, "a\r\nslate: NEW 0.9\r\nb", "CRLF input stays CRLF");
}

void test_notes_on_empty_input() {
  check_eq(slate::merge_notes("", "slate: X 0.5"), "slate: X 0.5", "empty notes");
}

void test_notes_replaces_only_the_first_slate_line() {
  const std::string existing = "slate: A 0.1\nslate: B 0.2";
  const std::string merged = slate::merge_notes(existing, "slate: C 0.3");
  check_eq(merged, "slate: C 0.3\nslate: B 0.2", "only the first slate line is replaced");
}

void test_slate_line_format() {
  slate::Prediction prediction;
  prediction.cat_id = "DOORWood";
  prediction.confidence = 0.9123;
  prediction.filename = "DOORWood_Door Wood Close_NONE_NONE";
  check_eq(slate::slate_line_for(prediction),
           "slate: DOORWood 0.912 DOORWood_Door Wood Close_NONE_NONE", "slate line format");
}

void test_slate_line_without_filename() {
  slate::Prediction prediction;
  prediction.cat_id = "DOORWood";
  prediction.confidence = 0.5;
  check_eq(slate::slate_line_for(prediction), "slate: DOORWood 0.500",
           "slate line without a filename");
}

// ---------------------------------------------------------------- marshal

void test_escape_covers_json_controls() {
  check_eq(slate::json_escape("a\"b\\c\nd\te"), "a\\\"b\\\\c\\nd\\te", "escape quotes and controls");
  check_eq(slate::json_escape(std::string("a\x01" "b")), "a\\u0001b", "escape a raw control byte");
}

void test_predict_body_shape() {
  const std::vector<std::string> texts = {"door creak", "car \"horn\""};
  check_eq(slate::predict_body(texts, 5),
           "{\"texts\":[\"door creak\",\"car \\\"horn\\\"\"],\"top_k\":5}", "predict body");
}

void test_predict_body_is_empty_for_no_texts() {
  check_eq(slate::predict_body({}, 3), "{\"texts\":[],\"top_k\":3}", "empty predict body");
}

void test_parse_predictions_reads_every_field() {
  const std::string body =
      "[{\"cat_id\":\"DOORWood\",\"confidence\":0.912,\"decision\":\"confident\","
      "\"filename\":\"DOORWood_Door Wood Close_NONE_NONE\",\"fx_name\":\"Door Wood Close\","
      "\"keywords\":[\"Door\",\"Wood\"]}]";
  std::vector<slate::Prediction> out;
  std::string error;
  const bool ok = slate::parse_predictions(body, &out, &error);
  check(ok, "parse predictions", error);
  check_eq_int(static_cast<int>(out.size()), 1, "one prediction");
  if (!out.empty()) {
    check_eq(out[0].cat_id, "DOORWood", "cat_id");
    check_eq(out[0].decision, "confident", "decision");
    check_eq(out[0].filename, "DOORWood_Door Wood Close_NONE_NONE", "filename");
    check_eq(out[0].fx_name, "Door Wood Close", "fx_name");
    check_eq_int(static_cast<int>(out[0].keywords.size()), 2, "keyword count");
    check(out[0].confidence > 0.911 && out[0].confidence < 0.913, "confidence", "0.912");
  }
}

void test_parse_predictions_accepts_an_empty_array() {
  std::vector<slate::Prediction> out;
  std::string error;
  check(slate::parse_predictions("[]", &out, &error), "empty array", error);
  check_eq_int(static_cast<int>(out.size()), 0, "no predictions");
}

void test_parse_predictions_skips_an_unknown_key() {
  const std::string body = "[{\"future\":\"x\",\"cat_id\":\"A\"}]";
  std::vector<slate::Prediction> out;
  std::string error;
  check(slate::parse_predictions(body, &out, &error), "unknown key tolerated", error);
  check_eq(out.empty() ? std::string() : out[0].cat_id, "A", "cat_id after an unknown key");
}

void test_parse_predictions_rejects_malformed() {
  std::vector<slate::Prediction> out;
  std::string error;
  check(!slate::parse_predictions("{\"cat_id\":\"A\"}", &out, &error), "object is not an array",
        error);
  check(!slate::parse_predictions("[{\"cat_id\":}]", &out, &error), "missing value", error);
  check(!slate::parse_predictions("[{\"cat_id\":\"A\"}", &out, &error), "unterminated", error);
}

void test_parse_schema_version() {
  int version = 0;
  check(slate::parse_schema_version("{\"warm\": true, \"schema_version\": 1}", &version),
        "parse schema_version", "1");
  check_eq_int(version, 1, "schema_version value");
  check(!slate::parse_schema_version("{\"warm\": true}", &version), "missing schema_version", "");
}

// ---------------------------------------------------------------- queue

void test_queue_round_trips() {
  slate::ResultQueue queue(4);
  slate::Result pushed;
  pushed.kind = slate::Result::Kind::Health;
  pushed.warm = true;
  check(queue.push(pushed), "push", "queue accepted one");

  slate::Result popped;
  check(queue.pop(&popped), "pop", "queue returned one");
  check(popped.kind == slate::Result::Kind::Health, "kind survives", "Health");
  check(popped.warm, "warm survives", "true");
  check(!queue.pop(&popped), "empty after pop", "queue is empty");
}

void test_queue_refuses_when_full() {
  slate::ResultQueue queue(2);
  slate::Result result;
  check(queue.push(result), "push 1", "");
  check(queue.push(result), "push 2", "");
  check(!queue.push(result), "push 3 refused", "a full queue reports rather than blocks");
  check_eq_int(static_cast<int>(queue.size()), 2, "size stays at capacity");
}

void test_queue_preserves_order() {
  slate::ResultQueue queue(8);
  for (int i = 0; i < 5; ++i) {
    slate::Result result;
    result.error = std::to_string(i);
    queue.push(result);
  }
  for (int i = 0; i < 5; ++i) {
    slate::Result popped;
    check(queue.pop(&popped), "pop in order", "");
    check_eq(popped.error, std::to_string(i), "FIFO order");
  }
}

void test_queue_wraps() {
  slate::ResultQueue queue(2);
  slate::Result result;
  for (int i = 0; i < 10; ++i) {
    check(queue.push(result), "push after wrap", "");
    slate::Result popped;
    check(queue.pop(&popped), "pop after wrap", "");
  }
  check_eq_int(static_cast<int>(queue.size()), 0, "empty after wrapping");
}

}  // namespace

int main() {
  test_notes_appends_when_absent();
  test_notes_preserves_every_other_line();
  test_notes_keeps_crlf();
  test_notes_on_empty_input();
  test_notes_replaces_only_the_first_slate_line();
  test_slate_line_format();
  test_slate_line_without_filename();

  test_escape_covers_json_controls();
  test_predict_body_shape();
  test_predict_body_is_empty_for_no_texts();
  test_parse_predictions_reads_every_field();
  test_parse_predictions_accepts_an_empty_array();
  test_parse_predictions_skips_an_unknown_key();
  test_parse_predictions_rejects_malformed();
  test_parse_schema_version();

  test_queue_round_trips();
  test_queue_refuses_when_full();
  test_queue_preserves_order();
  test_queue_wraps();

  std::printf("%d checks, %d failures\n", g_checks, g_failures);
  return g_failures == 0 ? 0 : 1;
}
