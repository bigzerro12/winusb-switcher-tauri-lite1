#include "jlink_bridge.h"

#include "bridge_support.h"
#include "commander_exec.h"
#include "runtime_dirs.h"

#include <cstdlib>
#include <sstream>
#include <string>
#include <vector>

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <Windows.h>
#endif

using bridge_state::api_or_set_err;
using bridge_state::g_api;
using bridge_state::g_dll_dir;
using bridge_state::g_err;
using bridge_state::g_mu;
using bridge_state::set_err;

void jlink_bridge_free_string(char* s) { std::free(s); }

const char* jlink_bridge_last_error(void) { return g_err.c_str(); }

int jlink_bridge_load(const char* dll_path_utf8) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  g_api = std::make_unique<JLinkARMDLL>();
  if (!dll_path_utf8 || !dll_path_utf8[0]) {
    set_err("empty DLL path");
    g_api.reset();
    return -1;
  }
  if (!g_api->Load(dll_path_utf8)) {
    set_err(g_api->lastError().empty() ? std::string("Load failed") : g_api->lastError());
    g_api.reset();
    return -1;
  }

  std::string lp = g_api->loadedPath();
  if (lp.empty()) lp = dll_path_utf8;
  g_dll_dir = runtime_dirs::dirname_utf8(lp.c_str());
  runtime_dirs::apply_jlink_runtime_dirs(g_dll_dir);
  return 0;
}

void jlink_bridge_unload(void) {
  std::lock_guard<std::mutex> lock(g_mu);
#ifdef _WIN32
  SetDllDirectory(nullptr);
#endif
  if (g_api) g_api->Unload();
  g_api.reset();
  g_err.clear();
  g_dll_dir.clear();
}

int jlink_bridge_is_loaded(void) {
  std::lock_guard<std::mutex> lock(g_mu);
  return g_api ? 1 : 0;
}

char* jlink_bridge_list_probes_json(void) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

  std::vector<JLINKARM_EMU_CONNECT_INFO> list;
  const int n = commander_exec::_ExecShowEmuList(*a, list);
  if (n < 0) {
    set_err("JLINKARM_EMU_GetList failed");
    return nullptr;
  }

  std::ostringstream oss;
  oss << "[";
  for (int i = 0; i < n; ++i) {
    if (i) oss << ',';
    const auto& e = list[static_cast<size_t>(i)];
    const char* conn =
        e.Connection == JLINKARM_HOSTIF_USB ? "USB" : (e.Connection == JLINKARM_HOSTIF_IP ? "IP" : "Unknown");
    oss << "{\"index\":" << i << ",\"serialNumber\":\"" << e.SerialNumber << "\",\"connection\":\"" << conn
        << "\",\"nickName\":\"" << bridge_util::json_escape(e.acNickName) << "\",\"productName\":\""
        << bridge_util::json_escape(e.acProduct) << "\",\"discoveryFirmware\":\"" << bridge_util::json_escape(e.acFWString)
        << "\"}";
  }
  oss << "]";
  return bridge_util::dup_str(oss.str());
}

char* jlink_bridge_probe_firmware(int index) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

  std::vector<JLINKARM_EMU_CONNECT_INFO> list;
  const int n = commander_exec::_ExecShowEmuList(*a, list);
  if (n < 0) {
    set_err("JLINKARM_EMU_GetList failed");
    return nullptr;
  }
  if (index < 0 || index >= n) {
    set_err("probe index out of range");
    return nullptr;
  }

#ifdef _WIN32
  runtime_dirs::ScopedCwd _cwd_probe(g_dll_dir);
#endif

  std::string open_cap;
  std::string open_err_s;
  if (!commander_exec::_ConnectToJLinkCapture(*a, index, list, open_cap, open_err_s)) {
    set_err(open_err_s);
    return nullptr;
  }
  if (!commander_exec::_EnsureSelectedUsbSn(*a, index, list, open_cap, open_err_s, /*diag=*/nullptr)) {
    set_err(open_err_s);
    return nullptr;
  }

  char fw[512] = {};
  a->JLINKARM_GetFirmwareString(fw, static_cast<int>(sizeof(fw) - 1));
  commander_exec::_DisconnectFromJLink(*a);

  std::string out = bridge_util::firmware_compiled_date(fw);
  if (out.empty() && fw[0]) out = fw;
  return bridge_util::dup_str(out);
}

char* jlink_bridge_update_firmware(int index) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

#ifdef _WIN32
  runtime_dirs::ScopedCwd _cwd(g_dll_dir);
#endif

  std::vector<JLINKARM_EMU_CONNECT_INFO> list;
  const int n = commander_exec::_ExecShowEmuList(*a, list);
  if (n < 0 || index < 0 || index >= n) {
    set_err("invalid probe list or index");
    return nullptr;
  }
  const auto& sel = list[static_cast<size_t>(index)];

  std::ostringstream env_diag;
#ifdef _WIN32
  {
    const std::string cwd_now = runtime_dirs::get_current_directory_a();
    const std::string bin_name = commander_exec::_GuessFirmwareBinName(sel);
    std::string fw_path = g_dll_dir + "\\Firmwares\\" + bin_name;
    env_diag << "dll_dir=" << g_dll_dir << "\n"
             << "cwd_during_update=" << cwd_now << "\n"
             << "expected_firmware_file=" << fw_path << "\n"
             << "firmware_file_exists=" << (runtime_dirs::file_exists_a(fw_path) ? "yes" : "no") << "\n";
  }
#endif

  std::string open_cap;
  std::string open_err_s;
  if (!commander_exec::_ConnectToJLinkCapture(*a, index, list, open_cap, open_err_s)) {
    set_err(open_err_s);
    return nullptr;
  }

  char fw_before[512] = {};
  a->JLINKARM_GetFirmwareString(fw_before, static_cast<int>(sizeof(fw_before) - 1));
  std::string fw_before_s = bridge_util::firmware_compiled_date(fw_before);
  if (fw_before_s.empty() && fw_before[0]) fw_before_s = fw_before;

  if (!commander_exec::_EnsureSelectedUsbSn(*a, index, list, open_cap, open_err_s, &env_diag)) {
    set_err(open_err_s);
    return nullptr;
  }

  bool updated = commander_exec::_CallbackLogSuggestsFirmwareActivity(open_cap);
  std::string out_all = open_cap;

  U32 rc = 0;
  const std::string cap_update = commander_exec::_CaptureUpdateFirmwareIfNewer(*a, &rc);
  env_diag << "JLINKARM_UpdateFirmwareIfNewer rc=" << static_cast<unsigned long>(rc) << "\n";
  if (!cap_update.empty()) {
    if (!out_all.empty()) out_all += "\n";
    out_all += cap_update;
  }
  updated = updated || commander_exec::_CallbackLogSuggestsFirmwareActivity(cap_update);
  updated = updated || (rc != 0);

  char fw[512] = {};
  a->JLINKARM_GetFirmwareString(fw, static_cast<int>(sizeof(fw) - 1));
  commander_exec::_DisconnectFromJLink(*a);

  std::string fw_after_s = bridge_util::firmware_compiled_date(fw);
  if (fw_after_s.empty() && fw[0]) fw_after_s = fw;
  updated = updated || (!fw_after_s.empty() && fw_after_s != fw_before_s);

  // If the probe rebooted mid-update, do a short reopen + re-read loop.
  if (fw_after_s == fw_before_s) {
    for (int i = 0; i < 6; ++i) {
      commander_exec::_ExecSleep(250);
      std::string retry_cap;
      std::string retry_err;
      if (!commander_exec::_ConnectToJLinkCapture(*a, index, list, retry_cap, retry_err)) continue;
      char tmp[512] = {};
      a->JLINKARM_GetFirmwareString(tmp, static_cast<int>(sizeof(tmp) - 1));
      commander_exec::_DisconnectFromJLink(*a);
      std::string tmp_s = bridge_util::firmware_compiled_date(tmp);
      if (tmp_s.empty() && tmp[0]) tmp_s = tmp;
      if (!tmp_s.empty()) fw_after_s = tmp_s;
      if (fw_after_s != fw_before_s) break;
    }
    updated = updated || (!fw_after_s.empty() && fw_after_s != fw_before_s);
  }

  bool reboot_attempted = false;
  bool reboot_not_supported = false;
  std::string reboot_command;
  if (updated) {
    commander_exec::_ExecSleep(100);
    const auto rr = commander_exec::_ExecReboot(*a, index, list);
    commander_exec::_ExecSleep(100);
    reboot_attempted = rr.attempted;
    reboot_not_supported = rr.not_supported;
    reboot_command = rr.command;
  }

  std::ostringstream oss;
  const std::string detail =
      env_diag.str() +
      std::string("post_update_sleep_ms=") + (updated ? "100" : "0") +
      "\npost_update_reboot_attempted=" + std::string(reboot_attempted ? "yes" : "no") +
      "\npost_update_reboot_command=" + reboot_command +
      "\npost_update_reboot_not_supported=" + std::string(reboot_not_supported ? "yes" : "no") +
      "\n" +
      std::string("Index=") + std::to_string(index) +
      "\nSerial=" + std::to_string(sel.SerialNumber) +
      "\nProduct=" + std::string(sel.acProduct ? sel.acProduct : "") +
      "\nNickname=" + std::string(sel.acNickName ? sel.acNickName : "") +
      "\nBefore=" + fw_before_s +
      "\nAfter=" + fw_after_s +
      (out_all.empty() ? std::string() : (std::string("\n\n[DLL callback log]\n") + bridge_util::tail_text(out_all, 2000)));

  oss << "{\"status\":\"" << (updated ? "updated" : "current") << "\",\"firmware\":\""
      << bridge_util::json_escape(fw_after_s.c_str()) << "\",\"beforeFirmware\":\""
      << bridge_util::json_escape(fw_before_s.c_str()) << "\",\"serialNumber\":\""
      << sel.SerialNumber << "\",\"detail\":\"" << bridge_util::json_escape_detail_for_json(detail) << "\",\"error\":\"\""
      << ",\"rebootNotSupported\":" << (reboot_not_supported ? "true" : "false")
      << ",\"rebootAttempted\":" << (reboot_attempted ? "true" : "false")
      << ",\"rebootCommand\":\"" << bridge_util::json_escape(reboot_command.c_str()) << "\""
      << ",\"sleepMs\":" << (updated ? "100" : "0")
      << "}";
  return bridge_util::dup_str(oss.str());
}

char* jlink_bridge_switch_usb_driver(int index, int winusb) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

#ifdef _WIN32
  runtime_dirs::ScopedCwd _cwd(g_dll_dir);
#endif

  std::vector<JLINKARM_EMU_CONNECT_INFO> list;
  const int n = commander_exec::_ExecShowEmuList(*a, list);
  if (n < 0 || index < 0 || index >= n) {
    set_err("invalid probe list or index");
    return bridge_util::dup_str("{\"success\":false,\"error\":\"invalid index\",\"detail\":\"\",\"rebootNotSupported\":false}");
  }

  std::string detail_for_error;
  const bool ok = winusb
      ? commander_exec::_ExecWebUSBEnable(*a, index, list, detail_for_error)
      : commander_exec::_ExecWebUSBDisable(*a, index, list, detail_for_error);
  if (!ok) {
    set_err(detail_for_error.empty() ? std::string("switch config failed") : detail_for_error);
    std::ostringstream oss;
    oss << "{\"success\":false,\"error\":\"Failed to switch probe config.\",\"detail\":\""
        << bridge_util::json_escape_str(detail_for_error) << "\",\"rebootNotSupported\":false}";
    return bridge_util::dup_str(oss.str());
  }

  commander_exec::_ExecSleep(100);
  const auto rr = commander_exec::_ExecReboot(*a, index, list);
  commander_exec::_ExecSleep(100);

  std::ostringstream oss;
  oss << "{\"success\":true,\"error\":\"\",\"detail\":\"\",\"rebootNotSupported\":"
      << (rr.not_supported ? "true" : "false")
      << ",\"rebootAttempted\":" << (rr.attempted ? "true" : "false")
      << ",\"rebootCommand\":\"" << bridge_util::json_escape(rr.command.c_str()) << "\""
      << ",\"sleepMs\":100}";
  return bridge_util::dup_str(oss.str());
}

char* jlink_bridge_dll_version_string(void) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;
  const U32 v = a->JLINKARM_GetDLLVersion();
  if (v == 0) return nullptr;
  std::ostringstream oss;
  oss << "SEGGER J-Link DLL (build " << v << ")";
  return bridge_util::dup_str(oss.str());
}

