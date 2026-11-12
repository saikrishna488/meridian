#pragma once
#include "core/error.hpp"
#include <gio/gio.h>
#include <memory>

namespace meridian {
struct ObjectUnref { void operator()(void *value) const { if (value) g_object_unref(value); } };
template<class T> using GObjectPtr = std::unique_ptr<T, ObjectUnref>;
struct GFree { void operator()(void *value) const { g_free(value); } };
template<class T> using GMemory = std::unique_ptr<T, GFree>;
struct StrvFree { void operator()(char **value) const { g_strfreev(value); } };
using GStrv = std::unique_ptr<char *, StrvFree>;
class GioError {
    GError *value_ = nullptr;
public:
    ~GioError() { g_clear_error(&value_); }
    GError **out() { return &value_; }
    bool failed() const { return value_ != nullptr; }
    [[noreturn]] void raise(const char *code = "failed") const {
        if (value_ && g_error_matches(value_, G_IO_ERROR, G_IO_ERROR_CANCELLED))
            throw Error("unavailable", "Request cancelled.");
        throw Error(code, value_ ? value_->message : "System operation failed.");
    }
};
inline void check_cancelled(GCancellable *cancel) {
    if (cancel && g_cancellable_is_cancelled(cancel)) throw Error("unavailable", "Request cancelled.");
}
}
