#include "jlink/JLinkARMDLL_Wrapper.h"

#include <cstdio>

// Internal trace — writes a single line to stderr.
// Replace body with a stored callback if the embedding app needs to redirect.
static void jlTrace(const char* prefix, const char* msg) {
  std::fprintf(stderr, "[JLinkARMDLL] %s%s\n", prefix, msg);
}

// ---------------------------------------------------------------------------

JLinkARMDLL::JLinkARMDLL()  = default;
JLinkARMDLL::~JLinkARMDLL() { Unload(); }

// ---------------------------------------------------------------------------

bool JLinkARMDLL::Load(const std::string& DLLpath) {
  Unload();
  m_lastError.clear();

  m_hDll = m_pal.loadLib(DLLpath.c_str());
  if (!m_hDll) {
    m_lastError  = "LoadLib failed: ";
    m_lastError += m_pal.lastErrorString();
    return false;
  }

  m_loadedPath = m_pal.loadedModulePath(m_hDll);
  if (m_loadedPath.empty()) m_loadedPath = DLLpath;

  bool ok = true;

  // Setup / connection
  mapRequired(m_fn.EMU_SelectByUSBSN,      "JLINKARM_EMU_SelectByUSBSN",     ok);
  mapOptional(m_fn.EMU_SelectIPBySN,       "JLINKARM_EMU_SelectIPBySN");
  mapOptional(m_fn.EMU_SelectIP,           "JLINKARM_EMU_SelectIP");
  mapOptional(m_fn.SelectIP,               "JLINKARM_SelectIP");
  mapRequired(m_fn.Open,                   "JLINKARM_Open",                   ok);
  mapRequired(m_fn.OpenEx,                 "JLINKARM_OpenEx",                 ok);
  mapRequired(m_fn.GetFirmwareString,      "JLINKARM_GetFirmwareString",      ok);
  mapOptional(m_fn.UpdateFirmware,         "JLINKARM_UpdateFirmware");
  // Intentionally not loading JLINKARM_UpdateFirmwareIfNewer.
  mapOptional(m_fn.UpdateReplaceFirmware,  "JLINKARM_UpdateReplaceFirmware");
  mapOptional(m_fn.Connect,               "JLINKARM_Connect");
  mapRequired(m_fn.Close,                  "JLINKARM_Close",                  ok);

  // Config
  mapRequired(m_fn.ExecCommand,            "JLINKARM_ExecCommand",            ok);
  mapOptional(m_fn.TIF_GetAvailable,       "JLINKARM_TIF_GetAvailable");
  mapOptional(m_fn.TIF_Select,             "JLINKARM_TIF_Select");
  mapOptional(m_fn.SetSpeed,               "JLINKARM_SetSpeed");
  mapOptional(m_fn.ConfigJTAG,             "JLINKARM_ConfigJTAG");

  // Debug control
  mapOptional(m_fn.IsHalted,               "JLINKARM_IsHalted");
  mapOptional(m_fn.Halt,                   "JLINKARM_Halt");
  mapOptional(m_fn.GoEx,                   "JLINKARM_GoEx");
  mapOptional(m_fn.Reset,                  "JLINKARM_Reset");
  mapOptional(m_fn.Step,                   "JLINKARM_Step");
  mapOptional(m_fn.StepComposite,          "JLINKARM_StepComposite");
  mapOptional(m_fn.GetMOEs,               "JLINKARM_GetMOEs");

  // Registers
  mapOptional(m_fn.GetRegisterList,        "JLINKARM_GetRegisterList");
  mapOptional(m_fn.GetRegisterName,        "JLINKARM_GetRegisterName");
  mapOptional(m_fn.ReadReg,                "JLINKARM_ReadReg");
  mapOptional(m_fn.ReadRegs,               "JLINKARM_ReadRegs");
  mapOptional(m_fn.WriteReg,               "JLINKARM_WriteReg");
  mapOptional(m_fn.ReadRegs_64,            "JLINK_ReadRegs_64");
  mapOptional(m_fn.WriteRegs_64,           "JLINK_WriteRegs_64");

  // Memory
  mapOptional(m_fn.ReadMemEx,              "JLINKARM_ReadMemEx");
  mapOptional(m_fn.ReadMem,                "JLINKARM_ReadMem");
  mapOptional(m_fn.WriteMemEx,             "JLINKARM_WriteMemEx");
  mapOptional(m_fn.WriteMem,               "JLINKARM_WriteMem");
  mapOptional(m_fn.ReadMemZonedEx,         "JLINK_ReadMemZonedEx");
  mapOptional(m_fn.WriteMemZonedEx,        "JLINK_WriteMemZonedEx");

  // Breakpoints / watchpoints
  mapOptional(m_fn.GetNumBPUnits,          "JLINKARM_GetNumBPUnits");
  mapOptional(m_fn.GetNumBPs,              "JLINKARM_GetNumBPs");
  mapOptional(m_fn.EnableSoftBPs,          "JLINKARM_EnableSoftBPs");
  mapOptional(m_fn.SetBPEx,                "JLINKARM_SetBPEx");
  mapOptional(m_fn.ClrBPEx,               "JLINKARM_ClrBPEx");
  mapOptional(m_fn.GetNumWPUnits,          "JLINKARM_GetNumWPUnits");
  mapOptional(m_fn.GetNumWPs,              "JLINKARM_GetNumWPs");
  mapOptional(m_fn.SetWP,                  "JLINKARM_SetWP");
  mapOptional(m_fn.ClrWP,                  "JLINKARM_ClrWP");
  mapOptional(m_fn.FindBP,                 "JLINKARM_FindBP");

  // Probe management
  mapRequired(m_fn.EMU_GetList,            "JLINKARM_EMU_GetList",            ok);
  mapOptional(m_fn.EMU_SelectByIndex,      "JLINKARM_EMU_SelectByIndex");
  mapRequired(m_fn.EMU_HasCapEx,           "JLINKARM_EMU_HasCapEx",           ok);
  mapRequired(m_fn.GetEmuCaps,             "JLINKARM_GetEmuCaps",             ok);
  mapRequired(m_fn.GetSN,                  "JLINKARM_GetSN",                  ok);
  mapRequired(m_fn.GetHardwareVersion,     "JLINKARM_GetHardwareVersion",     ok);
  mapOptional(m_fn.GetDLLVersion,          "JLINKARM_GetDLLVersion");
  mapRequired(m_fn.ReadEmuConfigMem,       "JLINKARM_ReadEmuConfigMem",       ok);
  mapRequired(m_fn.WriteEmuConfigMem,      "JLINKARM_WriteEmuConfigMem",      ok);

  // stdcall JLINK_* (optional)
  mapOptional(m_fn.JLINK_ExecCommand,      "JLINK_ExecCommand");
  mapOptional(m_fn.JLINK_Open,             "JLINK_Open");
  mapOptional(m_fn.JLINK_UpdateFirmware,   "JLINK_UpdateFirmware");
  mapOptional(m_fn.JLINK_UpdateFirmwareIfNewer, "JLINK_UpdateFirmwareIfNewer");
  mapOptional(m_fn.JLINK_UpdateReplaceFirmware, "JLINK_UpdateReplaceFirmware");
  mapOptional(m_fn.JLINK_Close,            "JLINK_Close");

  jlTrace("Loaded: ", m_loadedPath.c_str());
  return ok;
}

void JLinkARMDLL::Unload() {
  if (m_hDll) {
    m_pal.freeLib(m_hDll);
    m_hDll = nullptr;
  }
  m_loadedPath.clear();
  m_fn = Funcs{};   // zero-initialises all pointers in one assignment
}

// ---------------------------------------------------------------------------
// Call-through methods — thin wrappers; required functions call directly,
// optional ones guard with an early nullptr check.
// ---------------------------------------------------------------------------

int  JLinkARMDLL::JLINKARM_EMU_SelectByUSBSN(U32 SerialNo)
     { return m_fn.EMU_SelectByUSBSN(SerialNo); }

void JLinkARMDLL::JLINKARM_EMU_SelectIPBySN(U32 SerialNo)
     { if (m_fn.EMU_SelectIPBySN) m_fn.EMU_SelectIPBySN(SerialNo); }

int  JLinkARMDLL::JLINKARM_EMU_SelectIP(char* pIPAddr, int BufferSize, U16* pPort)
     { return m_fn.EMU_SelectIP ? m_fn.EMU_SelectIP(pIPAddr, BufferSize, pPort) : -1; }

char JLinkARMDLL::JLINKARM_SelectIP(const char* pHost, int port)
     { return m_fn.SelectIP ? m_fn.SelectIP(pHost, port) : (char)-1; }

const char* JLinkARMDLL::JLINKARM_Open(void)
     { return m_fn.Open(); }

const char* JLinkARMDLL::JLINKARM_OpenEx(JLINKARM_LOG* pfLog, JLINKARM_LOG* pfErrorOut)
     { return m_fn.OpenEx(pfLog, pfErrorOut); }

void JLinkARMDLL::JLINKARM_GetFirmwareString(char* s, int BufferSize)
     { m_fn.GetFirmwareString(s, BufferSize); }

U16  JLinkARMDLL::JLINKARM_UpdateFirmware(void)
     { return m_fn.UpdateFirmware ? m_fn.UpdateFirmware() : 0; }

int  JLinkARMDLL::JLINKARM_UpdateReplaceFirmware(int Replace, const char* sInfo)
     { return m_fn.UpdateReplaceFirmware ? m_fn.UpdateReplaceFirmware(Replace, sInfo) : -1; }

int  JLinkARMDLL::JLINKARM_Connect(void)
     { return m_fn.Connect ? m_fn.Connect() : -1; }

void JLinkARMDLL::JLINKARM_Close(void)
     { m_fn.Close(); }

int  JLinkARMDLL::JLINKARM_ExecCommand(const char* pIn, char* pOut, int BufferSize)
     { return m_fn.ExecCommand(pIn, pOut, BufferSize); }

void JLinkARMDLL::JLINKARM_TIF_GetAvailable(U32* pMask)
     { if (m_fn.TIF_GetAvailable) m_fn.TIF_GetAvailable(pMask); }

int  JLinkARMDLL::JLINKARM_TIF_Select(int Interface)
     { return m_fn.TIF_Select ? m_fn.TIF_Select(Interface) : -1; }

void JLinkARMDLL::JLINKARM_SetSpeed(U32 Speed)
     { if (m_fn.SetSpeed) m_fn.SetSpeed(Speed); }

void JLinkARMDLL::JLINKARM_ConfigJTAG(int IRPre, int DRPre)
     { if (m_fn.ConfigJTAG) m_fn.ConfigJTAG(IRPre, DRPre); }

char JLinkARMDLL::JLINKARM_IsHalted(void)
     { return m_fn.IsHalted      ? m_fn.IsHalted()                      : 0;   }
char JLinkARMDLL::JLINKARM_Halt(void)
     { return m_fn.Halt          ? m_fn.Halt()                          : 0;   }

void JLinkARMDLL::JLINKARM_GoEx(U32 MaxEmulInsts, U32 Flags)
     { if (m_fn.GoEx) m_fn.GoEx(MaxEmulInsts, Flags); }

int  JLinkARMDLL::JLINKARM_Reset(void)
     { return m_fn.Reset         ? m_fn.Reset()                         : -1;  }
char JLinkARMDLL::JLINKARM_Step(void)
     { return m_fn.Step          ? m_fn.Step()                          : 0;   }
char JLinkARMDLL::JLINKARM_StepComposite(void)
     { return m_fn.StepComposite ? m_fn.StepComposite()                 : 0;   }

int  JLinkARMDLL::JLINKARM_GetMOEs(JLINKARM_MOE_INFO* pInfo, int MaxNumMOEs)
     { return m_fn.GetMOEs ? m_fn.GetMOEs(pInfo, MaxNumMOEs) : -1; }

int  JLinkARMDLL::JLINKARM_GetRegisterList(U32* paList, int MaxNumItems)
     { return m_fn.GetRegisterList ? m_fn.GetRegisterList(paList, MaxNumItems) : -1; }

const char* JLinkARMDLL::JLINKARM_GetRegisterName(U32 RegIndex)
     { return m_fn.GetRegisterName ? m_fn.GetRegisterName(RegIndex) : ""; }

U32  JLinkARMDLL::JLINKARM_ReadReg(ARM_REG RegIndex)
     { return m_fn.ReadReg   ? m_fn.ReadReg(RegIndex) : 0; }

int  JLinkARMDLL::JLINKARM_ReadRegs(const U32* paRegIndex, U32* paData, U8* paStatus, U32 NumRegs)
     { return m_fn.ReadRegs  ? m_fn.ReadRegs(paRegIndex, paData, paStatus, NumRegs) : -1; }

char JLinkARMDLL::JLINKARM_WriteReg(ARM_REG RegIndex, U32 Data)
     { return m_fn.WriteReg  ? m_fn.WriteReg(RegIndex, Data) : 0; }

int  JLinkARMDLL::JLINK_ReadRegs_64(const U32* paRegIndex, U64* paData, U8* paStatus, U32 NumRegs)
     { return m_fn.ReadRegs_64  ? m_fn.ReadRegs_64(paRegIndex, paData, paStatus, NumRegs) : -1; }

int  JLinkARMDLL::JLINK_WriteRegs_64(const U32* paRegIndex, const U64* paData, U8* paStatus, U32 NumRegs)
     { return m_fn.WriteRegs_64 ? m_fn.WriteRegs_64(paRegIndex, paData, paStatus, NumRegs) : -1; }

int  JLinkARMDLL::JLINKARM_ReadMemEx(U32 Addr, U32 NumBytes, void* pData, U32 AccessWidth)
     { return m_fn.ReadMemEx  ? m_fn.ReadMemEx(Addr, NumBytes, pData, AccessWidth) : -1; }

int  JLinkARMDLL::JLINKARM_ReadMem(U32 Addr, U32 NumBytes, void* pData)
     { return m_fn.ReadMem    ? m_fn.ReadMem(Addr, NumBytes, pData) : -1; }

int  JLinkARMDLL::JLINKARM_WriteMemEx(U32 Addr, U32 NumBytes, const void* pData, U32 AccessWidth)
     { return m_fn.WriteMemEx ? m_fn.WriteMemEx(Addr, NumBytes, pData, AccessWidth) : -1; }

int  JLinkARMDLL::JLINKARM_WriteMem(U32 Addr, U32 Count, const void* pData)
     { return m_fn.WriteMem   ? m_fn.WriteMem(Addr, Count, pData) : -1; }

int  JLinkARMDLL::JLINK_ReadMemZonedEx(U32 Addr, U32 NumBytes, void* pData, U32 Flags, const char* sZone)
     { return m_fn.ReadMemZonedEx  ? m_fn.ReadMemZonedEx(Addr, NumBytes, pData, Flags, sZone) : -1; }

int  JLinkARMDLL::JLINK_WriteMemZonedEx(U32 Addr, U32 NumBytes, const void* pData, U32 Flags, const char* sZone)
     { return m_fn.WriteMemZonedEx ? m_fn.WriteMemZonedEx(Addr, NumBytes, pData, Flags, sZone) : -1; }

int  JLinkARMDLL::JLINKARM_GetNumBPUnits(U32 Type)
     { return m_fn.GetNumBPUnits ? m_fn.GetNumBPUnits(Type) : -1; }
int  JLinkARMDLL::JLINKARM_GetNumBPs(void)
     { return m_fn.GetNumBPs     ? m_fn.GetNumBPs()          : -1; }

void JLinkARMDLL::JLINKARM_EnableSoftBPs(char Enable)
     { if (m_fn.EnableSoftBPs) m_fn.EnableSoftBPs(Enable); }

int  JLinkARMDLL::JLINKARM_SetBPEx(U32 Addr, U32 TypeFlags)
     { return m_fn.SetBPEx ? m_fn.SetBPEx(Addr, TypeFlags) : -1; }
int  JLinkARMDLL::JLINKARM_ClrBPEx(int BPHandle)
     { return m_fn.ClrBPEx ? m_fn.ClrBPEx(BPHandle)        : -1; }

int  JLinkARMDLL::JLINKARM_GetNumWPUnits(void)
     { return m_fn.GetNumWPUnits ? m_fn.GetNumWPUnits() : -1; }
int  JLinkARMDLL::JLINKARM_GetNumWPs(void)
     { return m_fn.GetNumWPs     ? m_fn.GetNumWPs()      : -1; }

int  JLinkARMDLL::JLINKARM_SetWP(U32 AccessAddr, U32 AddrMask, U32 AccessData, U32 DataMask,
                                   U8 AccessType, U8 TypeMask)
     { return m_fn.SetWP ? m_fn.SetWP(AccessAddr, AddrMask, AccessData, DataMask, AccessType, TypeMask) : -1; }

int  JLinkARMDLL::JLINKARM_ClrWP(int WPHandle)
     { return m_fn.ClrWP  ? m_fn.ClrWP(WPHandle)  : -1; }
int  JLinkARMDLL::JLINKARM_FindBP(U32 Addr)
     { return m_fn.FindBP ? m_fn.FindBP(Addr)      : -1; }

int  JLinkARMDLL::JLINKARM_EMU_GetList(int HostIFs, JLINKARM_EMU_CONNECT_INFO* paConnectInfo, int MaxInfos)
     { return m_fn.EMU_GetList(HostIFs, paConnectInfo, MaxInfos); }

U32  JLinkARMDLL::JLINKARM_EMU_SelectByIndex(U32 iEmu)
     { return m_fn.EMU_SelectByIndex ? m_fn.EMU_SelectByIndex(iEmu) : 0; }

int  JLinkARMDLL::JLINKARM_EMU_HasCapEx(int CapEx)
     { return m_fn.EMU_HasCapEx(CapEx); }
U32  JLinkARMDLL::JLINKARM_GetEmuCaps(void)
     { return m_fn.GetEmuCaps(); }
int  JLinkARMDLL::JLINKARM_GetSN(void)
     { return m_fn.GetSN(); }
int  JLinkARMDLL::JLINKARM_GetHardwareVersion(void)
     { return m_fn.GetHardwareVersion(); }

U32  JLinkARMDLL::JLINKARM_GetDLLVersion(void)
     { return m_fn.GetDLLVersion ? m_fn.GetDLLVersion() : 0; }

int  JLinkARMDLL::JLINKARM_ReadEmuConfigMem(U8* p, U32 Off, U32 NumBytes)
     { return m_fn.ReadEmuConfigMem(p, Off, NumBytes); }
int  JLinkARMDLL::JLINKARM_WriteEmuConfigMem(const U8* p, U32 Off, U32 NumBytes)
     { return m_fn.WriteEmuConfigMem(p, Off, NumBytes); }

int  JLinkARMDLL::JLINK_ExecCommand(const char* pIn, char* pOut, int BufferSize)
     { return m_fn.JLINK_ExecCommand ? m_fn.JLINK_ExecCommand(pIn, pOut, BufferSize) : -1; }

const char* JLinkARMDLL::JLINK_Open(void)
     { return m_fn.JLINK_Open ? m_fn.JLINK_Open() : "JLINK_Open not available"; }

U16  JLinkARMDLL::JLINK_UpdateFirmware(void)
     { return m_fn.JLINK_UpdateFirmware ? m_fn.JLINK_UpdateFirmware() : 0; }

U32  JLinkARMDLL::JLINK_UpdateFirmwareIfNewer(void)
     { return m_fn.JLINK_UpdateFirmwareIfNewer ? m_fn.JLINK_UpdateFirmwareIfNewer() : 0; }

int  JLinkARMDLL::JLINK_UpdateReplaceFirmware(int Replace, const char* sInfo)
     { return m_fn.JLINK_UpdateReplaceFirmware ? m_fn.JLINK_UpdateReplaceFirmware(Replace, sInfo) : -1; }

void JLinkARMDLL::JLINK_Close(void)
     { if (m_fn.JLINK_Close) m_fn.JLINK_Close(); }
