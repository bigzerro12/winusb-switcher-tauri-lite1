#pragma once

#include <string>

namespace runtime_dirs {

std::string dirname_utf8(const char* path);

// Apply runtime directory settings so the SEGGER runtime can locate `Firmwares/`.
void apply_jlink_runtime_dirs(const std::string& dir);

// RAII process working directory switch (used around probe operations).
struct ScopedCwd {
  ScopedCwd(const std::string& new_dir);
  ~ScopedCwd();
  ScopedCwd(const ScopedCwd&) = delete;
  ScopedCwd& operator=(const ScopedCwd&) = delete;

private:
#ifdef _WIN32
  char old_[260] = {};
  bool changed_ = false;
#else
  std::string old_;
  bool changed_ = false;
#endif
};

#ifdef _WIN32
std::string get_current_directory_a();
bool file_exists_a(const std::string& path);
#endif

} // namespace runtime_dirs
