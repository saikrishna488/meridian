#pragma once
#include "services/services.hpp"
#include <memory>

namespace meridian {
// Backend-owned jobs; destruction cancels and joins every worker.
class Transfers {
    struct State;
    std::unique_ptr<State> state_;
public:
    Transfers();
    ~Transfers();
    std::optional<Json> dispatch(const Request &, GCancellable *);
};
}
