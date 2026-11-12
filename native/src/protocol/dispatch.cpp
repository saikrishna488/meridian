#include "services/services.hpp"
#include "services/transfers.hpp"
#include "services/trash.hpp"
#include "services/audio.hpp"

namespace meridian {
Json dispatch_request(const Request &request, ServiceContext &context, std::string_view surface) {
    check_cancelled(context.cancel);
    if (request.method == "shell.hello") return surface_info(surface);
    if (context.transfers) if (auto result = context.transfers->dispatch(request, context.cancel)) return *result;
    if (auto result = dispatch_trash(request, context)) return *result;
    if (auto result = dispatch_audio(request, context)) return *result;
    if (auto result = dispatch_stores(request, context)) return *result;
    if (auto result = dispatch_files(request, context)) return *result;
    if (auto result = dispatch_applications(request, context)) return *result;
    if (auto result = dispatch_startup(request, context)) return *result;
    if (auto result = dispatch_system(request, context)) return *result;
    throw Error("unavailable", "This service has not been ported to the native backend yet.");
}
}
