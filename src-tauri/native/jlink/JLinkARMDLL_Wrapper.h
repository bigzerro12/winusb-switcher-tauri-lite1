#pragma once

#include <string>

#include "common/Pal.h"
// Official SEGGER SDK headers (vendored under native/jlink/)
#include "JLINKARM_Const.h"
#include "JLinkARMDLL.h"

// GDB-server-style dynamic wrapper (Load -> map exports -> call-through).
// Naming is intentionally aligned with Src/Targets/ARM/JLinkARMDLL.{h,cpp}.
//
// Design notes:
//  - All resolved function pointers live in the nested Funcs struct.
//    Adding a new API requires only: one field in Funcs, one mapXxx() call
//    in Load(), and one call-through method. Unload() needs no change.
//  - lpfnXXX type aliases use decltype(&::JLINKARM_...) so calling
//    conventions are inherited from the official SEGGER header — critical
//    for cdecl vs stdcall correctness on Win32.
//  - mapRequired / mapOptional are private inline templates; no separate
//    GetProcAddress wrapper method is needed (that name also shadows the
//    Win32 global, which is confusing).
class JLinkARMDLL {
public:
  JLinkARMDLL();
  virtual ~JLinkARMDLL();

  // Pass by const ref — no copy, Load() only reads the path.
  virtual bool Load(const std::string& DLLpath);
  virtual void Unload();

  bool               isLoaded()    const { return m_hDll != nullptr; }
  const std::string& loadedPath()  const { return m_loadedPath; }
  const std::string& lastError()   const { return m_lastError; }

public:
  // ---- Setup / connection ----
  int         JLINKARM_EMU_SelectByUSBSN(U32 SerialNo);
  void        JLINKARM_EMU_SelectIPBySN(U32 SerialNo);
  int         JLINKARM_EMU_SelectIP(char* pIPAddr, int BufferSize, U16* pPort);
  char        JLINKARM_SelectIP(const char* pHost, int port);

  const char* JLINKARM_Open(void);
  const char* JLINKARM_OpenEx(JLINKARM_LOG* pfLog, JLINKARM_LOG* pfErrorOut);
  void        JLINKARM_GetFirmwareString(char* s, int BufferSize);
  U16         JLINKARM_UpdateFirmware(void);
  int         JLINKARM_UpdateReplaceFirmware(int Replace, const char* sInfo);
  int         JLINKARM_Connect(void);
  void        JLINKARM_Close(void);

  // ---- Config ----
  int         JLINKARM_ExecCommand(const char* pIn, char* pOut, int BufferSize);
  void        JLINKARM_TIF_GetAvailable(U32* pMask);
  int         JLINKARM_TIF_Select(int Interface);
  void        JLINKARM_SetSpeed(U32 Speed);
  void        JLINKARM_ConfigJTAG(int IRPre, int DRPre);

  // ---- Debug control ----
  char        JLINKARM_IsHalted(void);
  char        JLINKARM_Halt(void);
  void        JLINKARM_GoEx(U32 MaxEmulInsts, U32 Flags);
  int         JLINKARM_Reset(void);
  char        JLINKARM_Step(void);
  char        JLINKARM_StepComposite(void);
  int         JLINKARM_GetMOEs(JLINKARM_MOE_INFO* pInfo, int MaxNumMOEs);

  // ---- Registers ----
  int         JLINKARM_GetRegisterList(U32* paList, int MaxNumItems);
  const char* JLINKARM_GetRegisterName(U32 RegIndex);
  U32         JLINKARM_ReadReg(ARM_REG RegIndex);
  int         JLINKARM_ReadRegs(const U32* paRegIndex, U32* paData, U8* paStatus, U32 NumRegs);
  char        JLINKARM_WriteReg(ARM_REG RegIndex, U32 Data);
  int         JLINK_ReadRegs_64(const U32* paRegIndex, U64* paData, U8* paStatus, U32 NumRegs);
  int         JLINK_WriteRegs_64(const U32* paRegIndex, const U64* paData, U8* paStatus, U32 NumRegs);

  // ---- Memory ----
  int         JLINKARM_ReadMemEx(U32 Addr, U32 NumBytes, void* pData, U32 AccessWidth);
  int         JLINKARM_ReadMem(U32 Addr, U32 NumBytes, void* pData);
  int         JLINKARM_WriteMemEx(U32 Addr, U32 NumBytes, const void* pData, U32 AccessWidth);
  int         JLINKARM_WriteMem(U32 Addr, U32 Count, const void* pData);
  int         JLINK_ReadMemZonedEx(U32 Addr, U32 NumBytes, void* pData, U32 Flags, const char* sZone);
  int         JLINK_WriteMemZonedEx(U32 Addr, U32 NumBytes, const void* pData, U32 Flags, const char* sZone);

  // ---- Breakpoints / watchpoints ----
  int         JLINKARM_GetNumBPUnits(U32 Type);
  int         JLINKARM_GetNumBPs(void);
  void        JLINKARM_EnableSoftBPs(char Enable);
  int         JLINKARM_SetBPEx(U32 Addr, U32 TypeFlags);
  int         JLINKARM_ClrBPEx(int BPHandle);
  int         JLINKARM_GetNumWPUnits(void);
  int         JLINKARM_GetNumWPs(void);
  int         JLINKARM_SetWP(U32 AccessAddr, U32 AddrMask, U32 AccessData, U32 DataMask, U8 AccessType, U8 TypeMask);
  int         JLINKARM_ClrWP(int WPHandle);
  int         JLINKARM_FindBP(U32 Addr);

  // ---- Probe management ----
  int         JLINKARM_EMU_GetList(int HostIFs, JLINKARM_EMU_CONNECT_INFO* paConnectInfo, int MaxInfos);
  U32         JLINKARM_EMU_SelectByIndex(U32 iEmu);
  int         JLINKARM_EMU_HasCapEx(int CapEx);
  U32         JLINKARM_GetEmuCaps(void);
  int         JLINKARM_GetSN(void);
  int         JLINKARM_GetHardwareVersion(void);
  U32         JLINKARM_GetDLLVersion(void);
  int         JLINKARM_ReadEmuConfigMem(U8* p, U32 Off, U32 NumBytes);
  int         JLINKARM_WriteEmuConfigMem(const U8* p, U32 Off, U32 NumBytes);

  // ---- Reboot helpers (stdcall JLINK_* entry points) ----
  int         JLINK_ExecCommand(const char* pIn, char* pOut, int BufferSize);
  const char* JLINK_Open(void);
  U16         JLINK_UpdateFirmware(void);
  U32         JLINK_UpdateFirmwareIfNewer(void);
  int         JLINK_UpdateReplaceFirmware(int Replace, const char* sInfo);
  void        JLINK_Close(void);

private:
  // -------------------------------------------------------------------------
  // Function pointer type aliases.
  // decltype(&::JLINKARM_...) inherits the exact signature and calling
  // convention from the official SEGGER header — no manual __cdecl/__stdcall.
  // -------------------------------------------------------------------------
  using lpfnJLINKARM_EMU_SelectByUSBSN    = decltype(&::JLINKARM_EMU_SelectByUSBSN);
  using lpfnJLINKARM_EMU_SelectIPBySN     = decltype(&::JLINKARM_EMU_SelectIPBySN);
  using lpfnJLINKARM_EMU_SelectIP         = decltype(&::JLINKARM_EMU_SelectIP);
  using lpfnJLINKARM_SelectIP             = decltype(&::JLINKARM_SelectIP);
  using lpfnJLINKARM_Open                 = decltype(&::JLINKARM_Open);
  using lpfnJLINKARM_OpenEx               = decltype(&::JLINKARM_OpenEx);
  using lpfnJLINKARM_GetFirmwareString    = decltype(&::JLINKARM_GetFirmwareString);
  using lpfnJLINKARM_UpdateFirmware       = decltype(&::JLINKARM_UpdateFirmware);
  using lpfnJLINKARM_UpdateReplaceFirmware= decltype(&::JLINKARM_UpdateReplaceFirmware);
  using lpfnJLINKARM_Connect              = decltype(&::JLINKARM_Connect);
  using lpfnJLINKARM_Close                = decltype(&::JLINKARM_Close);
  using lpfnJLINKARM_ExecCommand          = decltype(&::JLINKARM_ExecCommand);
  using lpfnJLINKARM_TIF_GetAvailable     = decltype(&::JLINKARM_TIF_GetAvailable);
  using lpfnJLINKARM_TIF_Select           = decltype(&::JLINKARM_TIF_Select);
  using lpfnJLINKARM_SetSpeed             = decltype(&::JLINKARM_SetSpeed);
  using lpfnJLINKARM_ConfigJTAG           = decltype(&::JLINKARM_ConfigJTAG);
  using lpfnJLINKARM_IsHalted             = decltype(&::JLINKARM_IsHalted);
  using lpfnJLINKARM_Halt                 = decltype(&::JLINKARM_Halt);
  using lpfnJLINKARM_GoEx                 = decltype(&::JLINKARM_GoEx);
  using lpfnJLINKARM_Reset                = decltype(&::JLINKARM_Reset);
  using lpfnJLINKARM_Step                 = decltype(&::JLINKARM_Step);
  using lpfnJLINKARM_StepComposite        = decltype(&::JLINKARM_StepComposite);
  using lpfnJLINKARM_GetMOEs              = decltype(&::JLINKARM_GetMOEs);
  using lpfnJLINKARM_GetRegisterList      = decltype(&::JLINKARM_GetRegisterList);
  using lpfnJLINKARM_GetRegisterName      = decltype(&::JLINKARM_GetRegisterName);
  using lpfnJLINKARM_ReadReg              = decltype(&::JLINKARM_ReadReg);
  using lpfnJLINKARM_ReadRegs             = decltype(&::JLINKARM_ReadRegs);
  using lpfnJLINKARM_WriteReg             = decltype(&::JLINKARM_WriteReg);
  using lpfnJLINK_ReadRegs_64             = decltype(&::JLINK_ReadRegs_64);
  using lpfnJLINK_WriteRegs_64            = decltype(&::JLINK_WriteRegs_64);
  using lpfnJLINKARM_ReadMemEx            = decltype(&::JLINKARM_ReadMemEx);
  using lpfnJLINKARM_ReadMem              = decltype(&::JLINKARM_ReadMem);
  using lpfnJLINKARM_WriteMemEx           = decltype(&::JLINKARM_WriteMemEx);
  using lpfnJLINKARM_WriteMem             = decltype(&::JLINKARM_WriteMem);
  using lpfnJLINK_ReadMemZonedEx          = decltype(&::JLINK_ReadMemZonedEx);
  using lpfnJLINK_WriteMemZonedEx         = decltype(&::JLINK_WriteMemZonedEx);
  using lpfnJLINKARM_GetNumBPUnits        = decltype(&::JLINKARM_GetNumBPUnits);
  using lpfnJLINKARM_GetNumBPs            = decltype(&::JLINKARM_GetNumBPs);
  using lpfnJLINKARM_EnableSoftBPs        = decltype(&::JLINKARM_EnableSoftBPs);
  using lpfnJLINKARM_SetBPEx              = decltype(&::JLINKARM_SetBPEx);
  using lpfnJLINKARM_ClrBPEx              = decltype(&::JLINKARM_ClrBPEx);
  using lpfnJLINKARM_GetNumWPUnits        = decltype(&::JLINKARM_GetNumWPUnits);
  using lpfnJLINKARM_GetNumWPs            = decltype(&::JLINKARM_GetNumWPs);
  using lpfnJLINKARM_SetWP                = decltype(&::JLINKARM_SetWP);
  using lpfnJLINKARM_ClrWP                = decltype(&::JLINKARM_ClrWP);
  using lpfnJLINKARM_FindBP               = decltype(&::JLINKARM_FindBP);
  using lpfnJLINKARM_EMU_GetList          = decltype(&::JLINKARM_EMU_GetList);
  using lpfnJLINKARM_EMU_SelectByIndex    = decltype(&::JLINKARM_EMU_SelectByIndex);
  using lpfnJLINKARM_EMU_HasCapEx         = decltype(&::JLINKARM_EMU_HasCapEx);
  using lpfnJLINKARM_GetEmuCaps           = decltype(&::JLINKARM_GetEmuCaps);
  using lpfnJLINKARM_GetSN                = decltype(&::JLINKARM_GetSN);
  using lpfnJLINKARM_GetHardwareVersion   = decltype(&::JLINKARM_GetHardwareVersion);
  using lpfnJLINKARM_GetDLLVersion        = decltype(&::JLINKARM_GetDLLVersion);
  using lpfnJLINKARM_ReadEmuConfigMem     = decltype(&::JLINKARM_ReadEmuConfigMem);
  using lpfnJLINKARM_WriteEmuConfigMem    = decltype(&::JLINKARM_WriteEmuConfigMem);
  using lpfnJLINK_ExecCommand             = decltype(&::JLINK_ExecCommand);
  using lpfnJLINK_Open                    = decltype(&::JLINK_Open);
  using lpfnJLINK_UpdateFirmware          = decltype(&::JLINK_UpdateFirmware);
  using lpfnJLINK_UpdateFirmwareIfNewer   = decltype(&::JLINK_UpdateFirmwareIfNewer);
  using lpfnJLINK_UpdateReplaceFirmware   = decltype(&::JLINK_UpdateReplaceFirmware);
  using lpfnJLINK_Close                   = decltype(&::JLINK_Close);

  // -------------------------------------------------------------------------
  // All resolved function pointers in one aggregate.
  // Unload() resets the whole table with a single "m_fn = Funcs{}".
  // To add a new API: add one field here + one mapXxx() call in Load() +
  // one call-through method above. Unload() requires no change.
  // Short field names (strip JLINKARM_ prefix) keep call-through bodies
  // concise; JLINK_* names keep their prefix to distinguish stdcall variants.
  // -------------------------------------------------------------------------
  struct Funcs {
    lpfnJLINKARM_EMU_SelectByUSBSN    EMU_SelectByUSBSN    = nullptr;
    lpfnJLINKARM_EMU_SelectIPBySN     EMU_SelectIPBySN     = nullptr;
    lpfnJLINKARM_EMU_SelectIP         EMU_SelectIP         = nullptr;
    lpfnJLINKARM_SelectIP             SelectIP             = nullptr;

    lpfnJLINKARM_Open                 Open                 = nullptr;
    lpfnJLINKARM_OpenEx               OpenEx               = nullptr;
    lpfnJLINKARM_GetFirmwareString    GetFirmwareString    = nullptr;
    lpfnJLINKARM_UpdateFirmware       UpdateFirmware       = nullptr;
    lpfnJLINKARM_UpdateReplaceFirmware UpdateReplaceFirmware = nullptr;
    lpfnJLINKARM_Connect              Connect              = nullptr;
    lpfnJLINKARM_Close                Close                = nullptr;

    lpfnJLINKARM_ExecCommand          ExecCommand          = nullptr;
    lpfnJLINKARM_TIF_GetAvailable     TIF_GetAvailable     = nullptr;
    lpfnJLINKARM_TIF_Select           TIF_Select           = nullptr;
    lpfnJLINKARM_SetSpeed             SetSpeed             = nullptr;
    lpfnJLINKARM_ConfigJTAG           ConfigJTAG           = nullptr;

    lpfnJLINKARM_IsHalted             IsHalted             = nullptr;
    lpfnJLINKARM_Halt                 Halt                 = nullptr;
    lpfnJLINKARM_GoEx                 GoEx                 = nullptr;
    lpfnJLINKARM_Reset                Reset                = nullptr;
    lpfnJLINKARM_Step                 Step                 = nullptr;
    lpfnJLINKARM_StepComposite        StepComposite        = nullptr;
    lpfnJLINKARM_GetMOEs              GetMOEs              = nullptr;

    lpfnJLINKARM_GetRegisterList      GetRegisterList      = nullptr;
    lpfnJLINKARM_GetRegisterName      GetRegisterName      = nullptr;
    lpfnJLINKARM_ReadReg              ReadReg              = nullptr;
    lpfnJLINKARM_ReadRegs             ReadRegs             = nullptr;
    lpfnJLINKARM_WriteReg             WriteReg             = nullptr;
    lpfnJLINK_ReadRegs_64             ReadRegs_64          = nullptr;
    lpfnJLINK_WriteRegs_64            WriteRegs_64         = nullptr;

    lpfnJLINKARM_ReadMemEx            ReadMemEx            = nullptr;
    lpfnJLINKARM_ReadMem              ReadMem              = nullptr;
    lpfnJLINKARM_WriteMemEx           WriteMemEx           = nullptr;
    lpfnJLINKARM_WriteMem             WriteMem             = nullptr;
    lpfnJLINK_ReadMemZonedEx          ReadMemZonedEx       = nullptr;
    lpfnJLINK_WriteMemZonedEx         WriteMemZonedEx      = nullptr;

    lpfnJLINKARM_GetNumBPUnits        GetNumBPUnits        = nullptr;
    lpfnJLINKARM_GetNumBPs            GetNumBPs            = nullptr;
    lpfnJLINKARM_EnableSoftBPs        EnableSoftBPs        = nullptr;
    lpfnJLINKARM_SetBPEx              SetBPEx              = nullptr;
    lpfnJLINKARM_ClrBPEx              ClrBPEx              = nullptr;
    lpfnJLINKARM_GetNumWPUnits        GetNumWPUnits        = nullptr;
    lpfnJLINKARM_GetNumWPs            GetNumWPs            = nullptr;
    lpfnJLINKARM_SetWP                SetWP                = nullptr;
    lpfnJLINKARM_ClrWP                ClrWP                = nullptr;
    lpfnJLINKARM_FindBP               FindBP               = nullptr;

    lpfnJLINKARM_EMU_GetList          EMU_GetList          = nullptr;
    lpfnJLINKARM_EMU_SelectByIndex    EMU_SelectByIndex    = nullptr;
    lpfnJLINKARM_EMU_HasCapEx         EMU_HasCapEx         = nullptr;
    lpfnJLINKARM_GetEmuCaps           GetEmuCaps           = nullptr;
    lpfnJLINKARM_GetSN                GetSN                = nullptr;
    lpfnJLINKARM_GetHardwareVersion   GetHardwareVersion   = nullptr;
    lpfnJLINKARM_GetDLLVersion        GetDLLVersion        = nullptr;
    lpfnJLINKARM_ReadEmuConfigMem     ReadEmuConfigMem     = nullptr;
    lpfnJLINKARM_WriteEmuConfigMem    WriteEmuConfigMem    = nullptr;

    // stdcall JLINK_* variants — keep prefix to distinguish from JLINKARM_*
    lpfnJLINK_ExecCommand             JLINK_ExecCommand    = nullptr;
    lpfnJLINK_Open                    JLINK_Open           = nullptr;
    lpfnJLINK_UpdateFirmware          JLINK_UpdateFirmware  = nullptr;
    lpfnJLINK_UpdateFirmwareIfNewer   JLINK_UpdateFirmwareIfNewer = nullptr;
    lpfnJLINK_UpdateReplaceFirmware   JLINK_UpdateReplaceFirmware = nullptr;
    lpfnJLINK_Close                   JLINK_Close          = nullptr;
  };

  // -------------------------------------------------------------------------
  // mapRequired: export must be present; Load() returns false if missing.
  // mapOptional: missing export leaves fn = nullptr; callers check before use.
  // Both are inline templates — the compiler deduces T at each call site so
  // no explicit reinterpret_cast is needed at call sites in Load().
  // m_pal.getProc() is called directly (no shadowing of the Win32 global).
  // -------------------------------------------------------------------------
  template <typename T>
  void mapRequired(T& fn, const char* name, bool& ok) {
    fn = reinterpret_cast<T>(m_pal.getProc(m_hDll, name));
    if (!fn) {
      ok = false;
      m_lastError += "Missing required export: ";
      m_lastError += name;
      m_lastError += '\n';
    }
  }

  template <typename T>
  void mapOptional(T& fn, const char* name) {
    fn = reinterpret_cast<T>(m_pal.getProc(m_hDll, name));
  }

private:
  Pal         m_pal;
  void*       m_hDll     = nullptr;
  std::string m_loadedPath;
  std::string m_lastError;
  Funcs       m_fn;           // all resolved exports; reset as m_fn = Funcs{}
};
