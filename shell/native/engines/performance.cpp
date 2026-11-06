#include "engines.h"
#include <cstdio>
#include <cstring>
#include <unistd.h>

extern "C" int meridian_performance_sample(MeridianEngineMetrics *output) {
    if (!output) return -1;
    *output = {};
    output->memory_pressure_avg10 = -1;
    const long threads = sysconf(_SC_NPROCESSORS_ONLN);
    output->cpu_threads = threads > 0 ? static_cast<uint32_t>(threads) : 0;
    FILE *memory = std::fopen("/proc/meminfo", "r");
    if (!memory) return -1;
    char line[256];
    while (std::fgets(line, sizeof(line), memory)) {
        unsigned long long value = 0;
        if (std::sscanf(line, "MemTotal: %llu kB", &value) == 1) output->memory_total_kib = value;
        if (std::sscanf(line, "MemAvailable: %llu kB", &value) == 1) output->memory_available_kib = value;
    }
    std::fclose(memory);
    FILE *pressure = std::fopen("/proc/pressure/memory", "r");
    if (pressure) {
        while (std::fgets(line, sizeof(line), pressure)) {
            double value = -1;
            if (std::sscanf(line, "some avg10=%lf", &value) == 1) output->memory_pressure_avg10 = value;
        }
        std::fclose(pressure);
    }
    return output->memory_total_kib ? 0 : -1;
}

#include <sys/prctl.h>
// Coalesce this thread's timers by at most 1 ms; no privileged scheduling,
// CPU governor or realtime-priority changes. New threads inherit this value.
extern "C" int meridian_cpu_activate(void) {
    const long current = prctl(PR_GET_TIMERSLACK, 0UL, 0UL, 0UL, 0UL);
    if (current < 0) return -1;
    // Never reduce a larger timer slack already chosen by the parent/session.
    const unsigned long slack = current > 1000000L ? static_cast<unsigned long>(current) : 1000000UL;
    return prctl(PR_SET_TIMERSLACK, slack, 0UL, 0UL, 0UL);
}
