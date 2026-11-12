#pragma once
#include "services/services.hpp"
#include <functional>

namespace meridian {
using AudioCommand = std::function<std::string(const std::vector<std::string> &)>;
Json audio_devices(const Json &objects);
Json audio_streams(const Json &objects);
Json audio_state(const AudioCommand &run);
Json audio_action(const Json &choice, const AudioCommand &run);
std::optional<Json> dispatch_audio(const Request &, ServiceContext &);
}
