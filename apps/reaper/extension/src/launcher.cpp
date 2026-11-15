// Endpoint discovery and the HTTP calls. Only the worker thread calls into this file.

#include "slate_reaper.h"

#include <cstdlib>
#include <cstring>
#include <fstream>
#include <sstream>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment(lib, "ws2_32.lib")
using Socket = SOCKET;
constexpr Socket kInvalid = INVALID_SOCKET;
#else
#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
using Socket = int;
constexpr Socket kInvalid = -1;
#endif

namespace slate {
namespace {

// One-time socket library start-up. On Windows this is required before any call; on
// POSIX it is a no-op.
bool ensure_sockets() {
#ifdef _WIN32
  static bool started = false;
  if (!started) {
    WSADATA data;
    started = WSAStartup(MAKEWORD(2, 2), &data) == 0;
  }
  return started;
#else
  return true;
#endif
}

void close_socket(Socket socket) {
#ifdef _WIN32
  closesocket(socket);
#else
  close(socket);
#endif
}

// Read a whole response, up to a cap, so a hostile or broken server cannot exhaust
// memory on the audio thread's behalf.
bool read_all(Socket socket, std::string* out) {
  constexpr std::size_t kCap = 8u * 1024u * 1024u;
  char buffer[8192];
  while (out->size() < kCap) {
    const int got = static_cast<int>(recv(socket, buffer, sizeof(buffer), 0));
    if (got <= 0) {
      break;
    }
    out->append(buffer, static_cast<std::size_t>(got));
  }
  return !out->empty();
}

// Split an HTTP response into status code and body.
bool split_response(const std::string& raw, int* status, std::string* body) {
  const std::size_t header_end = raw.find("\r\n\r\n");
  if (header_end == std::string::npos) {
    return false;
  }
  const std::size_t space = raw.find(' ');
  if (space == std::string::npos || space + 4 > raw.size()) {
    return false;
  }
  *status = std::atoi(raw.substr(space + 1, 3).c_str());
  *body = raw.substr(header_end + 4);
  return true;
}

// One request. `method` is "GET" or "POST"; `body` is ignored for GET.
bool request(const Endpoint& endpoint, const char* method, const std::string& path,
             const std::string& body, std::string* response, std::string* error) {
  if (!endpoint.valid()) {
    *error = "no engine endpoint";
    return false;
  }
  if (!ensure_sockets()) {
    *error = "socket library did not start";
    return false;
  }

  addrinfo hints;
  std::memset(&hints, 0, sizeof(hints));
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;
  addrinfo* resolved = nullptr;
  const std::string port = std::to_string(endpoint.port);
  if (getaddrinfo(endpoint.host.c_str(), port.c_str(), &hints, &resolved) != 0 ||
      resolved == nullptr) {
    *error = "cannot resolve " + endpoint.host;
    return false;
  }

  Socket socket = kInvalid;
  for (addrinfo* candidate = resolved; candidate != nullptr; candidate = candidate->ai_next) {
    socket = ::socket(candidate->ai_family, candidate->ai_socktype, candidate->ai_protocol);
    if (socket == kInvalid) {
      continue;
    }
    if (::connect(socket, candidate->ai_addr, static_cast<int>(candidate->ai_addrlen)) == 0) {
      break;
    }
    close_socket(socket);
    socket = kInvalid;
  }
  freeaddrinfo(resolved);
  if (socket == kInvalid) {
    *error = "cannot connect to " + endpoint.host + ":" + port;
    return false;
  }

  std::ostringstream head;
  head << method << " " << path << " HTTP/1.1\r\n"
       << "Host: " << endpoint.host << ":" << port << "\r\n"
       << "Connection: close\r\n"
       << "Authorization: Bearer " << endpoint.token << "\r\n";
  if (std::strcmp(method, "POST") == 0) {
    head << "Content-Type: application/json\r\n"
         << "Content-Length: " << body.size() << "\r\n";
  }
  head << "\r\n";
  const std::string request_text = head.str() + body;

  std::size_t sent = 0;
  while (sent < request_text.size()) {
    const int wrote = static_cast<int>(
        send(socket, request_text.data() + sent, static_cast<int>(request_text.size() - sent), 0));
    if (wrote <= 0) {
      close_socket(socket);
      *error = "connection closed while sending";
      return false;
    }
    sent += static_cast<std::size_t>(wrote);
  }

  std::string raw;
  const bool read_ok = read_all(socket, &raw);
  close_socket(socket);
  if (!read_ok) {
    *error = "empty response";
    return false;
  }

  int status = 0;
  if (!split_response(raw, &status, response)) {
    *error = "malformed HTTP response";
    return false;
  }
  if (status < 200 || status >= 300) {
    *error = "HTTP " + std::to_string(status);
    return false;
  }
  return true;
}

// The platform cache directory the engine writes serve.json into when TTUCS_HOME is
// unset. Kept in step with ttucs.serve.endpoint_path.
std::string default_home() {
  const char* home = std::getenv("TTUCS_HOME");
  if (home != nullptr && *home != '\0') {
    return home;
  }
#ifdef _WIN32
  const char* local = std::getenv("LOCALAPPDATA");
  return local != nullptr ? std::string(local) + "\\ttucs" : std::string(".");
#elif defined(__APPLE__)
  const char* home_dir = std::getenv("HOME");
  return home_dir != nullptr ? std::string(home_dir) + "/Library/Caches/ttucs" : std::string(".");
#else
  const char* xdg = std::getenv("XDG_CACHE_HOME");
  if (xdg != nullptr && *xdg != '\0') {
    return std::string(xdg) + "/ttucs";
  }
  const char* home_dir = std::getenv("HOME");
  return home_dir != nullptr ? std::string(home_dir) + "/.cache/ttucs" : std::string(".");
#endif
}

// Pull a JSON string field out of a flat object, which is all serve.json is.
bool field(const std::string& text, const char* key, std::string* out) {
  const std::string needle = std::string("\"") + key + "\"";
  const std::size_t found = text.find(needle);
  if (found == std::string::npos) {
    return false;
  }
  const std::size_t colon = text.find(':', found + needle.size());
  if (colon == std::string::npos) {
    return false;
  }
  const std::size_t quote = text.find('"', colon + 1);
  if (quote == std::string::npos) {
    return false;
  }
  const std::size_t end = text.find('"', quote + 1);
  if (end == std::string::npos) {
    return false;
  }
  *out = text.substr(quote + 1, end - quote - 1);
  return true;
}

}  // namespace

bool read_endpoint(Endpoint* out, std::string* error) {
  const std::string path = default_home() + "/serve.json";
  std::ifstream file(path.c_str(), std::ios::binary);
  if (!file) {
    *error = "no endpoint file at " + path;
    return false;
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  const std::string text = buffer.str();

  std::string port;
  if (!field(text, "port", &port)) {
    *error = "endpoint file has no port";
    return false;
  }
  out->port = std::atoi(port.c_str());
  std::string host;
  if (field(text, "host", &host)) {
    out->host = host;
  }
  std::string token;
  if (field(text, "token", &token)) {
    out->token = token;
  }
  if (!out->valid()) {
    *error = "endpoint file has no usable port";
    return false;
  }
  return true;
}

LaunchStep connect_or_spawn(Endpoint* out, std::string* error) {
  if (read_endpoint(out, error)) {
    std::string body;
    std::string health_error;
    if (http_get(*out, "/health", &body, &health_error)) {
      int version = 0;
      if (parse_schema_version(body, &version) && version == kSchemaVersion) {
        return LaunchStep::Attach;
      }
      *error = "engine speaks schema_version " + std::to_string(version) + ", expected " +
               std::to_string(kSchemaVersion);
      return LaunchStep::Failed;
    }
  }
  // Spawning is the CLI's job: it already owns the three steps, the timeouts and the
  // token file, and duplicating that here would be a second implementation to keep in
  // step. The extension asks the user to run `slate doctor` instead of guessing.
  *error = "no engine is running; start one with `slate doctor` or launch the app";
  return LaunchStep::Failed;
}

bool http_post(const Endpoint& endpoint, const std::string& path, const std::string& body,
               std::string* response, std::string* error) {
  return request(endpoint, "POST", path, body, response, error);
}

bool http_get(const Endpoint& endpoint, const std::string& path, std::string* response,
              std::string* error) {
  return request(endpoint, "GET", path, std::string(), response, error);
}

}  // namespace slate
