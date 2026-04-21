#pragma once

#include <string>

// Platform abstraction layer for dynamic library loading.
// Mirrors the IPal concept in Src/Targets/ARM/ but kept lightweight
// with no external dependencies.
class Pal {
public:
  Pal()  = default;
  ~Pal() = default;

  // Loads a shared library by full path.
  // On Windows uses LOAD_WITH_ALTERED_SEARCH_PATH so the DLL's own folder
  // is used for secondary dependency resolution instead of polluting the
  // process-wide search path (reduces DLL-planting risk).
  // The error string is captured immediately on failure.
  void* loadLib(const char* path);

  void  freeLib(void* h);
  void* getProc(void* h, const char* name) const;

  // Error string set by the last loadLib() failure (empty on success).
  // Captured immediately after the OS call so GetLastError() is still valid.
  const std::string& lastErrorString() const { return m_lastErrStr; }

  std::string loadedModulePath(void* h) const;

private:
  std::string m_lastErrStr;
};
