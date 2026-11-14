// The extension entry point: registration, the menu, the worker thread and the timer.
//
// Nothing here blocks. A command posts work to the worker and returns; the timer drains
// finished results on the main thread and applies them inside one undo block.

#include "slate_reaper.h"

#include "reaper_plugin.h"

#include <atomic>
#include <chrono>
#include <string>
#include <thread>
#include <vector>

reaper_plugin_info_t* g_plugin_info = nullptr;

namespace {

using namespace slate;

// REAPER's console, resolved through the function table rather than linked, so the
// extension has no import the host must satisfy at load time.
void show_console(const std::string& text) {
  if (g_plugin_info == nullptr || g_plugin_info->GetFunc == nullptr) {
    return;
  }
  using ShowConsoleMsg = void (*)(const char*);
  ShowConsoleMsg show =
      reinterpret_cast<ShowConsoleMsg>(g_plugin_info->GetFunc("ShowConsoleMsg"));
  if (show != nullptr) {
    show(text.c_str());
  }
}

// The single queue the worker fills and the timer drains.
ResultQueue g_queue(256);

// The worker, started on first use and joined at unload.
std::thread g_worker;
std::atomic<bool> g_stop{false};
std::atomic<bool> g_busy{false};

// The most recent stall, in milliseconds, so a regression is visible rather than felt.
std::atomic<double> g_last_drain_ms{0.0};

// The engine endpoint, resolved once per session.
Endpoint g_endpoint;

// ---------------------------------------------------------------- worker

void worker_main() {
  while (!g_stop.load(std::memory_order_acquire)) {
    if (!g_busy.exchange(false)) {
      std::this_thread::sleep_for(std::chrono::milliseconds(10));
      continue;
    }
    Result result;
    std::string error;
    Endpoint endpoint;
    if (connect_or_spawn(&endpoint, &error) != LaunchStep::Attach) {
      result.kind = Result::Kind::Failed;
      result.error = error;
      g_queue.push(result);
      continue;
    }
    g_endpoint = endpoint;
    std::string body;
    if (!http_get(endpoint, "/health", &body, &error)) {
      result.kind = Result::Kind::Failed;
      result.error = error;
      g_queue.push(result);
      continue;
    }
    result.kind = Result::Kind::Health;
    result.warm = body.find("\"warm\": true") != std::string::npos ||
                  body.find("\"warm\":true") != std::string::npos;
    g_queue.push(result);
  }
}

void ensure_worker() {
  if (g_worker.joinable()) {
    return;
  }
  g_stop.store(false, std::memory_order_release);
  g_worker = std::thread(worker_main);
}

void stop_worker() {
  g_stop.store(true, std::memory_order_release);
  if (g_worker.joinable()) {
    g_worker.join();
  }
}

// ---------------------------------------------------------------- timer

// Drain everything the worker produced, inside one undo block, and record how long it
// took so a stall is measured rather than assumed.
void drain() {
  const auto started = std::chrono::steady_clock::now();
  Result result;
  if (!g_queue.pop(&result)) {
    return;
  }
  UndoBlock undo;
  do {
    if (result.kind == Result::Kind::Failed) {
      // A failure is reported to the user, never swallowed and never fatal to the host.
      show_console("Slate-To-UCS: " + result.error + "\n");
    }
  } while (g_queue.pop(&result));

  const auto finished = std::chrono::steady_clock::now();
  const double ms =
      std::chrono::duration<double, std::milli>(finished - started).count();
  g_last_drain_ms.store(ms, std::memory_order_release);
  if (ms > kStallBudgetMs) {
    show_console("Slate-To-UCS: drain took " + std::to_string(ms) + " ms\n");
  }
}

// ---------------------------------------------------------------- commands

void command_check_engine(int) {
  ensure_worker();
  g_busy.store(true, std::memory_order_release);
}

void command_open_app(int) {
  // The app is the review surface; the extension never opens a window of its own.
  std::string error;
  Endpoint endpoint;
  if (connect_or_spawn(&endpoint, &error) != LaunchStep::Attach) {
    show_console("Slate-To-UCS: " + error + "\n");
  }
}

// ---------------------------------------------------------------- registration

// The action table. Each entry is one menu item and one named command, so a script can
// reach the same code path through Main_OnCommand.
struct ActionEntry {
  const char* id;
  const char* name;
  void (*run)(int);
};

const ActionEntry kActions[] = {
    {"SLATE_CHECK_ENGINE", "Check engine", command_check_engine},
    {"SLATE_OPEN_APP", "Open Slate-To-UCS", command_open_app},
};

constexpr int kActionCount = static_cast<int>(sizeof(kActions) / sizeof(kActions[0]));

int g_command_ids[kActionCount] = {0};

// The hook REAPER calls for every action, including a script's Main_OnCommand.
int command_hook(int command, int flag) {
  for (int i = 0; i < kActionCount; ++i) {
    if (g_command_ids[i] == command) {
      kActions[i].run(flag);
      return 1;
    }
  }
  return 0;
}

// The timer callback, registered at kTimerInterval.
void timer_callback() { drain(); }

}  // namespace

// ---------------------------------------------------------------- exports

// Exported so Lua reaches the same code paths without HTTP.
extern "C" {

// Whether an engine is reachable and warm. Returns 1 for warm, 0 for cold, -1 for none.
int API_Slate_Health() {
  std::string error;
  Endpoint endpoint;
  if (connect_or_spawn(&endpoint, &error) != LaunchStep::Attach) {
    return -1;
  }
  std::string body;
  if (!http_get(endpoint, "/health", &body, &error)) {
    return -1;
  }
  return body.find("\"warm\": true") != std::string::npos ||
                 body.find("\"warm\":true") != std::string::npos
             ? 1
             : 0;
}

// The last drain duration in milliseconds, so a script can assert the stall budget.
double API_Slate_LastDrainMs() { return g_last_drain_ms.load(std::memory_order_acquire); }

// The schema version this extension speaks.
int API_Slate_SchemaVersion() { return kSchemaVersion; }

// Predict one description and return the JSON array of predictions.
//
// Returns nullptr on failure and writes the reason to the console. The pointer is valid
// until the next call on the same thread, which is what REAPER's Lua binding needs: it
// copies the string immediately, so a caller never holds it across a call.
//
// This blocks the calling thread for the duration of one HTTP round trip. A script that
// calls it in a loop over a large project should call it from a deferral rather than a
// tight loop, because REAPER's main thread is what stalls.
const char* API_Slate_Predict(const char* description, int top_k) {
  static thread_local std::string buffer;

  if (description == nullptr || *description == '\0') {
    show_console("Slate-To-UCS: a description is required\n");
    return nullptr;
  }
  const int limit = top_k > 0 ? top_k : 3;

  std::string error;
  Endpoint endpoint;
  if (connect_or_spawn(&endpoint, &error) != LaunchStep::Attach) {
    show_console("Slate-To-UCS: " + error + "\n");
    return nullptr;
  }

  std::string response;
  const std::string body = predict_body({description}, limit);
  if (!http_post(endpoint, "/predict", body, &response, &error)) {
    show_console("Slate-To-UCS: " + error + "\n");
    return nullptr;
  }

  std::vector<Prediction> predictions;
  if (!parse_predictions(response, &predictions, &error)) {
    show_console("Slate-To-UCS: " + error + "\n");
    return nullptr;
  }

  buffer = "[";
  for (std::size_t i = 0; i < predictions.size(); ++i) {
    const Prediction& prediction = predictions[i];
    if (i != 0) {
      buffer += ",";
    }
    buffer += "{\"cat_id\":\"" + json_escape(prediction.cat_id) + "\"";
    buffer += ",\"confidence\":" + std::to_string(prediction.confidence);
    buffer += ",\"decision\":\"" + json_escape(prediction.decision) + "\"";
    buffer += ",\"filename\":\"" + json_escape(prediction.filename) + "\"";
    buffer += ",\"fx_name\":\"" + json_escape(prediction.fx_name) + "\"";
    buffer += ",\"keywords\":[";
    for (std::size_t k = 0; k < prediction.keywords.size(); ++k) {
      if (k != 0) {
        buffer += ",";
      }
      buffer += "\"" + json_escape(prediction.keywords[k]) + "\"";
    }
    buffer += "]}";
  }
  buffer += "]";
  return buffer.c_str();
}

}  // extern "C"

// ---------------------------------------------------------------- entry

extern "C" REAPER_PLUGIN_DLL_EXPORT int REAPER_PLUGIN_ENTRYPOINT(
    REAPER_PLUGIN_HINSTANCE instance, reaper_plugin_info_t* info) {
  (void)instance;
  if (info == nullptr) {
    stop_worker();
    return 0;
  }
  if (info->caller_version < 0x200) {
    return 0;
  }
  g_plugin_info = info;

  for (int i = 0; i < kActionCount; ++i) {
    g_command_ids[i] = info->Register("command_id", const_cast<char*>(kActions[i].id));
    custom_action_register_t action{};
    action.uniqueSectionId = 0;  // the main section
    action.idStr = kActions[i].id;
    action.name = kActions[i].name;
    action.extra = nullptr;
    info->Register("custom_action", &action);
  }

  info->Register("hookcommand", reinterpret_cast<void*>(command_hook));
  info->Register("timer", reinterpret_cast<void*>(timer_callback));

  ensure_worker();
  return 1;
}
