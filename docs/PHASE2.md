# Alpha 0.3 performance work

Status: implementation and nested desktop evidence; **not a qualified Alpha 0.3 release**. Phase 1 hardware/security qualification remains required. The workspace version remains unchanged until release qualification.

## Implemented

- Orbit, Settings and Terminal construct their WebViews on first use and reuse existing windows. Nova, Calendar, Clock and Pulse retain their existing lazy construction. Desktop menus and Notifications are now lazy too.
- The dock, top bar and monitor wallpapers load first. Launcher and Control Center prewarm after one second. Surface isolation, per-surface capabilities, crash recovery and independent labwc ownership remain intact.
- Production ESM bundles share React, bridge code and common components. The build emits dependency and size manifests. Lock preflight validates the locker’s shared modules, including canonical path containment. Watch builds regenerate these manifests too.
- Application catalog changes debounce for 250 ms; Desktop folder notifications debounce for 150 ms. Initial GTK appearance/font setup reads the desktop configuration once. Hidden launcher invalidations no longer run searches.
- Menus show and focus while optional service refreshes proceed. Hidden power-status hooks skip bridge requests. Clock pauses its rendering timer when hidden, refreshes on visibility, and uses one-second ticks except for a running visible stopwatch. Pulse already pauses its sampling when hidden.
- Native application icon path resolution uses a 256-entry LRU. Encoded icon payloads have a separate 128-entry / 8 MiB budget and file metadata validation. Application catalog and icon-theme changes invalidate caches. The owner disconnects theme subscriptions on destruction.
- Orbit’s thumbnail cache is bounded by 128 entries and approximately 4 MiB of UTF-16 image-string payload, uses LRU eviction, and deduplicates pending requests. Its existing three-request concurrency limit remains. Browser image buffers and map/key overhead are additional allocations.
- Terminal and Settings request handlers live in feature modules under `shell/src/shell/`; surface construction/reuse is in `surfaces.rs`. Capability authorization, session lock gating and transfer/session safety stay in the parent dispatcher. Native services retain existing modules and can be tested without creating a WebView.
- Desktop configuration accepts legacy JSON and writes a version-one envelope on the next successful save. Unsupported versions cannot be overwritten. Corrupt backups and atomic private writes remain. Other configuration stores retain their existing formats.

## Measurement

`tools/profile-desktop.py` emits JSON Lines with per-process RSS, PSS, CPU and main-thread context-switch deltas. CPU percentage uses one logical CPU as 100%. Context switches are a proxy, not a count of idle wakeups. New/reused PIDs are marked unmeasured for their first CPU interval. Missing PSS is reported explicitly. The observer excludes itself.

Select the shell and compositor explicitly, adding roots for background services outside their descendant trees:

```sh
python3 tools/profile-desktop.py --shell-pid SHELL_PID --compositor-pid LABWC_PID \
  --service-pid BACKGROUND_SERVICE_PID --samples 300 --interval 1 > desktop-idle.jsonl
```

`shell` includes the shell and all its descendants: WebKit renderers, network helpers, sandbox helpers, and built-in terminal children. `desktop_stack` is the union of that tree, compositor descendants and explicitly supplied service trees. Overlapping roots count once. Unselected session services and the rest of the user’s desktop are excluded. Record selected roots with every report; this is not a machine-wide RAM metric. RSS double-counts shared mappings; use PSS for comparisons, and keep competing desktops/workloads constant because PSS attribution changes when processes share pages.

Use the same build profile, UI production bundle, monitor count, scale, renderer and service setup before and after. Measure a fresh session after login, then separately after opening applications. Do not label retained application memory as an idle-login regression. The nested test emits cold UI handshake timing, idle per-process samples, first/warm menu visibility timing and window/process counts around reuse cycles. These are observed timings, not release gates. UI handshake is not login-to-interactive or completion of an animation.

```sh
MERIDIAN_SOAK_SECONDS=60 bash tools/tests/wayland.sh
# Longer qualification (run on an otherwise idle test system):
MERIDIAN_SOAK_SECONDS=14400 bash tools/tests/wayland.sh
```

The test uses two virtual outputs, Cairo/pixman/software WebKit and unavailable optional system services. The fixture forces software rendering for every surface. Production policy is unchanged: hardware acceleration remains enabled except for Terminal’s existing per-surface exception. CI exercises module loading, lazy creation/reuse, crash isolation, lock fallback and shell restart independently of normal application windows. It does not establish Intel GPU performance.

## Recorded nested observations (2026-10-18)

The [raw process samples](performance/alpha-0.3-nested.json) preserve per-process
measurements and collection boundaries. The Phase 1 working tree was snapshotted
before these optimizations; baseline and final fixtures ran sequentially.

| Metric | Phase 1 snapshot | Optimized working tree |
| --- | ---: | ---: |
| Settled shell tree PSS | 625.1 MiB | 414.5 MiB |
| Settled shell tree processes, observer excluded | 68 | 38 |
| Initial native windows | 11 | 6 |
| Unique production JavaScript bytes, no source maps | 3,707,862 | 703,491 |

Shell tree PSS fell **33.7%** in this single comparison; JavaScript bytes fell
**81.0%**. Baseline and optimized rendering processes still have independent
contexts and heaps. The optimized selected compositor + shell trees used
438.0 MiB PSS; external session services are excluded. No corresponding baseline
stack total was collected, so only shell-tree totals are compared above.

The optimized launcher took 700 ms on its first prewarmed opening and 229 ms on a
repeat opening. Control Center took 98 ms and 66 ms respectively. These timings
stop when the menu’s hidden class is removed, before animation completion. The
launcher target is **not met** by this fixture; further profiling remains required.
The optimized test reached its initial handshake gate in 1,862 ms. That gate
waits for the dock, top bar and already-created menus; the menu creation sets
differ between versions, so these handshake timings are not a controlled
startup comparison. They do not measure wallpaper readiness or first paint.

Two short CPU intervals rounded to zero CPU ticks for the selected processes;
this is not evidence for the sustained laptop idle target.

The final strict nested test passed 50 application reuse cycles and a 60-second
soak: native windows stayed at nine, terminal sessions returned to zero, and the
shell tree stayed at 56 processes. Shell RSS changed from 290,184 to 287,672 KiB;
summed tree RSS changed from 2,305,420 to 2,299,596 KiB. This short observation
does not prove absence of leaks. Deliberate renderer failures, two-output lock
fallback and an actual shell SIGKILL/restart also passed; an independent GTK
application survived the shell restart. Formatting, Clippy, 197 Rust tests,
86 UI tests and two profiler regression tests passed. Four Rust tests are
ignored in the ordinary test run; nested coverage runs through the smoke binary,
and image/GVfs cases still require desktop services.

## Qualification still required

| Area | Remaining evidence/work |
| --- | --- |
| Login RAM | Repeated release-build measurements on the i5-8250U; include labwc and all background services. Confirm the proposed 20–30% reduction there. |
| Idle CPU | At least five minutes after settling, with real power/audio/network services; target below 1–2% on the test laptop. Profile timer/interrupt wakeups separately. |
| Responsiveness | First and repeated launcher/search/Control Center interactions, including missing/slow services. Target launcher <100 ms and Control Center <150 ms for typical interactions. |
| Startup | Compositor-readiness to usable desktop measurement in a real greetd session; target <3 seconds. Headless handshake timing has different boundaries. |
| GPU/rendering | Intel integrated GPU traces, window switching and animations at 60/120 Hz; 1x, 1.25x, 1.5x and 2x scale; multiple monitors and reduced motion. Profile before changing blur/transparency or GPU policy. |
| Long sessions | Four-hour and overnight sessions, repeated open/close, cached image churn, suspend/resume, monitor changes and WebKit failures. No sustained PSS/CPU increase. A short reuse test is not leak proof. |
| Architecture | Further feature dispatcher extraction, standardized service cancellation/ownership interfaces and migration of other stores. Catalog rebuilds still run synchronously after debouncing; move them only with a thread-safe DTO design rather than sharing GIO objects across threads. |
| Hidden surfaces | Cached apps preserve UI state on close. Generic idle eviction needs per-app persistence and transfer/unsaved-state guards before it can safely reclaim all closed WebViews. |

Reduced-motion styles and Pulse visibility gating were already present. No GPU frame-rate, real-hardware CPU, long-duration stability or install qualification is claimed by these changes.
