#pragma once

#include "jlink/JLinkARMDLL_Wrapper.h"

#include <sstream>
#include <string>
#include <vector>

namespace commander_exec {

struct RebootResult {
  bool attempted = false;
  bool not_supported = false;
  std::string command;
};

int _ExecShowEmuList(JLinkARMDLL& a, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list);
int _ExecSelectEmuFromList(JLinkARMDLL& a, int index, std::vector<JLINKARM_EMU_CONNECT_INFO>& out_list);

bool _ConnectToJLink(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_err);
bool _ConnectToJLinkCapture(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_capture, std::string& out_err);
void _DisconnectFromJLink(JLinkARMDLL& a);

bool _EnsureSelectedUsbSn(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& io_capture, std::string& out_err, std::ostringstream* diag);

void _ExecSleep(unsigned ms);
RebootResult _ExecReboot(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list);

bool _ExecWebUSBEnable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error);
bool _ExecWebUSBDisable(JLinkARMDLL& a, int index, const std::vector<JLINKARM_EMU_CONNECT_INFO>& list, std::string& out_detail_for_error);

// Low-level helpers used by the bridge.
const char* _OpenExCapture(JLinkARMDLL& a, std::string& cap);
std::string _ExecOut(JLinkARMDLL& a, const char* cmd);
bool _CallbackLogSuggestsFirmwareActivity(const std::string& s);
bool _ContainsUnknownCommand(const std::string& s);
std::string _GuessFirmwareBinName(const JLINKARM_EMU_CONNECT_INFO& e);

// Commander-style `exec <Command>` helper:
// - Extracts the command token (letters only) for routing decisions.
// - Ensures a J-Link connection when needed (OpenEx capture path).
// - Runs JLINKARM_ExecCommand and returns stdout + captured callback text.
std::string _ExecExecCommand(
    JLinkARMDLL& a,
    int index,
    const std::vector<JLINKARM_EMU_CONNECT_INFO>& list,
    const char* exec_cmd,
    std::string& out_err
);

} // namespace commander_exec

