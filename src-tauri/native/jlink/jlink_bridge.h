#pragma once

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Heap-allocated UTF-8 string from bridge — always free with `jlink_bridge_free_string`.
void jlink_bridge_free_string(char* s);

/// Load a SEGGER J-Link shared library (DLL/.so) from a UTF-8 path.
/// Returns 0 on success, non-zero on error (`jlink_bridge_last_error`).
int jlink_bridge_load(const char* dll_path_utf8);

void jlink_bridge_unload(void);

/// Non-zero if `jlink_bridge_load` succeeded and the library is still mapped.
int jlink_bridge_is_loaded(void);

/// Internal buffer; valid until the next bridge call. Never free.
const char* jlink_bridge_last_error(void);

/// JSON array: `[{"index":0,"serialNumber":"...","connection":"USB|IP","nickName":"...","productName":"..."},...]`
/// Caller must `jlink_bridge_free_string` the result. Null on failure.
char* jlink_bridge_list_probes_json(void);

/// After open: JSON `{"firmware":"...","usbDriver":"WinUSB"|"SEGGER"|"Unknown"}` (HW features byte, bit 3).
/// OpenEx + ReadEmuConfigMem + GetFirmwareString for one list index. USB: selects by serial first. Caller frees.
char* jlink_bridge_probe_firmware(int index);

/// Execute a Commander-style `exec <Command>` string against the selected probe.
/// Returns captured stdout/callback text. Caller frees. Null on failure.
char* jlink_bridge_exec_command(int index, const char* exec_cmd_utf8);

/// `{"status":"updated"|"current"|"failed","firmware":"...","error":""}` — caller frees.
char* jlink_bridge_update_firmware(int index);

/// `{"success":true|false,"error":"","rebootNotSupported":false}` — caller frees.
char* jlink_bridge_switch_usb_driver(int index, int winusb /*1=WinUSB/WebUSB, 0=SEGGER*/);

/// Library product string e.g. "V9.36" — caller frees. Null if unavailable.
char* jlink_bridge_dll_version_string(void);

#ifdef __cplusplus
}
#endif
