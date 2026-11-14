// Undo: one block per batch, so a 20-item tag is a single Ctrl+Z.
//
// The REAPER symbols are resolved at load time through the SDK's function table, which
// is why this file includes the plugin header and nothing else does.

#include "slate_reaper.h"

#include "reaper_plugin.h"

extern reaper_plugin_info_t* g_plugin_info;

namespace slate {
namespace {

// The SDK exposes Undo_BeginBlock2 through the function table. Resolving it once at
// first use keeps the entry point free of a per-call lookup.
using BeginBlock = void (*)(ReaProject*);
using EndBlock = void (*)(ReaProject*, const char*, int);

BeginBlock begin_block() {
  if (g_plugin_info == nullptr || g_plugin_info->GetFunc == nullptr) {
    return nullptr;
  }
  return reinterpret_cast<BeginBlock>(g_plugin_info->GetFunc("Undo_BeginBlock2"));
}

EndBlock end_block() {
  if (g_plugin_info == nullptr || g_plugin_info->GetFunc == nullptr) {
    return nullptr;
  }
  return reinterpret_cast<EndBlock>(g_plugin_info->GetFunc("Undo_EndBlock2"));
}

}  // namespace

UndoBlock::UndoBlock() {
  BeginBlock begin = begin_block();
  if (begin == nullptr) {
    return;
  }
  begin(nullptr);
  open_ = true;
}

UndoBlock::~UndoBlock() {
  if (!open_) {
    return;
  }
  EndBlock end = end_block();
  if (end != nullptr) {
    end(nullptr, "Slate-To-UCS: tag", -1);
  }
}

}  // namespace slate
