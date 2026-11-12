#pragma once
#include <systemd/sd-json.h>
#include <cstdint>
#include <initializer_list>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace meridian {
// Immutable shared values. Child access acquires a reference, so it survives
// parent destruction. Values/references must stay on one thread (sd-json's
// reference counts are not atomic). Cross-thread boundaries carry owned bytes.
class Json {
    sd_json_variant *value_ = nullptr;
    explicit Json(sd_json_variant *value) : value_(value) {}
public:
    Json() = default;
    ~Json() { sd_json_variant_unref(value_); }
    Json(const Json &other) : value_(sd_json_variant_ref(other.value_)) {}
    Json(Json &&other) noexcept : value_(std::exchange(other.value_, nullptr)) {}
    Json &operator=(Json other) noexcept { std::swap(value_, other.value_); return *this; }
    static Json parse(std::string_view input, size_t limit = 2 * 1024 * 1024);
    static Json string(std::string_view value);
    static Json boolean(bool value);
    static Json number(double value);
    static Json integer(int64_t value);
    static Json unsigned_number(uint64_t value);
    static Json array(const std::vector<Json> &values = {});
    static Json object(std::initializer_list<std::pair<std::string_view, Json>> fields = {});
    bool exists() const { return value_ != nullptr; }
    bool is_null() const { return !value_ || sd_json_variant_is_null(value_); }
    bool is_string() const { return sd_json_variant_is_string(value_); }
    bool is_bool() const { return sd_json_variant_is_boolean(value_); }
    bool is_number() const { return sd_json_variant_is_number(value_); }
    bool is_array() const { return sd_json_variant_is_array(value_); }
    bool is_object() const { return sd_json_variant_is_object(value_); }
    bool is_unsigned() const { return sd_json_variant_type(value_) == SD_JSON_VARIANT_UNSIGNED; }
    bool is_integer() const {
        return sd_json_variant_type(value_) != SD_JSON_VARIANT_REAL && sd_json_variant_is_integer(value_);
    }
    std::string_view text() const;
    bool boolean() const;
    double number() const;
    uint64_t unsigned_number() const;
    int64_t integer() const;
    size_t size() const { return sd_json_variant_elements(value_); }
    Json at(size_t index) const { return Json(sd_json_variant_ref(sd_json_variant_by_index(value_, index))); }
    Json get(const char *key) const { return Json(sd_json_variant_ref(sd_json_variant_by_key(value_, key))); }
    std::string dump() const;
    void set(const char *key, const Json &value);
};
}
