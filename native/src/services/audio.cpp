#include "services/audio.hpp"
#include "core/process.hpp"
#include "core/text.hpp"
#include <algorithm>

namespace meridian {
namespace {
std::string default_name(const Json &objects, std::string_view key) {
    for (size_t i = 0; i < objects.size(); ++i) {
        const auto object = objects.at(i);
        if (!object.get("type").is_string() || object.get("type").text() != "PipeWire:Interface:Metadata"
            || !object.get("props").get("metadata.name").is_string()
            || object.get("props").get("metadata.name").text() != "default") continue;
        const auto metadata = object.get("metadata");
        if (!metadata.is_array()) continue;
        for (size_t j = 0; j < metadata.size(); ++j) {
            const auto entry = metadata.at(j);
            if (!entry.get("subject").is_number() || entry.get("subject").number() != 0
                || !entry.get("key").is_string() || entry.get("key").text() != key) continue;
            auto value = entry.get("value");
            if (value.is_string()) {
                try { value = Json::parse(value.text()); } catch (const Error &) { continue; }
            }
            if (value.get("name").is_string()) return std::string(value.get("name").text());
        }
    }
    return {};
}
std::string string_or(const Json &value, std::string_view fallback) {
    return std::string(value.is_string() ? value.text() : fallback);
}
Json dump(const AudioCommand &run) {
    const auto data = Json::parse(run({"pw-dump", "-N"}));
    if (!data.is_array()) throw Error("failed", "Could not read PipeWire audio devices.");
    return data;
}
std::optional<std::pair<unsigned, bool>> volume(std::string_view text) {
    text = trim(text);
    if (!text.starts_with("Volume:")) return std::nullopt;
    text = trim(text.substr(7));
    const auto number = parse_number<double>(text.substr(0, text.find_first_of(" \t\n")));
    if (!number || *number < 0) return std::nullopt;
    return std::pair{static_cast<unsigned>(std::clamp(std::round(*number * 100), 0.0, 100.0)), text.find("[MUTED]") != std::string_view::npos};
}
}
Json audio_devices(const Json &objects) {
    if (!objects.is_array()) throw Error("failed", "Invalid PipeWire inventory.");
    const auto output = default_name(objects, "default.audio.sink"), input = default_name(objects, "default.audio.source");
    std::vector<Json> result;
    for (size_t i = 0; i < objects.size(); ++i) {
        const auto object = objects.at(i), props = object.get("info").get("props");
        if (!object.get("type").is_string() || object.get("type").text() != "PipeWire:Interface:Node") continue;
        const auto media = string_or(props.get("media.class"), "");
        if (media != "Audio/Sink" && media != "Audio/Source") continue;
        const auto id = object.get("id"), node = props.get("node.name");
        if (!id.is_integer() || id.number() < 0 || id.number() > UINT32_MAX || !node.is_string()) continue;
        const std::string name(node.text());
        const bool sink = media == "Audio/Sink";
        if (!sink && (name.ends_with(".monitor") || string_or(props.get("device.class"), "") == "monitor")) continue;
        result.push_back(Json::object({{"id", id}, {"name", node},
            {"description", Json::string(string_or(props.get("node.description"), string_or(props.get("node.nick"), name)))},
            {"kind", Json::string(sink ? "output" : "input")}, {"default", Json::boolean(name == (sink ? output : input))}}));
    }
    std::stable_sort(result.begin(), result.end(), [](const Json &a, const Json &b) {
        if (a.get("kind").text() != b.get("kind").text()) return a.get("kind").text() < b.get("kind").text();
        const std::string first(a.get("description").text()), second(b.get("description").text());
        GMemory<char> left(g_utf8_strdown(first.c_str(), -1)), right(g_utf8_strdown(second.c_str(), -1));
        return std::string_view(left.get()) < std::string_view(right.get());
    });
    return Json::array(result);
}
Json audio_streams(const Json &objects) {
    std::vector<Json> streams;
    for (size_t i = 0; i < objects.size() && streams.size() < 64; ++i) {
        const auto object = objects.at(i), props = object.get("info").get("props");
        if (string_or(object.get("type"), "") != "PipeWire:Interface:Node"
            || string_or(props.get("media.class"), "") != "Stream/Output/Audio") continue;
        const auto id = object.get("id"), name = props.get("node.name");
        if (!id.is_integer() || id.number() < 0 || id.number() > UINT32_MAX || !name.is_string()) continue;
        const auto params = object.get("info").get("params").get("Props");
        const auto values = params.is_array() && params.size() ? params.at(0) : Json{};
        const auto channels = values.get("channelVolumes");
        std::optional<double> maximum;
        if (channels.is_array()) for (size_t j = 0; j < channels.size(); ++j) {
            const auto channel = channels.at(j);
            if (channel.is_number() && std::isfinite(channel.number())) maximum = maximum ? std::max(*maximum, channel.number()) : channel.number();
        }
        streams.push_back(Json::object({{"id", id}, {"name", name},
            {"description", Json::string(string_or(props.get("application.name"), string_or(props.get("node.description"), "Playback stream")))},
            {"volume", maximum && *maximum >= 0 ? Json::unsigned_number(std::clamp(std::round(std::cbrt(*maximum) * 100), 0.0, 100.0)) : Json{}},
            {"muted", values.get("mute").is_bool() ? values.get("mute") : Json{}}}));
    }
    return Json::array(streams);
}
Json audio_state(const AudioCommand &run) {
    const auto objects = dump(run), devices = audio_devices(objects), streams = audio_streams(objects);
    Json state = Json::object({{"devices", devices}, {"streams", streams}, {"output_volume", Json{}},
        {"input_volume", Json{}}, {"output_muted", Json{}}, {"input_muted", Json{}}});
    for (size_t i = 0; i < devices.size(); ++i) {
        const auto device = devices.at(i);
        if (!device.get("default").boolean()) continue;
        std::optional<std::pair<unsigned, bool>> value;
        try { value = volume(run({"wpctl", "get-volume", std::to_string(static_cast<uint32_t>(device.get("id").number()))})); }
        catch (const Error &error) { if (error.code == "unavailable" && std::string_view(error.what()) == "Request cancelled.") throw; }
        if (value) {
            const auto kind = std::string(device.get("kind").text());
            state.set((kind + "_volume").c_str(), Json::unsigned_number(value->first));
            state.set((kind + "_muted").c_str(), Json::boolean(value->second));
        }
    }
    return state;
}
Json audio_action(const Json &choice, const AudioCommand &run) {
    const auto action = choice.get("action").text();
    const auto percent = choice.get("percent");
    if (action == "virtual-create" || action == "virtual-remove") {
        if (choice.get("id").number() != 0 || !choice.get("name").text().empty() || !percent.is_null()) invalid("Invalid virtual output request.");
        const auto modules = run({"pactl", "list", "short", "modules"});
        std::vector<std::string> ids;
        for (auto line : lines(modules)) {
            const auto token = [&]() {
                line = trim(line);
                const auto end = line.find_first_of(" \t");
                const auto result = line.substr(0, end);
                line = end == std::string_view::npos ? std::string_view{} : line.substr(end);
                return result;
            };
            const auto id = token();
            if (!parse_number<uint32_t>(id) || token() != "module-null-sink") continue;
            bool owned = false;
            while (!trim(line).empty()) owned |= token() == "sink_name=meridian_virtual_output";
            if (owned) ids.emplace_back(id);
        }
        if (action == "virtual-create" && ids.empty()) run({"pactl", "load-module", "module-null-sink", "sink_name=meridian_virtual_output", "sink_properties=device.description=MarsOS-Virtual-Output"});
        if (action == "virtual-remove") for (const auto &id : ids) run({"pactl", "unload-module", id});
        return audio_state(run);
    }
    if ((action != "default" && action != "mute" && action != "unmute" && action != "volume")
        || (!percent.is_null() && percent.number() > 100) || (action == "volume") != !percent.is_null()) invalid("Invalid audio control.");
    const auto objects = dump(run);
    bool found = false;
    for (const auto &items : {audio_devices(objects), audio_streams(objects)}) for (size_t i = 0; i < items.size(); ++i) {
        const auto item = items.at(i);
        if (item.get("id").number() == choice.get("id").number() && item.get("name").text() == choice.get("name").text()
            && (action != "default" || item.get("kind").is_string())) found = true;
    }
    if (!found) throw Error("not_found", "Audio device disconnected. Refresh and choose another device.");
    const auto id = std::to_string(static_cast<uint32_t>(choice.get("id").number()));
    if (action == "default") run({"wpctl", "set-default", id});
    else if (action == "volume") run({"wpctl", "set-volume", id, std::to_string(static_cast<unsigned>(percent.number())) + "%"});
    else run({"wpctl", "set-mute", id, action == "mute" ? "1" : "0"});
    return audio_state(run);
}
std::optional<Json> dispatch_audio(const Request &request, ServiceContext &context) {
    if (request.method != "settings.audio" && request.method != "settings.audio_action") return std::nullopt;
    const AudioCommand run = [&](const std::vector<std::string> &args) {
        return run_checked(args, context.cancel, std::chrono::seconds(5));
    };
    if (request.method == "settings.audio") return audio_state(run);
    return audio_action(request.params, run);
}
}
