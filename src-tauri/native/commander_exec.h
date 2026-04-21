#pragma once

#include "JLinkARMDLL_Wrapper.h"

#include <sstream>
#include <string>
#include <vector>

namespace commander_exec {

struct RebootResult {
  bool attempted = false;
  bool not_supported = false;
  std::string command;
};

int ExecShowEmuList(JLinkARMDLL& a, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list);
int ExecSelectEmuFromList(JLinkARMDLL& a, int index, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list);

bool ConnectToJLink(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_err);
bool ConnectToJLinkCapture(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_capture, std::string& out_err);
void DisconnectFromJLink(JLinkARMDLL& a);

bool EnsureSelectedUsbSn(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& io_capture, std::string& out_err, std::ostringstream* diag);

void ExecSleep(unsigned ms);
RebootResult ExecReboot(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list);

bool ExecWebUSBEnable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error);
bool ExecWebUSBDisable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error);

// Low-level helpers used by the bridge.
const char* OpenExCapture(JLinkARMDLL& a, std::string& cap);
std::string ExecOut(JLinkARMDLL& a, const char* cmd);
bool CallbackLogSuggestsFirmwareActivity(const std::string& s);
bool ContainsUnknownCommand(const std::string& s);
std::string GuessFirmwareBinName(const JLINKARM_EMU_CONNECT_INFO& e);

// Capture callback output during UpdateFirmwareIfNewer.
std::string CaptureUpdateFirmwareIfNewer(JLinkARMDLL& a, U32* out_rc);

} // namespace commander_exec

