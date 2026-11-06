# Meridian engines

Settings → Engines contains exactly four automatically saved switches: Font,
GPU, CPU and Audio. No advanced profiles, sliders, diagnostics or save button
are exposed. Font is enabled by default; GPU, CPU and Audio tuning are opt-in.
Older profile fields remain in the persisted protocol for compatibility but are
normalized to fixed presets. Existing font-family preferences are preserved.

## Font

The custom C++ Fontconfig/FreeType policy uses grayscale antialiasing and full
hinting. The scoped, content-addressed profile is loaded before GTK and WebKit
start. System/user font configuration is included, not replaced. Turning the
engine off retains platform rendering. Changes need a new Meridian session.
Font selection, display scaling and the panel still affect perceived sharpness.
Meridian does not replace other applications' text renderers.

## GPU

The C++ graphics policy requests WebKit accelerated compositing and larger icon
artwork. Turning it off restores WebKit's default policy, not a disabled GPU.
Hardware acceleration depends on drivers and workload. This does not overclock,
change monitor modes or replace labwc. Changes need a new session so WebViews
agree; the explicit nested-test software override remains authoritative.

## CPU

The C++ CPU engine uses Linux `PR_SET_TIMERSLACK` to allow at least 1 ms of timer
coalescing on the initial shell thread, inherited by subsequently created
threads/processes. An already larger inherited slack is preserved. It also
selects Meridian's reduced visual workload (no animations/transparency and
smaller icon requests). No governor, CPU clock, affinity, realtime priority or
system-wide scheduler changes are made. This is a workload/energy tradeoff,
not a guaranteed speed increase. Native timer changes need a new session;
active/pending policy is reported separately. Kernel failures are shown in
status while the visual policy can still apply.

## Audio

The custom C++ policy core generates PipeWire high-quality resampling settings
(`resample.quality = 10`) for native client, realtime-client and PulseAudio
compatibility streams. Rust applies the generated text through three private,
Meridian-owned `90-meridian-audio.conf` drop-ins below
`$XDG_CONFIG_HOME/pipewire/{client.conf.d,client-rt.conf.d,pipewire-pulse.conf.d}`.
It never replaces existing PipeWire configuration, forces a sample rate,
raises volume, adds equalization or changes Bluetooth codecs.

PipeWire supplies the actual optimized DSP/resampler; this is a native policy
engine, not a new mixer, audio server or custom resampling algorithm. Higher
resampling quality can use more CPU and cannot restore lost recording detail
or bypass hardware/codec limitations. No audible improvement is guaranteed.
Other later configuration fragments can override the profile.

Toggling Audio writes/removes only exact Meridian-owned profiles. Modified
files are preserved with an error. Partial updates are rolled back; failures
are returned to Settings. Configuration persistence failure also triggers a
rollback attempt, with rollback failures logged. Audio services are not
restarted automatically: reopen native playback apps, and restart the user
pipewire-pulse service or reboot for PulseAudio clients. This avoids abruptly
interrupting playback. Status reports profile configuration, not measured DSP
usage. Turning off removes the profile with the same restart requirements.

## Precompiled native libraries

Four architecture-specific static libraries are shipped under
`shell/native/prebuilt/x86_64`: font, GPU, CPU and audio. Their combined archive
size is about 21 KB (system libraries excluded). They are linked into the
shell, with no extra daemons, interpreters or runtime compiler. Font and GPU
reuse existing Fontconfig/FreeType/Pango/Cairo dependencies.

`build.rs` verifies source/archive checksums before using a native-host bundle;
stale, unavailable or cross-architecture bundles fall back to source compilation.
`bash tools/package-engines.sh` refreshes the four libraries sequentially using
size optimization, no debug symbols and deterministic archives. Normal release
compilation refreshes them before recording the source manifest. The source
release integrity manifest covers native sources and precompiled libraries.
Only x86_64 is bundled at present; the script also supports building on aarch64.

No UI build, full shell build, automated tests or audio-quality benchmarks were
run for this change. Only the requested native libraries were compiled.

## References

- [PipeWire client stream properties](https://docs.pipewire.org/page_man_pipewire-client_conf_5.html)
- [PipeWire PulseAudio stream configuration](https://docs.pipewire.org/page_man_pipewire-pulse_conf_5.html)
- [Linux timer slack](https://man7.org/linux/man-pages/man2/pr_set_timerslack.2const.html)
- [Fontconfig rendering properties](https://fontconfig.pages.freedesktop.org/fontconfig/fontconfig-user.html)
- [WebKit acceleration policy](https://webkitgtk.org/reference/webkit2gtk/stable/property.Settings.hardware-acceleration-policy.html)
