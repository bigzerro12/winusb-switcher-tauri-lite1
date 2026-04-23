#include "common/runtime_dirs.h"

#ifdef _WIN32
#  define WIN32_LEAN_AND_MEAN
#  include <Windows.h>
#else
#  include <limits.h>
#  include <unistd.h>
#endif

namespace runtime_dirs {

#ifdef _WIN32
static bool set_current_directory_best_effort(const std::string& new_dir) {
  if (new_dir.empty()) return false;
  if (SetCurrentDirectoryA(new_dir.c_str())) return true;
  if (new_dir.size() >= 4 && new_dir[0] == '\\' && new_dir[1] == '\\' && new_dir[2] == '?' && new_dir[3] == '\\') {
    return SetCurrentDirectoryA(new_dir.substr(4).c_str()) != 0;
  }
  return false;
}
#endif

std::string dirname_utf8(const char* path) {
  if (!path) return {};
  std::string p(path);
#ifdef _WIN32
  for (char& c : p) {
    if (c == '/') c = '\\';
  }
  const size_t pos = p.find_last_of("\\");
#else
  const size_t pos = p.find_last_of('/');
#endif
  if (pos == std::string::npos) return {};
  return p.substr(0, pos);
}

void apply_jlink_runtime_dirs(const std::string& dirRaw) {
  if (dirRaw.empty()) return;
#ifdef _WIN32
  std::string dir = dirRaw;
  for (char& c : dir) {
    if (c == '/') c = '\\';
  }
  set_current_directory_best_effort(dir);
  if (dir.size() >= 4 && dir[0] == '\\' && dir[1] == '\\' && dir[2] == '?' && dir[3] == '\\') {
    dir = dir.substr(4);
  }
  SetDllDirectoryA(dir.c_str());
#else
  (void)chdir(dirRaw.c_str());
#endif
}

ScopedCwd::ScopedCwd(const std::string& new_dir) {
  if (new_dir.empty()) return;
#ifdef _WIN32
  DWORD n = GetCurrentDirectoryA(static_cast<DWORD>(sizeof(old_)), old_);
  (void)n;
  if (set_current_directory_best_effort(new_dir)) {
    changed_ = true;
  }
#else
  char buf[PATH_MAX] = {};
  if (getcwd(buf, sizeof(buf))) old_ = buf;
  if (chdir(new_dir.c_str()) == 0) {
    changed_ = true;
  }
#endif
}

ScopedCwd::~ScopedCwd() {
#ifdef _WIN32
  if (changed_) {
    SetCurrentDirectoryA(old_);
  }
#else
  if (changed_ && !old_.empty()) {
    (void)chdir(old_.c_str());
  }
#endif
}

#ifdef _WIN32
std::string get_current_directory_a() {
  char buf[MAX_PATH * 4] = {};
  DWORD n = GetCurrentDirectoryA(static_cast<DWORD>(sizeof(buf)), buf);
  if (!n || n >= sizeof(buf)) return {};
  return std::string(buf);
}

bool file_exists_a(const std::string& path) {
  if (path.empty()) return false;
  const DWORD a = GetFileAttributesA(path.c_str());
  return a != INVALID_FILE_ATTRIBUTES && (a & FILE_ATTRIBUTE_DIRECTORY) == 0;
}
#endif

} // namespace runtime_dirs
