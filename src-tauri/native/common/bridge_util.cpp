#include "common/bridge_util.h"

#include <cstdlib>
#include <cstring>

namespace bridge_util {

std::string json_escape(const char* s) {
  std::string out;
  if (!s) return out;
  for (const unsigned char* p = reinterpret_cast<const unsigned char*>(s); *p; ++p) {
    if (*p == '"' || *p == '\\') out += '\\';
    if (*p == '\n' || *p == '\r') continue;
    out += static_cast<char>(*p);
  }
  return out;
}

std::string json_escape_str(const std::string& s) { return json_escape(s.c_str()); }

std::string json_escape_detail_for_json(const std::string& s) {
  std::string flat;
  flat.reserve(s.size() + 8);
  for (char c : s) {
    if (c == '\n') {
      flat += " | ";
    } else if (c != '\r') {
      flat += c;
    }
  }
  return json_escape(flat.c_str());
}

char* dup_str(const std::string& s) {
  char* p = static_cast<char*>(std::malloc(s.size() + 1));
  if (!p) return nullptr;
  std::memcpy(p, s.c_str(), s.size() + 1);
  return p;
}

std::string tail_text(const std::string& s, size_t max_chars) {
  if (s.size() <= max_chars) return s;
  return s.substr(s.size() - max_chars);
}

std::string firmware_compiled_date(const char* fw_line) {
  if (!fw_line) return {};
  std::string line(fw_line);
  auto pos = line.find("compiled ");
  if (pos == std::string::npos) return {};
  std::string date = line.substr(pos + 9);
  while (!date.empty() && (date.back() == '\r' || date.back() == '\n')) date.pop_back();
  return date;
}

} // namespace bridge_util
