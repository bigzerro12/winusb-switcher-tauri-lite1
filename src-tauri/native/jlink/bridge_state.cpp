#include "jlink/bridge_state.h"

namespace bridge_state {

std::mutex g_mu;
std::unique_ptr<JLinkARMDLL> g_api;
std::string g_err;
std::string g_dll_dir;

void set_err(std::string s) { g_err = std::move(s); }

JLinkARMDLL* api_or_set_err() {
  if (!g_api) {
    set_err("J-Link API not loaded");
    return nullptr;
  }
  return g_api.get();
}

} // namespace bridge_state
