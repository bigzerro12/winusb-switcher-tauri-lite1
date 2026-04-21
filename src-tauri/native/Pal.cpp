#include "Pal.h"

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <Windows.h>
#else
#  include <dlfcn.h>
#endif

namespace {
#ifdef _WIN32
static std::string win32ErrorString(DWORD err) {
  if (err == 0) return {};
  LPSTR buf = nullptr;
  DWORD n = FormatMessageA(
      FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM |
          FORMAT_MESSAGE_IGNORE_INSERTS,
      nullptr, err, MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
      reinterpret_cast<LPSTR>(&buf), 0, nullptr);
  std::string msg;
  if (n && buf) { msg.assign(buf, n); LocalFree(buf); }
  return msg;
}
#endif
} // namespace

void* Pal::loadLib(const char* path) {
#ifdef _WIN32
  // LOAD_WITH_ALTERED_SEARCH_PATH restricts the secondary DLL search to the
  // folder of the named DLL (not the process working directory), which prevents
  // DLL search-path pollution and reduces DLL-planting risk.
  void* h = reinterpret_cast<void*>(
      LoadLibraryExA(path, nullptr, LOAD_WITH_ALTERED_SEARCH_PATH));
  // Capture GetLastError() immediately — any subsequent Win32 call would clear it.
  m_lastErrStr = h ? std::string{} : win32ErrorString(GetLastError());
  return h;
#else
  dlerror(); // clear any stale error
  void* h = dlopen(path, RTLD_NOW);
  if (!h) {
    const char* e = dlerror();
    m_lastErrStr = e ? e : "dlopen failed";
  } else {
    m_lastErrStr.clear();
  }
  return h;
#endif
}

void Pal::freeLib(void* h) {
  if (!h) return;
#ifdef _WIN32
  FreeLibrary(static_cast<HMODULE>(h));
#else
  dlclose(h);
#endif
}

void* Pal::getProc(void* h, const char* name) const {
  if (!h) return nullptr;
#ifdef _WIN32
  return reinterpret_cast<void*>(GetProcAddress(static_cast<HMODULE>(h), name));
#else
  return dlsym(h, name);
#endif
}

std::string Pal::loadedModulePath(void* h) const {
#ifdef _WIN32
  char buf[MAX_PATH] = {};
  if (GetModuleFileNameA(static_cast<HMODULE>(h), buf, static_cast<DWORD>(sizeof(buf))))
    return std::string(buf);
  return {};
#else
  (void)h;
  return {};
#endif
}
