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

