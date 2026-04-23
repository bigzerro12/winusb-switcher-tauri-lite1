#include "jlink/commander_exec.h"

#include <chrono>
#include <cstdio>
#include <thread>

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <Windows.h>
#endif

namespace commander_exec {

// ---------------------------------------------------------------------------
//  Commander-style structure
//
//  This file is organized to mirror the high-level flow of SEGGER's J-Link
//  Commander Main.c, without copying its implementation:
//    - list probes (ShowEmuList)
//    - select a probe (SelectProbe)
//    - connect (OpenEx)
//    - execute an operation
//    - disconnect (Close)
// ---------------------------------------------------------------------------

// Many J-Link commands write output via the log/error callbacks rather than the
// `pOut` buffer of JLINKARM_ExecCommand(). Capture callback output so we can
// parse success reliably (mirrors what you see in J-Link Commander).
thread_local std::string* g_capture = nullptr;

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

// ─── "Commander-style" internal helpers ─────────────────────────────────────
//
// We mirror the structure of J-Link Commander's Main.c:
// - One-time list query (ShowEmuList / SelectProbe)
// - Select the desired emulator from the list
// - OpenEx (connect to J-Link, not target)
// - Execute the operation (ExecCommand, Read/WriteEmuConfigMem, etc.)
// - Disconnect (Close)

static bool _SelectProbeFromProvidedList(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_err) {
  out_err.clear();
  if (index < 0 || index >= static_cast<int>(list.size())) {
    out_err = "invalid index";
    return false;
  }
  const auto& sel = list[static_cast<size_t>(index)];
  if (sel.Connection == JLINKARM_HOSTIF_USB) {
    if (a.JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) >= 0) return true;
  }
  // For IP probes (or if SelectByUSBSN fails), fall back to index selection.
  if (a.JLINKARM_EMU_SelectByIndex(static_cast<U32>(index)) != 0) return true;
  out_err = "select probe failed";
  return false;
}

static bool _ConnectToJLinkInternal(JLinkARMDLL& a, std::string& out_err) {
  out_err.clear();

  // Disable the J-Link default "auto firmware update on connect" behavior.
  // Commander supports this via `exec DisableAutoUpdateFW` before SelectProbe/Open.
  // We apply it immediately before OpenEx for every connection attempt.
  {
    char out[512] = {};
    (void)a.JLINKARM_ExecCommand("DisableAutoUpdateFW", out, static_cast<int>(sizeof(out) - 1));
  }

  const char* open_err = a.JLINKARM_OpenEx(&log_cb, &err_cb);
  if (open_err != nullptr) {
    out_err = std::string("OpenEx: ") + open_err;
    return false;
  }
  return true;
}

void _DisconnectFromJLink(JLinkARMDLL& a) { a.JLINKARM_Close(); }

struct _SCOPED_DISCONNECT {
  JLinkARMDLL& a;
  bool close = false;
  explicit _SCOPED_DISCONNECT(JLinkARMDLL& api) : a(api) {}
  ~_SCOPED_DISCONNECT() {
    if (close) {
      a.JLINKARM_Close();
    }
  }
};

// ---------------------------------------------------------------------------
//  List / select
// ---------------------------------------------------------------------------

int _ExecShowEmuList(JLinkARMDLL& a, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list) {
  out_list.clear();
  const int interface_mask = static_cast<int>(JLINKARM_HOSTIF_USB | JLINKARM_HOSTIF_IP);
  std::vector<JLINKARM_EMU_CONNECT_INFO> buf(64);
  int r = a.JLINKARM_EMU_GetList(interface_mask, buf.data(), static_cast<int>(buf.size()));
  if (r < 0) return -1;
  if (r > static_cast<int>(buf.size())) {
    buf.resize(static_cast<size_t>(r));
    r = a.JLINKARM_EMU_GetList(interface_mask, buf.data(), static_cast<int>(buf.size()));
    if (r < 0) return -1;
  }
  if (r > static_cast<int>(buf.size())) r = static_cast<int>(buf.size());
  buf.resize(static_cast<size_t>(r));
  out_list = std::move(buf);
  return r;
}

int _ExecSelectEmuFromList(JLinkARMDLL& a, int index, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list) {
  const int r = _ExecShowEmuList(a, out_list);
  if (r < 0) return -1;
  if (index < 0 || index >= static_cast<int>(out_list.size())) return -1;
  std::string ignored;
  return _SelectProbeFromProvidedList(a, index, out_list, ignored) ? 0 : -1;
}

// ---------------------------------------------------------------------------
//  Connect / disconnect
// ---------------------------------------------------------------------------

bool _ConnectToJLink(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_err) {
  out_err.clear();
  if (!_SelectProbeFromProvidedList(a, index, list, out_err)) {
    return false;
  }

  return _ConnectToJLinkInternal(a, out_err);
}

const char* _OpenExCapture(JLinkARMDLL& a, std::string& cap) {
  cap.clear();
  g_capture = &cap;

  // Disable the default auto-update behavior before opening the J-Link connection.
  // Capture any callback output into `cap` for support logs.
  {
    char out[512] = {};
    (void)a.JLINKARM_ExecCommand("DisableAutoUpdateFW", out, static_cast<int>(sizeof(out) - 1));
    if (out[0]) cap.append(out);
  }

  const char* err = a.JLINKARM_OpenEx(&log_cb, &err_cb);
  g_capture = nullptr;
  return err;
}

bool _ConnectToJLinkCapture(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_capture, std::string& out_err) {
  out_err.clear();
  out_capture.clear();
  if (!_SelectProbeFromProvidedList(a, index, list, out_err)) {
    return false;
  }

  const char* open_err = _OpenExCapture(a, out_capture);
  if (open_err != nullptr) {
    out_err = std::string("OpenEx: ") + open_err;
    return false;
  }
  return true;
}

// ---------------------------------------------------------------------------
//  ExecCommand output capture helpers
// ---------------------------------------------------------------------------

bool _EnsureSelectedUsbSn(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& io_capture, std::string& out_err, std::ostringstream* diag) {
  out_err.clear();
  if (index < 0 || index >= static_cast<int>(list.size())) {
    out_err = "invalid index";
    return false;
  }
  const auto& sel = list[static_cast<size_t>(index)];
  if (sel.Connection != JLINKARM_HOSTIF_USB) return true;

  const int sn_after = a.JLINKARM_GetSN();
  if (sn_after < 0 || static_cast<U32>(sn_after) == sel.SerialNumber) return true;

  if (diag) *diag << "GetSN_after_OpenEx=" << sn_after << " expected_serial=" << sel.SerialNumber << " MISMATCH\n";
  _DisconnectFromJLink(a);
  if (a.JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) < 0) {
    out_err = "SelectByUSBSN failed";
    return false;
  }
  io_capture.clear();
  const char* open_err = _OpenExCapture(a, io_capture);
  if (open_err != nullptr) {
    out_err = std::string("OpenEx: ") + open_err;
    return false;
  }
  if (diag) *diag << "reopen_after_SN_mismatch: OK\n";
  return true;
}

std::string _ExecOut(JLinkARMDLL& a, const char* cmd) {
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
  if (rc != 0) {
    s = std::string("[ExecCommand rc=") + std::to_string(rc) + "] " + s;
  }
  return s;
}

bool _ContainsUnknownCommand(const std::string& s) {
  return s.find("Unknown command") != std::string::npos || s.find("ERROR: Unknown command") != std::string::npos;
}

bool _CallbackLogSuggestsFirmwareActivity(const std::string& s) {
  return s.find("Updating firmware") != std::string::npos ||
         s.find("Replacing firmware") != std::string::npos ||
         s.find("New firmware") != std::string::npos ||
         s.find("Waiting for new firmware") != std::string::npos;
}

static const char* _EatWhite(const char* s) {
  if (!s) return "";
  while (*s == ' ' || *s == '\t' || *s == '\r' || *s == '\n') ++s;
  return s;
}

static std::string _ExtractExecCommandToken(const char* s) {
  s = _EatWhite(s);
  std::string cmd;
  cmd.reserve(64);
  for (;;) {
    const char c = *s;
    if (!c) break;
    const bool is_letter = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
    if (!is_letter) break;
    cmd.push_back(c);
    ++s;
    if (cmd.size() >= 256) break;
  }
  return cmd;
}

static bool _ExecNeedsConnect(const std::string& cmd_token) {
  // Commander asks the DLL via JLINK_IFUNC_EXEC_GET_INFO to learn SetBeforeOpen/SetAfterOpen.
  // This project doesn't expose that interface, so we hardcode known "before open" toggles.
  if (cmd_token == "DisableAutoUpdateFW") return false;
  if (cmd_token == "EnableAutoUpdateFW") return false;
  return true;
}

std::string _GuessFirmwareBinName(const JLINKARM_EMU_CONNECT_INFO& e) {
  const char* p = e.acProduct;
  if (!p || !p[0]) return "JLink_OB_S124.bin";
  std::string s(p);
  if (s.find("OB-S124") != std::string::npos) return "JLink_OB_S124.bin";
  if (s.find("OB-S") != std::string::npos) return "JLink_OB_S124.bin";
  return "JLink_OB_S124.bin";
}

void _ExecSleep(unsigned ms) {
  std::this_thread::sleep_for(std::chrono::milliseconds(ms));
}

// ---------------------------------------------------------------------------
//  Commander-style "Exec" operations
// ---------------------------------------------------------------------------

std::string _ExecExecCommand(
    JLinkARMDLL& a,
    int index,
    const std::vector<JLINKARM_EMU_CONNECT_INFO>& list,
    const char* exec_cmd,
    std::string& out_err
) {
  out_err.clear();
  if (!exec_cmd) exec_cmd = "";

  const std::string token = _ExtractExecCommandToken(exec_cmd);
  const bool need_connect = _ExecNeedsConnect(token);

  // If the exec requires a J-Link connection, ensure we have one.
  // We don't track connection state in this module; GetSN() returns <0 when not connected.
  std::string open_cap;
  if (need_connect) {
    const int sn = a.JLINKARM_GetSN();
    if (sn < 0) {
      if (!_ConnectToJLinkCapture(a, index, list, open_cap, out_err)) {
        return open_cap;
      }
    }
  }

  char out[4000] = {};
  std::string exec_cap;
  g_capture = &exec_cap;
  const int rc = a.JLINKARM_ExecCommand(exec_cmd, out, static_cast<int>(sizeof(out) - 1));
  g_capture = nullptr;

  std::string s(out);
  if (!exec_cap.empty()) {
    if (!s.empty() && s.back() != '\n') s.push_back('\n');
    s += exec_cap;
  }
  if (!open_cap.empty()) {
    if (!s.empty() && s.back() != '\n') s.push_back('\n');
    s += open_cap;
  }
  if (rc != 0) {
    s = std::string("[ExecCommand rc=") + std::to_string(rc) + "] " + s;
  }
  return s;
}

RebootResult _ExecReboot(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list) {
  RebootResult r;
  r.attempted = true;

  std::string conn_err;
  if (!_ConnectToJLink(a, index, list, conn_err)) {
    r.command.clear();
    r.not_supported = true;
    return r;
  }

  r.command = "ScheduleReboot";
  std::string reboot_out = _ExecOut(a, "ScheduleReboot");
  if (_ContainsUnknownCommand(reboot_out)) {
    r.command = "reboot";
    reboot_out = _ExecOut(a, "reboot");
  }
  _DisconnectFromJLink(a);

  r.not_supported = reboot_out.find("Command not supported by connected probe.") != std::string::npos;

  if (!r.command.empty() && !r.not_supported) {
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(5000);
    _ExecSleep(500);
    while (std::chrono::steady_clock::now() < deadline) {
      std::string tmp_err;
      if (_ConnectToJLink(a, index, list, tmp_err)) {
        _DisconnectFromJLink(a);
        break;
      }
      _ExecSleep(50);
    }
  }

  return r;
}

static bool ExecWinUSBConfig(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, bool enable_winusb, std::string& out_detail_for_error) {
  out_detail_for_error.clear();
  if (!_ConnectToJLink(a, index, list, out_detail_for_error)) return false;
  _SCOPED_DISCONNECT _close(a);
  _close.close = true;

  constexpr U32 CONFIG_OFF_HW_FEATURES = 0x8E;
  constexpr U8  HW_FEATURE_WINUSB_DISABLE_BIT = 3;

  const int cap = a.JLINKARM_EMU_HasCapEx(JLINKARM_EMU_CAP_EX_WINUSB);
  if (cap == 0) {
    out_detail_for_error = "Probe does not report WINUSB capability (JLINKARM_EMU_CAP_EX_WINUSB).";
    return false;
  }

  U8 cfg = 0;
  if (a.JLINKARM_ReadEmuConfigMem(&cfg, CONFIG_OFF_HW_FEATURES, 1) != 0) {
    out_detail_for_error = "ReadEmuConfigMem failed.";
    return false;
  }

  const U8 oldCfg = cfg;
  if (enable_winusb) {
    cfg &= static_cast<U8>(~(1u << HW_FEATURE_WINUSB_DISABLE_BIT));
  } else {
    cfg |= static_cast<U8>(1u << HW_FEATURE_WINUSB_DISABLE_BIT);
  }

  if (cfg != oldCfg) {
    if (a.JLINKARM_WriteEmuConfigMem(&cfg, CONFIG_OFF_HW_FEATURES, 1) != 0) {
      out_detail_for_error = "WriteEmuConfigMem failed.";
      return false;
    }
  }
  return true;
}

bool _ExecWebUSBEnable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error) {
  return ExecWinUSBConfig(a, index, list, true, out_detail_for_error);
}

bool _ExecWebUSBDisable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error) {
  return ExecWinUSBConfig(a, index, list, false, out_detail_for_error);
}

} // namespace commander_exec

