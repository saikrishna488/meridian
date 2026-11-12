#include "services/audio.hpp"
#include <stdexcept>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("audio contract failed"); }
template<class Action> void rejects(Action action) {
    try { action(); } catch (const Error &) { return; }
    throw std::runtime_error("unsafe audio action accepted");
}
int main() {
    const std::string fixture = R"([
      {"id":1,"type":"PipeWire:Interface:Metadata","props":{"metadata.name":"default"},"metadata":[
        {"subject":0,"key":"default.audio.sink","value":"{\"name\":\"speaker\"}"},
        {"subject":0,"key":"default.audio.source","value":{"name":"microphone"}}]},
      {"id":10,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Sink","node.name":"speaker","node.description":"Speakers"}}},
      {"id":11,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Source","node.name":"microphone"}}},
      {"id":12,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Source","node.name":"speaker.monitor"}}},
      {"id":13,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Source","device.class":"monitor","node.name":"monitor"}}},
      {"id":20,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Stream/Output/Audio","node.name":"music","application.name":"Player"},"params":{"Props":[{"channelVolumes":[0.125,0.064],"mute":true}]}}},
      {"id":21,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Video/Source","node.name":"camera"}}},
      {"id":4294967296,"type":"PipeWire:Interface:Node","info":{"props":{"media.class":"Audio/Sink","node.name":"overflow"}}}
    ])";
    std::vector<std::vector<std::string>> calls;
    const AudioCommand run = [&](const std::vector<std::string> &args) -> std::string {
        calls.push_back(args);
        if (args[0] == "pw-dump") return fixture;
        if (args[0] == "wpctl" && args[1] == "get-volume") return args[2] == "10" ? "Volume: 0.42 [MUTED]" : "Volume: 1.25";
        if (args == std::vector<std::string>{"pactl", "list", "short", "modules"}) return "5\tmodule-null-sink\tsink_name=meridian_virtual_output\n6\tmodule-null-sink\tsink_name=other\n7\tmodule-null-sink\tsink_name=meridian_virtual_output_other\n";
        return "";
    };
    const auto state = audio_state(run);
    require(state.get("devices").size() == 2 && state.get("streams").size() == 1);
    require(state.get("output_volume").number() == 42 && state.get("output_muted").boolean());
    require(state.get("input_volume").number() == 100 && !state.get("input_muted").boolean());
    require(state.get("streams").at(0).get("volume").number() == 50);
    const auto action = [](unsigned id, const char *name, const char *command, const Json &percent = {}) {
        return Json::object({{"id", Json::unsigned_number(id)}, {"name", Json::string(name)},
            {"action", Json::string(command)}, {"percent", percent}});
    };
    calls.clear();
    audio_action(action(10, "speaker", "volume", Json::unsigned_number(75)), run);
    require(calls.at(1) == std::vector<std::string>{"wpctl", "set-volume", "10", "75%"});
    calls.clear();
    rejects([&] { audio_action(action(10, "reused-name", "mute"), run); });
    require(calls.size() == 1 && calls[0][0] == "pw-dump");
    rejects([&] { audio_action(action(20, "music", "default"), run); });
    rejects([&] { audio_action(action(10, "speaker", "volume", Json::unsigned_number(101)), run); });
    rejects([&] { audio_action(action(10, "speaker", "mute", Json::unsigned_number(25)), run); });
    rejects([&] { audio_action(action(10, "speaker", "unknown"), run); });
    calls.clear();
    audio_action(action(20, "music", "unmute"), run);
    require(calls.at(1) == std::vector<std::string>{"wpctl", "set-mute", "20", "0"});
    calls.clear();
    audio_action(action(0, "", "virtual-remove"), run);
    require(calls.at(1) == std::vector<std::string>{"pactl", "unload-module", "5"});
    require(calls.at(2) == std::vector<std::string>{"pw-dump", "-N"});
    const AudioCommand unavailable = [](const auto &) -> std::string { throw Error("unavailable", "Missing service."); };
    rejects([&] { audio_state(unavailable); });
    const AudioCommand malformed = [](const auto &) { return "{}"; };
    rejects([&] { audio_state(malformed); });
    const AudioCommand invalid_volume = [&](const auto &args) { return args[0] == "pw-dump" ? fixture : "Volume: nan"; };
    require(audio_state(invalid_volume).get("output_volume").is_null());
}
