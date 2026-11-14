// Note merging: the one place the extension writes text a user will read.

#include "slate_reaper.h"

#include <cstdio>

namespace slate {
namespace {

constexpr const char* kPrefix = "slate:";

// The line ending the existing text uses, so a merge never mixes CRLF and LF.
std::string dominant_eol(const std::string& text) {
  return text.find("\r\n") != std::string::npos ? "\r\n" : "\n";
}

}  // namespace

std::string merge_notes(const std::string& existing, const std::string& slate_line) {
  const std::string eol = dominant_eol(existing);

  std::vector<std::string> lines;
  std::size_t start = 0;
  while (start <= existing.size()) {
    const std::size_t nl = existing.find('\n', start);
    if (nl == std::string::npos) {
      if (start < existing.size()) {
        lines.push_back(existing.substr(start));
      }
      break;
    }
    std::string line = existing.substr(start, nl - start);
    if (!line.empty() && line.back() == '\r') {
      line.pop_back();
    }
    lines.push_back(line);
    start = nl + 1;
  }

  bool replaced = false;
  for (std::string& line : lines) {
    if (line.rfind(kPrefix, 0) == 0) {
      line = slate_line;
      replaced = true;
      break;
    }
  }
  if (!replaced) {
    lines.push_back(slate_line);
  }

  std::string out;
  for (std::size_t i = 0; i < lines.size(); ++i) {
    if (i != 0) {
      out += eol;
    }
    out += lines[i];
  }
  return out;
}

std::string slate_line_for(const Prediction& prediction) {
  char buffer[64];
  std::snprintf(buffer, sizeof(buffer), "%.3f", prediction.confidence);
  std::string line = std::string(kPrefix) + " " + prediction.cat_id + " " + buffer;
  if (!prediction.filename.empty()) {
    line += " " + prediction.filename;
  }
  return line;
}

}  // namespace slate
