#pragma once

#include <cstddef>
#include <string>

namespace bridge_util {

std::string json_escape(const char* s);
std::string json_escape_str(const std::string& s);
std::string json_escape_detail_for_json(const std::string& s);
char* dup_str(const std::string& s);
std::string tail_text(const std::string& s, size_t max_chars);
std::string firmware_compiled_date(const char* fw_line);

} // namespace bridge_util
