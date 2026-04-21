#include "jlink_bridge.h"

#include "JLinkARMDLL_Wrapper.h"

#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <mutex>
#include <memory>
#include <sstream>
#include <string>
#include <thread>
#include <chrono>
#include <vector>

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <Windows.h>
#else
#  include <unistd.h>
#  include <limits.h>
#endif

namespace {

std::mutex g_mu;
std::unique_ptr<JLinkARMDLL> g_api;
std::string g_err;
std::string g_dll_dir;

// Many J-Link commands write output via the log/error callbacks rather than the
// `pOut` buffer of JLINKARM_ExecCommand(). Capture callback output so we can
// parse success reliably (mirrors what you see in J-Link Commander).
thread_local std::string* g_capture = nullptr;

// Match WinUSBSwitcher demo (main.cpp): stream DLL log/error to stderr so `tauri dev` shows SEGGER text.
static void log_cb(const char* s) {
  if (s) {
    std::fputs(s, stderr);
#ifdef _WIN32
    OutputDebugStringA(s);
#endif
  }
  if (g_capture && s) g_capture->append(s);
}

static void err_cb(const char* s) {
  if (s) {
    std::fputs(s, stderr);
#ifdef _WIN32
    OutputDebugStringA(s);
#endif
  }
  if (g_capture && s) g_capture->append(s);
}

static void set_err(std::string s) {
  g_err = std::move(s);
}

static void sleep_ms(unsigned ms) {
#ifdef _WIN32
  ::Sleep(ms);
#else
  std::this_thread::sleep_for(std::chrono::milliseconds(ms));
#endif
}

#ifdef _WIN32
static std::string dirname_utf8(const char* path) {
  if (!path) return {};
  std::string p(path);
  // Normalize to backslashes for Windows APIs (optional).
  for (char& c : p) {
    if (c == '/') c = '\\';
  }
  const size_t pos = p.find_last_of("\\");
  if (pos == std::string::npos) return {};
  return p.substr(0, pos);
}

// Some Windows APIs reject `\\?\` extended paths for SetCurrentDirectoryA; retry without the prefix.
static bool set_current_directory_best_effort(const std::string& new_dir) {
  if (new_dir.empty()) return false;
  if (SetCurrentDirectoryA(new_dir.c_str())) return true;
  if (new_dir.size() >= 4 && new_dir[0] == '\\' && new_dir[1] == '\\' && new_dir[2] == '?' && new_dir[3] == '\\') {
    return SetCurrentDirectoryA(new_dir.substr(4).c_str()) != 0;
  }
  return false;
}

// WinUSBSwitcher main.cpp: after LoadLibrary, set process CWD + DLL search dir to the folder containing
// the loaded module (so `Firmwares\\` resolves like J-Link.exe). Uses resolved `loadedPath()`, not only
// the path string passed to Load.
static void win32_apply_demo_runtime_dirs(const std::string& dirRaw) {
  if (dirRaw.empty()) return;
  std::string dir = dirRaw;
  for (char& c : dir) {
    if (c == '/') c = '\\';
  }
  set_current_directory_best_effort(dir);
  if (dir.size() >= 4 && dir[0] == '\\' && dir[1] == '\\' && dir[2] == '?' && dir[3] == '\\') {
    dir = dir.substr(4);
  }
  SetDllDirectoryA(dir.c_str());
}

struct ScopedCwd {
  char old[MAX_PATH] = {};
  bool changed = false;
  explicit ScopedCwd(const std::string& new_dir) {
    if (new_dir.empty()) return;
    DWORD n = GetCurrentDirectoryA(static_cast<DWORD>(sizeof(old)), old);
    (void)n;
    if (set_current_directory_best_effort(new_dir)) {
      changed = true;
    }
  }
  ~ScopedCwd() {
    if (changed) {
      SetCurrentDirectoryA(old);
    }
  }
};

// Helps `JLink_x64.dll` locate `Firmwares\\` the same way as J-Link.exe (search path, not only CWD).
struct ScopedDllDirectory {
  bool active = false;
  explicit ScopedDllDirectory(const std::string& dll_dir) {
    if (dll_dir.empty()) return;
    std::string d = dll_dir;
    if (d.size() >= 4 && d[0] == '\\' && d[1] == '\\' && d[2] == '?' && d[3] == '\\') {
      d = d.substr(4);
    }
    active = SetDllDirectoryA(d.c_str()) != 0;
  }
  ~ScopedDllDirectory() {
    if (active) {
      SetDllDirectoryA(nullptr);
    }
  }
};

static std::string get_current_directory_a() {
  char buf[MAX_PATH * 4] = {};
  DWORD n = GetCurrentDirectoryA(static_cast<DWORD>(sizeof(buf)), buf);
  if (!n || n >= sizeof(buf)) return {};
  return std::string(buf);
}

static bool file_exists_a(const std::string& path) {
  if (path.empty()) return false;
  const DWORD a = GetFileAttributesA(path.c_str());
  return a != INVALID_FILE_ATTRIBUTES && (a & FILE_ATTRIBUTE_DIRECTORY) == 0;
}
#else
static std::string dirname_utf8(const char* path) {
  if (!path) return {};
  std::string p(path);
  const size_t pos = p.find_last_of('/');
  if (pos == std::string::npos) return {};
  return p.substr(0, pos);
}

static void posix_apply_demo_runtime_dir(const std::string& dir) {
  if (dir.empty()) return;
  (void)chdir(dir.c_str());
}

struct ScopedCwd {
  std::string old;
  bool changed = false;
  explicit ScopedCwd(const std::string& new_dir) {
    if (new_dir.empty()) return;
    char buf[PATH_MAX] = {};
    if (getcwd(buf, sizeof(buf))) old = buf;
    if (chdir(new_dir.c_str()) == 0) {
      changed = true;
    }
  }
  ~ScopedCwd() {
    if (changed && !old.empty()) {
      (void)chdir(old.c_str());
    }
  }
};

[[maybe_unused]] static std::string get_current_directory_a() {
  char buf[PATH_MAX] = {};
  if (!getcwd(buf, sizeof(buf))) return {};
  return std::string(buf);
}

[[maybe_unused]] static bool file_exists_a(const std::string& path) {
  if (path.empty()) return false;
  return access(path.c_str(), R_OK) == 0;
}
#endif

static JLinkARMDLL* api_or_set_err() {
  if (!g_api) {
    set_err("J-Link API not loaded");
    return nullptr;
  }
  return g_api.get();
}

static std::string json_escape(const char* s) {
  std::string out;
  if (!s) return out;
  for (const unsigned char* p = reinterpret_cast<const unsigned char*>(s); *p; ++p) {
    if (*p == '"' || *p == '\\') out += '\\';
    if (*p == '\n' || *p == '\r') continue;
    out += static_cast<char>(*p);
  }
  return out;
}

static std::string json_escape_str(const std::string& s) {
  return json_escape(s.c_str());
}

// json_escape() drops newlines, which makes multi-line `detail` unreadable in logs; flatten first.
static std::string json_escape_detail_for_json(const std::string& s) {
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

static char* dup_str(const std::string& s) {
  char* p = static_cast<char*>(std::malloc(s.size() + 1));
  if (!p) return nullptr;
  std::memcpy(p, s.c_str(), s.size() + 1);
  return p;
}

static std::vector<JLINKARM_EMU_CONNECT_INFO> fetch_probe_list(JLinkARMDLL& a, int* out_count) {
  std::vector<JLINKARM_EMU_CONNECT_INFO> buf(64);
  int n = a.JLINKARM_EMU_GetList(
      static_cast<int>(JLINKARM_HOSTIF_USB | JLINKARM_HOSTIF_IP),
      buf.data(),
      static_cast<int>(buf.size()));
  if (n < 0) {
    *out_count = -1;
    return {};
  }
  if (n > static_cast<int>(buf.size())) n = static_cast<int>(buf.size());
  buf.resize(static_cast<size_t>(n));
  *out_count = n;
  return buf;
}

static bool select_probe(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list) {
  if (index < 0 || index >= static_cast<int>(list.size())) return false;
  const auto& sel = list[static_cast<size_t>(index)];
  // Same order as SEGGER WinUSBSwitcher / Main.c: SelectByIndex first; if it fails (==0), use USBSN.
  if (a.JLINKARM_EMU_SelectByIndex(static_cast<U32>(index)) != 0) {
    return true;
  }
  if (sel.Connection == JLINKARM_HOSTIF_USB) {
    if (a.JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) >= 0) {
      return true;
    }
  }
  return false;
}

// For passive FW reads / table scan: with multiple USB probes, SelectByIndex can attach to the wrong
// unit. Prefer USB serial selection first (matches update_firmware SN-mismatch fix path).
static bool select_probe_usb_serial_first(JLinkARMDLL& a, int index,
                                          const std::vector<JLINKARM_EMU_CONNECT_INFO>& list) {
  if (index < 0 || index >= static_cast<int>(list.size())) return false;
  const auto& sel = list[static_cast<size_t>(index)];
  if (sel.Connection == JLINKARM_HOSTIF_USB) {
    if (a.JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) >= 0) {
      return true;
    }
  }
  return select_probe(a, index, list);
}

static std::string exec_out(JLinkARMDLL& a, const char* cmd) {
  char out[8192] = {};
  std::string cap;
  g_capture = &cap;
  const int rc = a.JLINKARM_ExecCommand(cmd, out, static_cast<int>(sizeof(out) - 1));
  g_capture = nullptr;

  std::string s(out);
  if (!cap.empty()) {
    if (!s.empty() && s.back() != '\n') s.push_back('\n');
    s += cap;
  }
  // Prefix with rc so we can detect silent failures.
  if (rc != 0) {
    std::ostringstream oss;
    oss << "[ExecCommand rc=" << rc << "] ";
    s = oss.str() + s;
  }
  return s;
}

static bool command_not_supported(const std::string& s) {
  return s.find("Unknown command") != std::string::npos ||
         s.find("Syntax error") != std::string::npos ||
         s.find("not supported") != std::string::npos;
}

static bool write_succeeded(const std::string& s) {
  if (s.find("Probe configured successfully") != std::string::npos) return true;
  std::string lower;
  lower.reserve(s.size());
  for (char c : s) lower += static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
  if (lower.find("probe configured successfully") != std::string::npos) return true;
  if (!command_not_supported(s) &&
      (s.find("WebUSBEnable") != std::string::npos || s.find("WebUSBDisable") != std::string::npos ||
       s.find("WinUSBEnable") != std::string::npos || s.find("WinUSBDisable") != std::string::npos) &&
      (s.find("O.K.") != std::string::npos || s.find("OK") != std::string::npos ||
       s.find("Serial number:") != std::string::npos)) {
    return true;
  }
  return false;
}

static std::string tail_text(const std::string& s, size_t max_chars) {
  if (s.size() <= max_chars) return s;
  return s.substr(s.size() - max_chars);
}

// WinUSBSwitcher demo (main.cpp) never calls JLINKARM_Connect() after OpenEx; the DLL may run an
// automatic firmware update during OpenEx and stream progress via log callbacks ("Updating firmware...").
// We capture that here. Calling Connect() before the first read can change DLL state and skip that path
// on some builds.
static const char* open_ex_capture(JLinkARMDLL& a, std::string& cap) {
  cap.clear();
  g_capture = &cap;
  const char* err = a.JLINKARM_OpenEx(&log_cb, &err_cb);
  g_capture = nullptr;
  return err;
}

static bool callback_log_suggests_firmware_activity(const std::string& s) {
  return s.find("Updating firmware") != std::string::npos ||
         s.find("Replacing firmware") != std::string::npos ||
         s.find("New firmware") != std::string::npos ||
         s.find("Waiting for new firmware") != std::string::npos;
}

[[maybe_unused]] static std::string guess_firmware_bin_name(const JLINKARM_EMU_CONNECT_INFO& e) {
  const char* p = e.acProduct;
  if (!p || !p[0]) return "JLink_OB_S124.bin";
  std::string s(p);
  if (s.find("OB-S124") != std::string::npos) return "JLink_OB_S124.bin";
  if (s.find("OB-S") != std::string::npos) return "JLink_OB_S124.bin";
  return "JLink_OB_S124.bin";
}

[[maybe_unused]] static bool contains_connect_ok(const std::string& s) {
  return s.find("Connecting to J-Link") != std::string::npos &&
         (s.find("O.K.") != std::string::npos || s.find("OK") != std::string::npos);
}

[[maybe_unused]] static bool contains_connect_failed(const std::string& s) {
  return s.find("Connecting to J-Link") != std::string::npos &&
         (s.find("FAILED") != std::string::npos || s.find("Cannot connect") != std::string::npos);
}

static bool contains_unknown_command(const std::string& s) {
  return s.find("Unknown command") != std::string::npos || s.find("ERROR: Unknown command") != std::string::npos;
}

static std::string firmware_compiled_date(const char* fw_line) {
  if (!fw_line) return {};
  std::string line(fw_line);
  auto pos = line.find("compiled ");
  if (pos == std::string::npos) return {};
  std::string date = line.substr(pos + 9);
  while (!date.empty() && (date.back() == '\r' || date.back() == '\n')) date.pop_back();
  return date;
}

} // namespace

void jlink_bridge_free_string(char* s) {
  std::free(s);
}

const char* jlink_bridge_last_error(void) {
  return g_err.c_str();
}

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

  {
    std::string lp = g_api->loadedPath();
    if (lp.empty()) {
      lp = dll_path_utf8;
    }
    g_dll_dir = dirname_utf8(lp.c_str());
#ifdef _WIN32
    win32_apply_demo_runtime_dirs(g_dll_dir);
#else
    // Linux: keep process CWD pointing at the runtime directory so `Firmwares/` is discoverable.
    // (No SetDllDirectory equivalent; library resolution uses LD_LIBRARY_PATH set by Rust code.)
    posix_apply_demo_runtime_dir(g_dll_dir);
#endif
  }

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

  int n = 0;
  auto list = fetch_probe_list(*a, &n);
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
    // acFWString: best-effort firmware from discovery (no OpenEx). USB entries may be empty; IP often filled.
    oss << "{\"index\":" << i << ",\"serialNumber\":\"" << e.SerialNumber << "\",\"connection\":\"" << conn
        << "\",\"nickName\":\"" << json_escape(e.acNickName) << "\",\"productName\":\""
        << json_escape(e.acProduct) << "\",\"discoveryFirmware\":\"" << json_escape(e.acFWString) << "\"}";
  }
  oss << "]";
  return dup_str(oss.str());
}

char* jlink_bridge_probe_firmware(int index) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

  int n = 0;
  auto list = fetch_probe_list(*a, &n);
  if (n < 0) {
    set_err("JLINKARM_EMU_GetList failed");
    return nullptr;
  }
  if (index < 0 || index >= n) {
    set_err("probe index out of range");
    return nullptr;
  }
  const auto& sel = list[static_cast<size_t>(index)];
  if (!select_probe_usb_serial_first(*a, index, list)) {
    set_err("select probe failed");
    return nullptr;
  }

#ifdef _WIN32
  ScopedCwd _cwd_probe(g_dll_dir);
#endif

  std::string open_cap;
  const char* open_err = open_ex_capture(*a, open_cap);
  if (open_err != nullptr) {
    set_err(std::string("OpenEx: ") + open_err);
    return nullptr;
  }

  int sn_after = a->JLINKARM_GetSN();
  if (sel.Connection == JLINKARM_HOSTIF_USB && sn_after >= 0 &&
      static_cast<U32>(sn_after) != sel.SerialNumber) {
    a->JLINKARM_Close();
    if (a->JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) >= 0) {
      open_cap.clear();
      open_err = open_ex_capture(*a, open_cap);
      if (open_err != nullptr) {
        set_err(std::string("OpenEx: ") + open_err);
        return nullptr;
      }
    }
  }

  char fw[512] = {};
  a->JLINKARM_GetFirmwareString(fw, static_cast<int>(sizeof(fw) - 1));
  a->JLINKARM_Close();

  std::string out = firmware_compiled_date(fw);
  if (out.empty() && fw[0]) out = fw;
  return dup_str(out);
}

char* jlink_bridge_update_firmware(int index) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

#ifdef _WIN32
  // CWD + SetDllDirectory are set persistently in jlink_bridge_load (WinUSBSwitcher demo parity).
  ScopedCwd _cwd(g_dll_dir);
#endif

  int n = 0;
  auto list = fetch_probe_list(*a, &n);
  if (n < 0 || index < 0 || index >= n) {
    set_err("invalid probe list or index");
    return nullptr;
  }
  if (!select_probe(*a, index, list)) {
    set_err("select probe failed");
    return nullptr;
  }
  const auto& sel = list[static_cast<size_t>(index)];

  std::ostringstream env_diag;
#ifdef _WIN32
  {
    const std::string cwd_now = get_current_directory_a();
    const std::string bin_name = guess_firmware_bin_name(sel);
    std::string fw_path = g_dll_dir + "\\Firmwares\\" + bin_name;
    env_diag << "dll_dir=" << g_dll_dir << "\n"
             << "cwd_during_update=" << cwd_now << "\n"
             << "expected_firmware_file=" << fw_path << "\n"
             << "firmware_file_exists=" << (file_exists_a(fw_path) ? "yes" : "no") << "\n";
  }
#endif

  std::string open_cap;
  const char* open_err = open_ex_capture(*a, open_cap);
  if (open_err != nullptr) {
    set_err(std::string("OpenEx: ") + open_err);
    return nullptr;
  }

  char fw_before[512] = {};
  a->JLINKARM_GetFirmwareString(fw_before, static_cast<int>(sizeof(fw_before) - 1));
  std::string fw_before_s = firmware_compiled_date(fw_before);
  if (fw_before_s.empty() && fw_before[0]) fw_before_s = fw_before;

  int sn_after_open = a->JLINKARM_GetSN();
  env_diag << "GetSN_after_OpenEx=" << sn_after_open << " expected_serial=" << sel.SerialNumber;
  if (sn_after_open >= 0 && static_cast<U32>(sn_after_open) != sel.SerialNumber) {
    env_diag << " MISMATCH";
  }
  env_diag << "\n";

  if (sel.Connection == JLINKARM_HOSTIF_USB && sn_after_open >= 0 &&
      static_cast<U32>(sn_after_open) != sel.SerialNumber) {
    a->JLINKARM_Close();
    if (a->JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) >= 0) {
      open_cap.clear();
      open_err = open_ex_capture(*a, open_cap);
      if (open_err == nullptr) {
        env_diag << "reopen_after_SN_mismatch: OK\n";
        std::memset(fw_before, 0, sizeof(fw_before));
        a->JLINKARM_GetFirmwareString(fw_before, static_cast<int>(sizeof(fw_before) - 1));
        fw_before_s = firmware_compiled_date(fw_before);
        if (fw_before_s.empty() && fw_before[0]) fw_before_s = fw_before;
        sn_after_open = a->JLINKARM_GetSN();
        env_diag << "GetSN_after_reopen=" << sn_after_open << " expected_serial=" << sel.SerialNumber << "\n";
      } else {
        env_diag << "reopen_after_SN_mismatch: OpenEx failed\n";
      }
    } else {
      env_diag << "reopen_after_SN_mismatch: SelectByUSBSN failed\n";
    }
  }

  bool updated = callback_log_suggests_firmware_activity(open_cap);
  std::string out_all = open_cap;

  std::string cap_update;
  g_capture = &cap_update;
  const U32 rc = a->JLINKARM_UpdateFirmwareIfNewer();
  g_capture = nullptr;
  env_diag << "JLINKARM_UpdateFirmwareIfNewer rc=" << static_cast<unsigned long>(rc)
           << " (WinUSBSwitcher demo: single UpdateFirmwareIfNewer after OpenEx)\n";
  if (!cap_update.empty()) {
    if (!out_all.empty()) out_all += "\n";
    out_all += cap_update;
  }
  updated = updated || callback_log_suggests_firmware_activity(cap_update);
  updated = updated || (rc != 0);

  // Re-read firmware; if the probe rebooted mid-update, do a short reopen+re-read loop.
  char fw[512] = {};
  a->JLINKARM_GetFirmwareString(fw, static_cast<int>(sizeof(fw) - 1));
  a->JLINKARM_Close();

  std::string fw_after_s = firmware_compiled_date(fw);
  if (fw_after_s.empty() && fw[0]) fw_after_s = fw;

  if (fw_after_s == fw_before_s) {
    for (int i = 0; i < 6; ++i) {
      sleep_ms(250);
      if (!select_probe(*a, index, list)) continue;
      std::string retry_cap;
      const char* oe2 = open_ex_capture(*a, retry_cap);
      if (oe2 != nullptr) continue;
      char tmp[512] = {};
      a->JLINKARM_GetFirmwareString(tmp, static_cast<int>(sizeof(tmp) - 1));
      a->JLINKARM_Close();
      std::string tmp_s = firmware_compiled_date(tmp);
      if (tmp_s.empty() && tmp[0]) tmp_s = tmp;
      if (!tmp_s.empty()) fw_after_s = tmp_s;
      if (fw_after_s != fw_before_s) break;
    }
  }

  // "updated" should reflect reality, not just API return codes.
  updated = updated || (!fw_after_s.empty() && fw_after_s != fw_before_s);

  std::ostringstream oss;
  const std::string detail =
      env_diag.str() +
      std::string("Index=") + std::to_string(index) +
      "\nSerial=" + std::to_string(sel.SerialNumber) +
      "\nProduct=" + std::string(sel.acProduct) +
      "\nNickname=" + std::string(sel.acNickName) +
      "\n\n[note] WinUSBSwitcher demo flow: load → set CWD/DLL dir from loadedPath → OpenEx → "
      "GetFirmwareString → JLINKARM_UpdateFirmwareIfNewer only (no JLINKARM_Connect). "
      "If GetSN mismatches expected_serial, wrong probe. If firmware_file_exists=no, fix bundle layout."
      "\nBefore=" + fw_before_s +
      "\nAfter=" + fw_after_s +
      (out_all.empty() ? std::string()
                       : (std::string("\n\n[DLL callback log — OpenEx + UpdateFirmwareIfNewer]\n") +
                          tail_text(out_all, 2000)));

  oss << "{\"status\":\"" << (updated ? "updated" : "current") << "\",\"firmware\":\""
      << json_escape(fw_after_s.c_str()) << "\",\"beforeFirmware\":\""
      << json_escape(fw_before_s.c_str()) << "\",\"serialNumber\":\""
      << sel.SerialNumber << "\",\"detail\":\"" << json_escape_detail_for_json(detail) << "\",\"error\":\"\"}";
  return dup_str(oss.str());
}

char* jlink_bridge_switch_usb_driver(int index, int winusb) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;

#ifdef _WIN32
  ScopedCwd _cwd(g_dll_dir);
#endif

  int n = 0;
  auto list = fetch_probe_list(*a, &n);
  if (n < 0 || index < 0 || index >= n) {
    set_err("invalid probe list or index");
    return dup_str("{\"success\":false,\"error\":\"invalid index\",\"detail\":\"\",\"rebootNotSupported\":false}");
  }
  if (!select_probe(*a, index, list)) {
    set_err("select probe failed");
    return dup_str("{\"success\":false,\"error\":\"select probe failed\",\"detail\":\"\",\"rebootNotSupported\":false}");
  }

  const char* open_err = a->JLINKARM_OpenEx(&log_cb, &err_cb);
  if (open_err != nullptr) {
    set_err(std::string("OpenEx: ") + open_err);
    return dup_str("{\"success\":false,\"error\":\"OpenEx failed\",\"detail\":\"\",\"rebootNotSupported\":false}");
  }

  // ---- Preferred path for DLL mode: config-memory WinUSB toggle ----
  // Your probes (J-Link-OB-S124) support WinUSB switching in Commander; in DLL mode the
  // Commander-like commands (selectprobe/WebUSBEnable) are not always available via
  // JLINKARM_ExecCommand(). The config-mem method is the most reliable.
  //
  // This matches the WinUSBSwitcher demo logic:
  //   read config at offset 0x8E, bit3 (0 = enabled, 1 = disabled)
  constexpr U32 CONFIG_OFF_HW_FEATURES = 0x8E;
  constexpr U8  HW_FEATURE_WINUSB_DISABLE_BIT = 3;

  const int cap = a->JLINKARM_EMU_HasCapEx(JLINKARM_EMU_CAP_EX_WINUSB);
  if (cap == 0) {
    std::string dbg = "Probe does not report WINUSB capability (JLINKARM_EMU_CAP_EX_WINUSB).";
    set_err(dbg);
    a->JLINKARM_Close();
    std::ostringstream oss;
    oss << "{\"success\":false,\"error\":\"USB driver commands not supported by this DLL/probe.\",\"detail\":\""
        << json_escape_str(dbg) << "\",\"rebootNotSupported\":false}";
    return dup_str(oss.str());
  }

  U8 cfg = 0;
  if (a->JLINKARM_ReadEmuConfigMem(&cfg, CONFIG_OFF_HW_FEATURES, 1) != 0) {
    std::string dbg = "ReadEmuConfigMem failed.";
    set_err(dbg);
    a->JLINKARM_Close();
    std::ostringstream oss;
    oss << "{\"success\":false,\"error\":\"Failed to read probe config.\",\"detail\":\""
        << json_escape_str(dbg) << "\",\"rebootNotSupported\":false}";
    return dup_str(oss.str());
  }

  const U8 oldCfg = cfg;
  if (winusb) {
    cfg &= static_cast<U8>(~(1u << HW_FEATURE_WINUSB_DISABLE_BIT)); // 0 => enabled
  } else {
    cfg |= static_cast<U8>(1u << HW_FEATURE_WINUSB_DISABLE_BIT);    // 1 => disabled
  }

  if (cfg != oldCfg) {
    if (a->JLINKARM_WriteEmuConfigMem(&cfg, CONFIG_OFF_HW_FEATURES, 1) != 0) {
      std::string dbg = "WriteEmuConfigMem failed.";
      set_err(dbg);
      a->JLINKARM_Close();
      std::ostringstream oss;
      oss << "{\"success\":false,\"error\":\"Failed to write probe config.\",\"detail\":\""
          << json_escape_str(dbg) << "\",\"rebootNotSupported\":false}";
      return dup_str(oss.str());
    }
  }

  // Close the JLINKARM session before reboot scheduling.
  a->JLINKARM_Close();

  // Reboot session
  if (!select_probe(*a, index, list)) {
    return dup_str("{\"success\":true,\"error\":\"\",\"detail\":\"\",\"rebootNotSupported\":true}");
  }

  open_err = a->JLINKARM_OpenEx(&log_cb, &err_cb);
  if (open_err != nullptr) {
    return dup_str("{\"success\":true,\"error\":\"\",\"detail\":\"\",\"rebootNotSupported\":true}");
  }

  // Prefer ScheduleReboot (works across more probe families), fall back to reboot.
  std::string reboot_out = exec_out(*a, "ScheduleReboot");
  if (contains_unknown_command(reboot_out)) {
    reboot_out = exec_out(*a, "reboot");
  }
  a->JLINKARM_Close();

  bool reboot_unsupported = reboot_out.find("Command not supported by connected probe.") != std::string::npos;
  std::ostringstream oss;
  oss << "{\"success\":true,\"error\":\"\",\"detail\":\"\",\"rebootNotSupported\":" << (reboot_unsupported ? "true" : "false") << "}";
  return dup_str(oss.str());
}

char* jlink_bridge_dll_version_string(void) {
  std::lock_guard<std::mutex> lock(g_mu);
  g_err.clear();
  JLinkARMDLL* a = api_or_set_err();
  if (!a) return nullptr;
  U32 v = a->JLINKARM_GetDLLVersion();
  if (v == 0) return nullptr;
  std::ostringstream oss;
  oss << "SEGGER J-Link DLL (build " << v << ")";
  return dup_str(oss.str());
}
