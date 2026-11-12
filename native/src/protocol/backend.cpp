#include "meridian_backend.h"
#include "services/services.hpp"
#include "services/transfers.hpp"
#include <algorithm>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <deque>
#include <mutex>
#include <thread>

namespace meridian {
namespace {
using Clock = std::chrono::steady_clock;
std::string failure(std::string_view code, std::string_view message) {
    return Json::object({{"ok", Json::boolean(false)}, {"error", Json::object({
        {"code", Json::string(code)}, {"message", Json::string(message)}})}}).dump();
}
struct Task {
    std::mutex mutex;
    std::condition_variable ready;
    std::string reply;
    bool done = false;
    GObjectPtr<GCancellable> cancel{g_cancellable_new()};
    void complete(std::string value) {
        { std::lock_guard lock(mutex); reply = std::move(value); done = true; }
        ready.notify_all();
    }
};
struct Client {
    const std::string surface;
    std::mutex mutex;
    bool alive = true;
    size_t pending = 0;
    unsigned requests = 0;
    Clock::time_point window = Clock::now();
    std::vector<std::weak_ptr<Task>> tasks;
    explicit Client(std::string surface) : surface(std::move(surface)) {}
    void close() {
        std::lock_guard lock(mutex);
        alive = false;
        for (auto &weak : tasks) if (auto task = weak.lock()) g_cancellable_cancel(task->cancel.get());
    }
};
struct Input {
    std::string text;
    explicit Input(std::string_view value) : text(value) {}
    ~Input() { if (!text.empty()) explicit_bzero(text.data(), text.size()); }
};
struct Job { std::shared_ptr<Client> client; std::shared_ptr<Task> task; std::shared_ptr<Input> input; };
class Backend {
    ConfigStore config_;
    Transfers transfers_;
    std::mutex mutex_;
    std::condition_variable wake_;
    bool stopping_ = false;
    size_t pending_ = 0;
    std::deque<Job> queue_;
    std::vector<std::weak_ptr<Task>> tasks_;
    std::vector<std::thread> workers_;
    void run() {
        while (true) {
            Job job;
            {
                std::unique_lock lock(mutex_);
                wake_.wait(lock, [&] { return stopping_ || !queue_.empty(); });
                if (queue_.empty()) return;
                job = std::move(queue_.front()); queue_.pop_front();
            }
            std::string reply;
            try {
                check_cancelled(job.task->cancel.get());
                const auto request = parse_request(job.input->text, job.client->surface);
                ServiceContext context{config_, job.task->cancel.get(), &transfers_};
                const auto result = dispatch_request(request, context, job.client->surface);
                reply = Json::object({{"ok", Json::boolean(true)}, {"result", result}}).dump();
            } catch (const Error &error) {
                // Never log request bodies or include parser-supplied input.
                try { reply = failure(error.code, error.what()); } catch (...) {}
            } catch (...) {
                try { reply = failure("failed", "Native service operation failed."); } catch (...) {}
            }
            if (reply.empty()) reply = R"({"ok":false,"error":{"code":"failed","message":"Native allocation failed."}})";
            // Release admission before waking the caller, so a completed task
            // can immediately be followed by another without a transient limit.
            {
                std::scoped_lock lock(mutex_, job.client->mutex);
                --pending_; --job.client->pending;
            }
            job.input.reset();
            job.task->complete(std::move(reply));
        }
    }
public:
    explicit Backend(std::filesystem::path root) : config_(std::move(root)) {
        try { for (unsigned i = 0; i < 2; ++i) workers_.emplace_back([this] { run(); }); }
        catch (...) { close(); throw; }
    }
    ~Backend() { close(); }
    void close() {
        {
            std::lock_guard lock(mutex_);
            stopping_ = true;
            for (auto &weak : tasks_) if (auto task = weak.lock()) g_cancellable_cancel(task->cancel.get());
        }
        wake_.notify_all();
        for (auto &worker : workers_) if (worker.joinable()) worker.join();
    }
    std::shared_ptr<Task> submit(const std::shared_ptr<Client> &client, std::string_view input) {
        auto task = std::make_shared<Task>();
        std::scoped_lock lock(mutex_, client->mutex);
        const auto now = Clock::now();
        if (now - client->window >= std::chrono::seconds(1)) { client->window = now; client->requests = 0; }
        if (stopping_ || !client->alive) task->complete(failure("unavailable", "Client or backend is closed."));
        else if (++client->requests > 256 || client->pending >= 16 || pending_ >= 128)
            task->complete(failure("unavailable", "Too many requests; try again shortly."));
        else if (input.size() > 2 * 1024 * 1024)
            task->complete(failure("invalid_request", "Request exceeds the 2 MiB limit."));
        else {
            const auto completed = [](const auto &weak) {
                const auto task = weak.lock();
                if (!task) return true;
                std::lock_guard lock(task->mutex);
                return task->done;
            };
            std::erase_if(tasks_, completed);
            std::erase_if(client->tasks, completed);
            tasks_.push_back(task);
            client->tasks.push_back(task);
            queue_.push_back({client, task, std::make_shared<Input>(input)});
            ++pending_; ++client->pending;
            wake_.notify_one();
        }
        return task;
    }
};
}
}
struct meridian_backend { std::shared_ptr<meridian::Backend> value; };
struct meridian_client { std::shared_ptr<meridian::Backend> backend; std::shared_ptr<meridian::Client> value; };
struct meridian_task { std::shared_ptr<meridian::Task> value; };
extern "C" {
meridian_backend *meridian_backend_create(const char *directory) {
    try {
        auto handle = std::make_unique<meridian_backend>();
        auto root = directory ? std::filesystem::path(directory) : std::filesystem::path(g_get_user_config_dir()) / "meridian";
        handle->value = std::make_shared<meridian::Backend>(std::move(root));
        return handle.release();
    } catch (...) { return nullptr; }
}
void meridian_backend_destroy(meridian_backend *backend) {
    if (!backend) return;
    backend->value->close(); delete backend;
}
meridian_client *meridian_client_create(meridian_backend *backend, const char *surface) {
    if (!backend || !surface) return nullptr;
    try {
        meridian::surface_info(surface);
        auto client = std::make_unique<meridian_client>();
        client->backend = backend->value;
        client->value = std::make_shared<meridian::Client>(surface);
        return client.release();
    } catch (...) { return nullptr; }
}
void meridian_client_destroy(meridian_client *client) {
    if (!client) return;
    client->value->close(); delete client;
}
meridian_task *meridian_client_submit(meridian_client *client, const char *json, size_t length) {
    if (!client || !json) return nullptr;
    try {
        auto task = std::make_unique<meridian_task>();
        task->value = client->backend->submit(client->value, std::string_view(json, length));
        return task.release();
    } catch (...) { return nullptr; }
}
int meridian_task_wait(meridian_task *task, unsigned timeout_ms) {
    if (!task) return 0;
    try {
        std::unique_lock lock(task->value->mutex);
        return task->value->ready.wait_for(lock, std::chrono::milliseconds(std::min(timeout_ms, 60000u)), [&] { return task->value->done; });
    } catch (...) { return 0; }
}
const char *meridian_task_reply(meridian_task *task) {
    if (!task) return nullptr;
    std::lock_guard lock(task->value->mutex);
    return task->value->done ? task->value->reply.c_str() : nullptr;
}
void meridian_task_cancel(meridian_task *task) { if (task) g_cancellable_cancel(task->value->cancel.get()); }
void meridian_task_destroy(meridian_task *task) { if (task) { meridian_task_cancel(task); delete task; } }
}
