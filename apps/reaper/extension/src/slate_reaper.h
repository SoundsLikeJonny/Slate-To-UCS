// The Slate-To-UCS REAPER extension.
//
// Everything here runs on REAPER's main thread except the HTTP worker, which owns the
// only socket and hands results back through a lock-free queue drained by a timer
// callback. No allocation, no lock and no I/O happens on the main thread in response to
// a timer tick beyond draining what the worker already produced.

#ifndef SLATE_REAPER_H
#define SLATE_REAPER_H

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace slate {

// The contract version this extension speaks. It must equal the integer the engine
// reports on /health, or the extension refuses to talk to it.
constexpr int kSchemaVersion = 1;

// How often the timer callback drains the worker queue, in seconds. REAPER's timer
// resolution is coarse, so this is a request rather than a guarantee.
constexpr double kTimerInterval = 1.0 / 30.0;

// The longest a single drain may take before it is reported as a stall.
constexpr double kStallBudgetMs = 50.0;

// One row of a prediction, as the engine returns it.
struct Prediction {
  std::string cat_id;
  double confidence = 0.0;
  std::string decision;
  std::string filename;
  std::string fx_name;
  std::vector<std::string> keywords;
};

// What the extension wants done with one media item.
struct ItemPlan {
  std::uintptr_t item = 0;
  std::string notes;
  std::string take_name;
  bool set_take_name = false;
};

// The three launcher steps of PLAN.md 5.4, in the order they are tried.
enum class LaunchStep { Attach, Spawn, Failed };

// A finished unit of work, produced by the worker and consumed by the timer.
struct Result {
  enum class Kind { Health, Predict, Failed } kind = Kind::Failed;
  std::vector<Prediction> predictions;
  std::string error;
  bool warm = false;
};

// ---------------------------------------------------------------- notes

// Merge a slate line into existing notes, preserving every other line.
//
// The slate line is the first line beginning with "slate:"; it is replaced in place, or
// appended when absent. Every other byte of the input is preserved, including line
// endings, so a user's own notes survive a re-tag.
std::string merge_notes(const std::string& existing, const std::string& slate_line);

// The slate line for a prediction: cat_id, confidence and the UCS file name.
std::string slate_line_for(const Prediction& prediction);

// ---------------------------------------------------------------- marshal

// Escape a string for a JSON string literal, including the control characters JSON
// forbids unescaped.
std::string json_escape(const std::string& value);

// Build the /predict request body for a batch of descriptions.
std::string predict_body(const std::vector<std::string>& descriptions, int top_k);

// Parse a /predict response. Returns false and fills `error` on malformed input.
bool parse_predictions(const std::string& body, std::vector<Prediction>* out,
                       std::string* error);

// Parse the integer schema_version out of a /health body.
bool parse_schema_version(const std::string& body, int* out);

// ---------------------------------------------------------------- undo

// One undo block per batch, so a 20-item tag is a single Ctrl+Z.
class UndoBlock {
 public:
  UndoBlock();
  ~UndoBlock();
  UndoBlock(const UndoBlock&) = delete;
  UndoBlock& operator=(const UndoBlock&) = delete;

  // Whether the block opened. A false value means the caller must not write.
  bool open() const { return open_; }

 private:
  bool open_ = false;
};

// ---------------------------------------------------------------- queue

// A single-producer single-consumer queue of results.
//
// The worker pushes, the timer pops. No lock is taken on either side: the indices are
// atomic and the buffer is preallocated, so a push never allocates.
class ResultQueue {
 public:
  explicit ResultQueue(std::size_t capacity);
  ~ResultQueue();
  ResultQueue(const ResultQueue&) = delete;
  ResultQueue& operator=(const ResultQueue&) = delete;

  // Push one result. Returns false when the queue is full, which the worker treats as
  // "drop and report" rather than blocking the socket thread.
  bool push(const Result& result);

  // Pop one result into `out`. Returns false when the queue is empty.
  bool pop(Result* out);

  std::size_t size() const;

 private:
  struct Impl;
  Impl* impl_;
};

// ---------------------------------------------------------------- launcher

// The endpoint the extension talks to: host, port and bearer token.
struct Endpoint {
  std::string host = "127.0.0.1";
  int port = 0;
  std::string token;
  bool valid() const { return port > 0; }
};

// Read $TTUCS_HOME/serve.json, or the platform cache path when the variable is unset.
bool read_endpoint(Endpoint* out, std::string* error);

// Try to attach to a running engine, then to spawn one. Never blocks longer than the
// caller's budget, and never throws into REAPER.
LaunchStep connect_or_spawn(Endpoint* out, std::string* error);

// ---------------------------------------------------------------- http

// A blocking POST on the calling thread. Only the worker calls this.
bool http_post(const Endpoint& endpoint, const std::string& path, const std::string& body,
               std::string* response, std::string* error);

// A blocking GET on the calling thread. Only the worker calls this.
bool http_get(const Endpoint& endpoint, const std::string& path, std::string* response,
              std::string* error);

}  // namespace slate

#endif  // SLATE_REAPER_H
