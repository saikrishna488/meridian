#pragma once
#include "core/gio.hpp"
#include <chrono>
#include <string>
#include <vector>

namespace meridian {
struct ProcessResult { std::string output, diagnostics; int status; };
// Fixed argument vectors only. No shell, bounded stdout/stderr and input,
// monotonic timeout, cancellation and owned child reaping. Diagnostics are
// available to classification code; never log them or put secrets in argv.
ProcessResult run_process(const std::vector<std::string> &arguments, GCancellable *cancel = nullptr,
    std::chrono::milliseconds timeout = std::chrono::seconds(12), std::string_view input = {});
std::string run_checked(const std::vector<std::string> &arguments, GCancellable *cancel = nullptr,
    std::chrono::milliseconds timeout = std::chrono::seconds(12), std::string_view input = {});
}
