#include "commander_exec.h"

#include <chrono>
#include <cstdio>
#include <thread>

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <Windows.h>
#endif

namespace commander_exec {

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

int ExecShowEmuList(JLinkARMDLL& a, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list) {
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

int ExecSelectEmuFromList(JLinkARMDLL& a, int index, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list) {
  const int r = ExecShowEmuList(a, out_list);
  if (r < 0) return -1;
  if (index < 0 || index >= static_cast<int>(out_list.size())) return -1;

  const auto& sel = out_list[static_cast<size_t>(index)];
  if (sel.Connection == JLINKARM_HOSTIF_USB) {
    if (a.JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) >= 0) return 0;
  }
  if (a.JLINKARM_EMU_SelectByIndex(static_cast<U32>(index)) != 0) return 0;
  return -1;
}

bool ConnectToJLink(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_err) {
  out_err.clear();
  if (index < 0 || index >= static_cast<int>(list.size())) {
    out_err = "invalid index";
    return false;
  }
  std::vector<JLINKARM_EMU_CONNECT_INFO> ignored;
  if (ExecSelectEmuFromList(a, index, ignored) < 0) {
    out_err = "select probe failed";
    return false;
  }
  const char* open_err = a.JLINKARM_OpenEx(&log_cb, &err_cb);
  if (open_err != nullptr) {
    out_err = std::string("OpenEx: ") + open_err;
    return false;
  }
  return true;
}

void DisconnectFromJLink(JLinkARMDLL& a) { a.JLINKARM_Close(); }

const char* OpenExCapture(JLinkARMDLL& a, std::string& cap) {
  cap.clear();
  g_capture = &cap;
  const char* err = a.JLINKARM_OpenEx(&log_cb, &err_cb);
  g_capture = nullptr;
  return err;
}

bool ConnectToJLinkCapture(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_capture, std::string& out_err) {
  out_err.clear();
  out_capture.clear();
  if (index < 0 || index >= static_cast<int>(list.size())) {
    out_err = "invalid index";
    return false;
  }
  std::vector<JLINKARM_EMU_CONNECT_INFO> ignored;
  if (ExecSelectEmuFromList(a, index, ignored) < 0) {
    out_err = "select probe failed";
    return false;
  }
  const char* open_err = OpenExCapture(a, out_capture);
  if (open_err != nullptr) {
    out_err = std::string("OpenEx: ") + open_err;
    return false;
  }
  return true;
}

bool EnsureSelectedUsbSn(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& io_capture, std::string& out_err, std::ostringstream* diag) {
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
  DisconnectFromJLink(a);
  if (a.JLINKARM_EMU_SelectByUSBSN(static_cast<U32>(sel.SerialNumber)) < 0) {
    out_err = "SelectByUSBSN failed";
    return false;
  }
  io_capture.clear();
  const char* open_err = OpenExCapture(a, io_capture);
  if (open_err != nullptr) {
    out_err = std::string("OpenEx: ") + open_err;
    return false;
  }
  if (diag) *diag << "reopen_after_SN_mismatch: OK\n";
  return true;
}

std::string ExecOut(JLinkARMDLL& a, const char* cmd) {
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

bool ContainsUnknownCommand(const std::string& s) {
  return s.find("Unknown command") != std::string::npos || s.find("ERROR: Unknown command") != std::string::npos;
}

bool CallbackLogSuggestsFirmwareActivity(const std::string& s) {
  return s.find("Updating firmware") != std::string::npos ||
         s.find("Replacing firmware") != std::string::npos ||
         s.find("New firmware") != std::string::npos ||
         s.find("Waiting for new firmware") != std::string::npos;
}

std::string GuessFirmwareBinName(const JLINKARM_EMU_CONNECT_INFO& e) {
  const char* p = e.acProduct;
  if (!p || !p[0]) return "JLink_OB_S124.bin";
  std::string s(p);
  if (s.find("OB-S124") != std::string::npos) return "JLink_OB_S124.bin";
  if (s.find("OB-S") != std::string::npos) return "JLink_OB_S124.bin";
  return "JLink_OB_S124.bin";
}

std::string CaptureUpdateFirmwareIfNewer(JLinkARMDLL& a, U32* out_rc) {
  std::string cap;
  g_capture = &cap;
  const U32 rc = a.JLINKARM_UpdateFirmwareIfNewer();
  g_capture = nullptr;
  if (out_rc) *out_rc = rc;
  return cap;
}

void ExecSleep(unsigned ms) {
  std::this_thread::sleep_for(std::chrono::milliseconds(ms));
}

RebootResult ExecReboot(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list) {
  RebootResult r;
  r.attempted = true;

  std::string conn_err;
  if (!ConnectToJLink(a, index, list, conn_err)) {
    r.command.clear();
    r.not_supported = true;
    return r;
  }

  r.command = "ScheduleReboot";
  std::string reboot_out = ExecOut(a, "ScheduleReboot");
  if (ContainsUnknownCommand(reboot_out)) {
    r.command = "reboot";
    reboot_out = ExecOut(a, "reboot");
  }
  DisconnectFromJLink(a);

  r.not_supported = reboot_out.find("Command not supported by connected probe.") != std::string::npos;

  if (!r.command.empty() && !r.not_supported) {
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(5000);
    ExecSleep(500);
    while (std::chrono::steady_clock::now() < deadline) {
      std::string tmp_err;
      if (ConnectToJLink(a, index, list, tmp_err)) {
        DisconnectFromJLink(a);
        break;
      }
      ExecSleep(50);
    }
  }

  return r;
}

static bool ExecWinUSBConfig(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, bool enable_winusb, std::string& out_detail_for_error) {
  out_detail_for_error.clear();
  if (!ConnectToJLink(a, index, list, out_detail_for_error)) return false;

  constexpr U32 CONFIG_OFF_HW_FEATURES = 0x8E;
  constexpr U8  HW_FEATURE_WINUSB_DISABLE_BIT = 3;

  const int cap = a.JLINKARM_EMU_HasCapEx(JLINKARM_EMU_CAP_EX_WINUSB);
  if (cap == 0) {
    out_detail_for_error = "Probe does not report WINUSB capability (JLINKARM_EMU_CAP_EX_WINUSB).";
    DisconnectFromJLink(a);
    return false;
  }

  U8 cfg = 0;
  if (a.JLINKARM_ReadEmuConfigMem(&cfg, CONFIG_OFF_HW_FEATURES, 1) != 0) {
    out_detail_for_error = "ReadEmuConfigMem failed.";
    DisconnectFromJLink(a);
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
      DisconnectFromJLink(a);
      return false;
    }
  }
  DisconnectFromJLink(a);
  return true;
}

bool ExecWebUSBEnable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error) {
  return ExecWinUSBConfig(a, index, list, true, out_detail_for_error);
}

bool ExecWebUSBDisable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error) {
  return ExecWinUSBConfig(a, index, list, false, out_detail_for_error);
}

} // namespace commander_exec

