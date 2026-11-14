// JSON in and out, hand-written so the extension has no dependency beyond the C++
// runtime. Every parser here is total: malformed input returns false, never throws.

#include "slate_reaper.h"

#include <cstdlib>

namespace slate {
namespace {

// Skip whitespace, which JSON allows between every token.
std::size_t skip_ws(const std::string& text, std::size_t at) {
  while (at < text.size()) {
    const char c = text[at];
    if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
      break;
    }
    ++at;
  }
  return at;
}

// Read a JSON string starting at the opening quote. Returns the position after the
// closing quote, or npos on malformed input.
std::size_t read_string(const std::string& text, std::size_t at, std::string* out) {
  if (at >= text.size() || text[at] != '"') {
    return std::string::npos;
  }
  ++at;
  out->clear();
  while (at < text.size()) {
    const char c = text[at];
    if (c == '"') {
      return at + 1;
    }
    if (c == '\\') {
      if (at + 1 >= text.size()) {
        return std::string::npos;
      }
      const char esc = text[at + 1];
      switch (esc) {
        case '"': out->push_back('"'); break;
        case '\\': out->push_back('\\'); break;
        case '/': out->push_back('/'); break;
        case 'b': out->push_back('\b'); break;
        case 'f': out->push_back('\f'); break;
        case 'n': out->push_back('\n'); break;
        case 'r': out->push_back('\r'); break;
        case 't': out->push_back('\t'); break;
        case 'u': {
          if (at + 5 >= text.size()) {
            return std::string::npos;
          }
          const std::string hex = text.substr(at + 2, 4);
          const unsigned code = static_cast<unsigned>(std::strtoul(hex.c_str(), nullptr, 16));
          // Encode as UTF-8. Surrogate pairs are left as replacement characters, which
          // is honest: a CatID or file name never contains one.
          if (code < 0x80) {
            out->push_back(static_cast<char>(code));
          } else if (code < 0x800) {
            out->push_back(static_cast<char>(0xC0 | (code >> 6)));
            out->push_back(static_cast<char>(0x80 | (code & 0x3F)));
          } else {
            out->push_back(static_cast<char>(0xE0 | (code >> 12)));
            out->push_back(static_cast<char>(0x80 | ((code >> 6) & 0x3F)));
            out->push_back(static_cast<char>(0x80 | (code & 0x3F)));
          }
          at += 4;
          break;
        }
        default:
          return std::string::npos;
      }
      at += 2;
      continue;
    }
    out->push_back(c);
    ++at;
  }
  return std::string::npos;
}

// Read a JSON number, which is all the parser needs for confidence and top_k.
std::size_t read_number(const std::string& text, std::size_t at, double* out) {
  const std::size_t begin = at;
  while (at < text.size()) {
    const char c = text[at];
    if ((c >= '0' && c <= '9') || c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E') {
      ++at;
    } else {
      break;
    }
  }
  if (at == begin) {
    return std::string::npos;
  }
  *out = std::strtod(text.substr(begin, at - begin).c_str(), nullptr);
  return at;
}

// Read a JSON array of strings.
std::size_t read_string_array(const std::string& text, std::size_t at,
                              std::vector<std::string>* out) {
  if (at >= text.size() || text[at] != '[') {
    return std::string::npos;
  }
  out->clear();
  at = skip_ws(text, at + 1);
  if (at < text.size() && text[at] == ']') {
    return at + 1;
  }
  while (at < text.size()) {
    std::string value;
    at = read_string(text, at, &value);
    if (at == std::string::npos) {
      return std::string::npos;
    }
    out->push_back(value);
    at = skip_ws(text, at);
    if (at < text.size() && text[at] == ',') {
      at = skip_ws(text, at + 1);
      continue;
    }
    if (at < text.size() && text[at] == ']') {
      return at + 1;
    }
    return std::string::npos;
  }
  return std::string::npos;
}

// Read one prediction object.
std::size_t read_prediction(const std::string& text, std::size_t at, Prediction* out) {
  if (at >= text.size() || text[at] != '{') {
    return std::string::npos;
  }
  at = skip_ws(text, at + 1);
  while (at < text.size() && text[at] != '}') {
    std::string key;
    at = read_string(text, at, &key);
    if (at == std::string::npos) {
      return std::string::npos;
    }
    at = skip_ws(text, at);
    if (at >= text.size() || text[at] != ':') {
      return std::string::npos;
    }
    at = skip_ws(text, at + 1);

    if (key == "cat_id") {
      at = read_string(text, at, &out->cat_id);
    } else if (key == "decision") {
      at = read_string(text, at, &out->decision);
    } else if (key == "filename") {
      at = read_string(text, at, &out->filename);
    } else if (key == "fx_name") {
      at = read_string(text, at, &out->fx_name);
    } else if (key == "confidence") {
      at = read_number(text, at, &out->confidence);
    } else if (key == "keywords") {
      at = read_string_array(text, at, &out->keywords);
    } else {
      // An unknown key is skipped by value type, so a newer engine can add a field
      // without breaking an older extension.
      if (at < text.size() && text[at] == '"') {
        std::string ignored;
        at = read_string(text, at, &ignored);
      } else if (at < text.size() && text[at] == '[') {
        std::vector<std::string> ignored;
        at = read_string_array(text, at, &ignored);
      } else {
        double ignored = 0.0;
        at = read_number(text, at, &ignored);
      }
    }
    if (at == std::string::npos) {
      return std::string::npos;
    }
    at = skip_ws(text, at);
    if (at < text.size() && text[at] == ',') {
      at = skip_ws(text, at + 1);
    }
  }
  if (at >= text.size()) {
    return std::string::npos;
  }
  return at + 1;
}

}  // namespace

std::string json_escape(const std::string& value) {
  std::string out;
  out.reserve(value.size() + 8);
  for (const char c : value) {
    switch (c) {
      case '"': out += "\\\""; break;
      case '\\': out += "\\\\"; break;
      case '\b': out += "\\b"; break;
      case '\f': out += "\\f"; break;
      case '\n': out += "\\n"; break;
      case '\r': out += "\\r"; break;
      case '\t': out += "\\t"; break;
      default:
        if (static_cast<unsigned char>(c) < 0x20) {
          char buffer[8];
          std::snprintf(buffer, sizeof(buffer), "\\u%04x", static_cast<unsigned char>(c));
          out += buffer;
        } else {
          out.push_back(c);
        }
    }
  }
  return out;
}

std::string predict_body(const std::vector<std::string>& descriptions, int top_k) {
  std::string body = "{\"texts\":[";
  for (std::size_t i = 0; i < descriptions.size(); ++i) {
    if (i != 0) {
      body += ",";
    }
    body += "\"" + json_escape(descriptions[i]) + "\"";
  }
  body += "],\"top_k\":";
  body += std::to_string(top_k);
  body += "}";
  return body;
}

bool parse_predictions(const std::string& body, std::vector<Prediction>* out,
                       std::string* error) {
  out->clear();
  std::size_t at = skip_ws(body, 0);
  if (at >= body.size() || body[at] != '[') {
    *error = "expected a JSON array of predictions";
    return false;
  }
  at = skip_ws(body, at + 1);
  if (at < body.size() && body[at] == ']') {
    return true;
  }
  while (at < body.size()) {
    Prediction prediction;
    at = read_prediction(body, at, &prediction);
    if (at == std::string::npos) {
      *error = "malformed prediction object";
      return false;
    }
    out->push_back(prediction);
    at = skip_ws(body, at);
    if (at < body.size() && body[at] == ',') {
      at = skip_ws(body, at + 1);
      continue;
    }
    if (at < body.size() && body[at] == ']') {
      return true;
    }
    *error = "expected ',' or ']' after a prediction";
    return false;
  }
  *error = "unterminated prediction array";
  return false;
}

bool parse_schema_version(const std::string& body, int* out) {
  const std::string key = "\"schema_version\"";
  const std::size_t found = body.find(key);
  if (found == std::string::npos) {
    return false;
  }
  std::size_t at = skip_ws(body, found + key.size());
  if (at >= body.size() || body[at] != ':') {
    return false;
  }
  at = skip_ws(body, at + 1);
  double value = 0.0;
  if (read_number(body, at, &value) == std::string::npos) {
    return false;
  }
  *out = static_cast<int>(value);
  return true;
}

}  // namespace slate
