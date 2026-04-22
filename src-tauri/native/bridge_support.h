#pragma once

#include "JLinkARMDLL_Wrapper.h"

#include <memory>
#include <mutex>
#include <string>

namespace bridge_state {

extern std::mutex g_mu;
extern std::unique_ptr<JLinkARMDLL> g_api;
extern std::string g_err;
extern std::string g_dll_dir;

void set_err(std::string s);
JLinkARMDLL* api_or_set_err();

} // namespace bridge_state

namespace bridge_util {

std::string json_escape(const char* s);
std::string json_escape_str(const std::string& s);
std::string json_escape_detail_for_json(const std::string& s);

char* dup_str(const std::string& s);

std::string tail_text(const std::string& s, size_t max_chars);
std::string firmware_compiled_date(const char* fw_line);

} // namespace bridge_util
