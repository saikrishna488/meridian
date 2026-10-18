//! Run only inside tools/tests/wayland.sh's disposable compositor and bus.
use super::*;
use std::cell::Cell;
use std::time::Instant;
use webkit6::prelude::*;

fn wait_until(label: &str, mut ready: impl FnMut() -> bool) {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out: {label}");
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn eval(surface: &WebSurface, script: &str) -> String {
    let result = Rc::new(RefCell::new(None));
    let output = result.clone();
    surface.view.evaluate_javascript(script, None, None, gio::Cancellable::NONE, move |value| {
        *output.borrow_mut() = Some(value.map(|value| value.to_str().to_string()).map_err(|error| error.to_string()));
    });
    wait_until("JavaScript evaluation", || result.borrow().is_some());
    result.borrow_mut().take().unwrap().unwrap()
}
fn page_contains(surface: &WebSurface, text: &str) -> bool {
    eval(surface, &format!("document.body.textContent.includes({})", serde_json::to_string(text).unwrap())) == "true"
}
fn bridge_request(surface: &WebSurface, raw: &str) -> serde_json::Value {
    eval(
        surface,
        &format!(
            "window.__smokeReply=null; window.webkit.messageHandlers.meridian.postMessage({}).then(value => window.__smokeReply=value); 'sent'",
            serde_json::to_string(raw).unwrap()
        ),
    );
    wait_until("bridge reply", || eval(surface, "typeof window.__smokeReply") == "string");
    serde_json::from_str(&eval(surface, "window.__smokeReply")).unwrap()
}
fn tree_memory(pid: u32) -> (u64, u64) {
    let rss = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|data| {
            data.lines()
                .find(|line| line.starts_with("VmRSS:"))
                .and_then(|line| line.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .unwrap_or_default();
    let mut total = (1u64, rss);
    if let Ok(children) = std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")) {
        for child in children.split_whitespace().filter_map(|value| value.parse::<u32>().ok()) {
            let sample = tree_memory(child);
            total.0 += sample.0;
            total.1 += sample.1;
        }
    }
    total
}
fn rss() -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .unwrap_or_default()
        .to_string()
}

#[cfg_attr(test, test)]
#[cfg_attr(test, ignore = "requires tools/tests/wayland.sh's isolated headless compositor")]
pub(super) fn nested_wayland_smoke() {
    assert_eq!(std::env::var("MERIDIAN_NESTED_TEST").as_deref(), Ok("1"));
    gtk::init().unwrap();
    crate::preflight().unwrap();
    crate::install_css();
    let app = gtk::Application::builder()
        .application_id("org.meridian.SmokeTest")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(gio::Cancellable::NONE).unwrap();
    let started = Instant::now();
    let shell = Shell::start(&app, crate::paths::roots().unwrap(), true);
    wait_until("critical surface startup", || {
        let state = shell.state.borrow();
        state.panel.as_ref().is_some_and(|surface| surface.is_ready())
            && state.topbar.as_ref().is_some_and(|surface| surface.is_ready())
            && state.popovers.values().all(|surface| surface.is_ready())
    });
    println!(
        "performance_cold critical_ready_ms={} windows={} tree={:?} shell={:?}",
        started.elapsed().as_millis(),
        app.windows().len(),
        tree_memory(std::process::id()),
        rss()
    );
    if std::env::var_os("MERIDIAN_EXPECT_EAGER").is_none() {
        assert!(shell.state.borrow().finder.is_none());
        assert!(shell.state.borrow().terminal.is_none());
        assert!(shell.state.borrow().preferences.is_none());
    }
    glib::MainContext::default().block_on(glib::timeout_future(Duration::from_secs(5)));
    let profiler = std::path::Path::new(&std::env::var("MERIDIAN_TEST_ROOT").unwrap()).join("tools/profile-desktop.py");
    let pid = std::process::id().to_string();
    let profile = gio::Subprocess::newv(
        &[
            std::ffi::OsStr::new("python3"),
            profiler.as_os_str(),
            std::ffi::OsStr::new("--shell-pid"),
            std::ffi::OsStr::new(&pid),
            std::ffi::OsStr::new("--samples"),
            std::ffi::OsStr::new("3"),
            std::ffi::OsStr::new("--compositor-pid"),
            std::ffi::OsStr::new(&std::env::var("MERIDIAN_TEST_COMPOSITOR_PID").unwrap()),
        ],
        gio::SubprocessFlags::STDOUT_PIPE,
    )
    .unwrap();
    let (output, _) = glib::MainContext::default().block_on(profile.communicate_utf8_future(None)).unwrap();
    assert!(profile.is_successful());
    for line in output.unwrap().lines() {
        println!("performance_idle {line}");
    }
    let panel = shell.state.borrow().panel.clone().unwrap();
    let lost_panel = Rc::new(Cell::new(0));
    let count = lost_panel.clone();
    panel.view.connect_web_process_terminated(move |_, _| count.set(count.get() + 1));
    for popover in [Popover::Applications, Popover::Options] {
        let response_started = Instant::now();
        shell.open_popover(popover);
        let surface = shell.state.borrow().popovers[&popover].clone();
        wait_until("menu startup", || surface.is_ready());
        wait_until("menu visible", || {
            eval(&surface, "document.querySelector('#menu')?.classList.contains('hidden') === false") == "true"
        });
        println!("performance_menu surface={popover:?} visible_ms={}", response_started.elapsed().as_millis());
        shell.dismiss_popover(popover);
        wait_until("menu dismissed", || !surface.window.is_visible());
        let warm_started = Instant::now();
        shell.open_popover(popover);
        wait_until("warm menu visible", || {
            eval(&surface, "document.querySelector('#menu')?.classList.contains('hidden') === false") == "true"
        });
        println!("performance_menu surface={popover:?} warm_visible_ms={}", warm_started.elapsed().as_millis());
        for _ in 0..3 {
            let crashes = Rc::new(Cell::new(false));
            let crashed = crashes.clone();
            let handler = surface.view.connect_web_process_terminated(move |_, _| crashed.set(true));
            surface.view.terminate_web_process();
            wait_until("renderer termination", || crashes.get());
            wait_until("renderer recovery", || surface.is_ready());
            assert_eq!(lost_panel.get(), 0, "unrelated dock renderer was terminated");
            assert!(surface.window.is_visible());
            assert!(panel.is_ready());
            surface.view.disconnect(handler);
        }
        shell.hide_popover(popover);
    }
    assert_eq!(bridge_request(&panel, r#"{"method":"terminal.start"}"#)["error"]["code"], "permission_denied");
    assert_eq!(bridge_request(&panel, r#"{"method":"settings.users"}"#)["error"]["code"], "permission_denied");
    assert_eq!(bridge_request(&panel, "not-json")["error"]["code"], "invalid_request");
    drop(panel);
    // Change primary output through the real authorized bridge. Ordinary app
    // windows and the compositor remain independent of bar reconstruction.
    shell.open_preferences();
    let preferences = shell.state.borrow().preferences.clone().unwrap();
    wait_until("settings ready", || preferences.is_ready());
    let display = gdk::Display::default().unwrap();
    let second = display.monitors().item(1).unwrap().downcast::<gdk::Monitor>().unwrap();
    let connector = second.connector().unwrap().to_string();
    let request =
        serde_json::json!({"method":"settings.primary_display_set","params":{"connector":connector}}).to_string();
    assert_eq!(bridge_request(&preferences, &request)["ok"], true);
    assert_eq!(shell.state.borrow().primary.as_ref(), Some(&second));
    assert_eq!(
        bridge_request(
            &preferences,
            r#"{"method":"settings.primary_display_set","params":{"connector":"nonexistent-monitor"}}"#
        )["error"]["code"],
        "invalid_request"
    );
    assert_eq!(
        bridge_request(&preferences, r#"{"method":"settings.primary_display_set","params":{"connector":null}}"#)["ok"],
        true
    );
    wait_until("bars after primary change", || {
        let state = shell.state.borrow();
        state.panel.as_ref().is_some_and(|surface| surface.is_ready())
            && state.topbar.as_ref().is_some_and(|surface| surface.is_ready())
    });
    assert_eq!(
        bridge_request(
            &preferences,
            r#"{"method":"settings.search_set","params":{"settings":true,"calculator":true}}"#
        )["ok"],
        true
    );
    shell.open_popover(Popover::Applications);
    let launcher = shell.state.borrow().popovers[&Popover::Applications].clone();
    let results = bridge_request(&launcher, r#"{"method":"search.query","params":{"query":"2+3*4","serial":99}}"#);
    assert!(
        results["result"]["sections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|section| section["provider"] == "calculator" && section["items"][0]["title"] == "14")
    );
    let results = bridge_request(&launcher, r#"{"method":"search.query","params":{"query":"wifi","serial":100}}"#);
    assert!(results["result"]["sections"].as_array().unwrap().iter().any(|section| section["provider"] == "settings"));
    assert_eq!(
        bridge_request(&launcher, r#"{"method":"files.preview","params":{"path":"/etc/passwd"}}"#)["error"]["code"],
        "permission_denied"
    );
    shell.hide_popover(Popover::Applications);
    preferences.window.set_visible(false);
    let previous_ids: Vec<_> =
        shell.windows.borrow().as_ref().unwrap().windows().iter().map(|window| window.id).collect();
    shell.windows.borrow_mut().take();
    wait_until("window service reconnection", || shell.windows.borrow().is_some());
    assert!(shell.windows.borrow().as_ref().unwrap().windows().iter().all(|window| !previous_ids.contains(&window.id)));
    let unlocked_windows = app.windows().len();
    for _ in 0..3 {
        assert!(shell.locker.lock(), "disposable compositor must support session locking");
        wait_until("confirmed compositor lock", || {
            shell.locker.is_locked() && !shell.state.borrow().lock_surfaces.is_empty()
        });
        assert_eq!(
            shell.state.borrow().lock_surfaces.len(),
            gdk::Display::default().unwrap().monitors().n_items() as usize
        );
        let locked = shell.state.borrow().lock_surfaces[0].1.clone();
        wait_until("lock renderer startup", || locked.is_ready());
        let invalid = serde_json::json!({"method":"locker.unlock","params":{"password":"x".repeat(4097)}}).to_string();
        assert_eq!(bridge_request(&locked, &invalid)["error"]["code"], "invalid_request");
        assert!(shell.locker.is_locked());
        locked.view.terminate_web_process();
        wait_until("native lock fallback", || locked.fallback_visible());
        assert!(shell.locker.is_locked());
        wait_until("lock renderer recovery", || locked.is_ready());
        assert!(shell.locker.is_locked());
        // Test-only teardown of the disposable compositor's lock. No PAM call.
        shell.locker.unlock();
        wait_until("lock teardown", || shell.state.borrow().lock_surfaces.is_empty());
        drop(locked);
        assert_eq!(app.windows().len(), unlocked_windows, "lock/unlock leaked windows");
    }
    shell.open_terminal();
    let terminal = shell.state.borrow().terminal.clone().unwrap();
    wait_until("terminal ready", || {
        terminal.is_ready() && eval(&terminal, "!!document.querySelector('.xterm-helper-textarea')") == "true"
    });
    eval(
        &terminal,
        "window.webkit.messageHandlers.meridian.postMessage(JSON.stringify({method:'terminal.write',params:{id:1,data:'printf \"SMOKE_LINE_%s\\\\n\" $(seq 1 300)\\n'}})); 'sent'",
    );
    wait_until("terminal scrollback output", || page_contains(&terminal, "SMOKE_LINE_300"));
    let initial = eval(&terminal, "document.querySelector('.xterm-rows').textContent");
    eval(
        &terminal,
        "document.querySelector('.xterm').dispatchEvent(new WheelEvent('wheel',{deltaY:-240,deltaMode:0,bubbles:true,cancelable:true})); 'scrolled'",
    );
    wait_until("terminal wheel scroll", || {
        eval(&terminal, "document.querySelector('.xterm-rows').textContent") != initial
    });
    wait_until("terminal jump control", || eval(&terminal, "!!document.querySelector('.terminal-follow')") == "true");
    eval(&terminal, "document.querySelector('.terminal-follow').click(); 'clicked'");
    wait_until("terminal latest output", || page_contains(&terminal, "SMOKE_LINE_300"));
    assert_eq!(eval(&terminal, "document.querySelector('.terminal-status') === null"), "true");
    // The rendered grid must fit the container rather than being clipped at the bottom.
    assert_eq!(
        eval(
            &terminal,
            "document.querySelector('.xterm-screen').getBoundingClientRect().height <= document.querySelector('.terminal-screen').clientHeight + 1"
        ),
        "true"
    );
    terminal.window.set_visible(false);
    wait_until("terminal cleanup", || shell.terminals.borrow().count() == 0);
    shell.open_preferences();
    shell.state.borrow().preferences.as_ref().unwrap().window.set_visible(false);
    shell.open_finder();
    shell.state.borrow().finder.as_ref().unwrap().window.set_visible(false);
    wait_until("cached app startup", || {
        let state = shell.state.borrow();
        state.finder.as_ref().unwrap().is_ready() && state.preferences.as_ref().unwrap().is_ready()
    });
    let finder = shell.state.borrow().finder.clone().unwrap();
    finder.window.present();
    let directory = crate::desktop::folder();
    std::fs::write(directory.join("preview-smoke.txt"), "<script>plain text only</script>").unwrap();
    finder.send(&Event::FinderLocation(proto::FilePath { path: directory.to_string_lossy().into_owned() }));
    wait_until("Orbit preview fixture", || {
        eval(&finder, "!!document.querySelector('[aria-label=\"preview-smoke.txt\"]')") == "true"
    });
    eval(&finder, "document.querySelector('[aria-label=\"preview-smoke.txt\"]').click(); 'selected'");
    wait_until("Orbit selected file", || {
        eval(&finder, "document.querySelector('[aria-label=\"preview-smoke.txt\"]').getAttribute('aria-pressed')")
            == "true"
    });
    eval(&finder, "document.querySelector('[aria-label=\"Preview file\"]').click(); 'preview'");
    wait_until("Orbit text preview", || {
        eval(&finder, "document.querySelector('.file-preview pre')?.textContent") == "<script>plain text only</script>"
    });
    assert_eq!(eval(&finder, "document.querySelector('.file-preview script') === null"), "true");
    eval(&finder, "document.querySelector('[aria-label=\"Close preview\"]').click(); 'closed'");
    eval(&finder, "document.querySelector('[aria-label=\"New folder tab\"]').click(); 'new tab'");
    wait_until("Orbit second tab", || eval(&finder, "document.querySelectorAll('[role=tab]').length") == "2");
    finder.window.set_visible(false);
    let topbar = shell.state.borrow().topbar.clone().unwrap();
    assert_eq!(
        bridge_request(&topbar, r#"{"method":"notifications.do_not_disturb","params":{"enabled":true}}"#)["result"]["do_not_disturb"],
        true
    );
    assert_eq!(
        bridge_request(&topbar, r#"{"method":"notifications.do_not_disturb","params":{"enabled":false}}"#)["result"]["do_not_disturb"],
        false
    );

    glib::MainContext::default().block_on(glib::timeout_future(Duration::from_secs(2)));
    let before = rss();
    let tree_before = tree_memory(std::process::id());
    let windows = app.windows().len();
    let cycles = std::env::var("MERIDIAN_SOAK_CYCLES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(50)
        .min(10_000);
    for _ in 0..cycles {
        shell.open_preferences();
        shell.state.borrow().preferences.as_ref().unwrap().window.set_visible(false);
        shell.open_finder();
        shell.state.borrow().finder.as_ref().unwrap().window.set_visible(false);
        shell.open_terminal();
        terminal.window.set_visible(false);
        let context = glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
    }
    assert_eq!(app.windows().len(), windows, "opening cached applications leaked windows");
    let soak = std::env::var("MERIDIAN_SOAK_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
        .min(86_400);
    let deadline = Instant::now() + Duration::from_secs(soak);
    while Instant::now() < deadline {
        shell.open_preferences();
        shell.state.borrow().preferences.as_ref().unwrap().window.set_visible(false);
        shell.open_terminal();
        terminal.window.set_visible(false);
        glib::MainContext::default().block_on(glib::timeout_future(Duration::from_secs(1)));
        assert_eq!(app.windows().len(), windows);
        assert_eq!(shell.terminals.borrow().count(), 0);
    }
    println!(
        "phase1_smoke soak_seconds={soak} process_tree_before={tree_before:?} process_tree_after={:?} metric=process_count_and_summed_rss_kib",
        tree_memory(std::process::id())
    );
    wait_until("closed terminal sessions", || shell.terminals.borrow().count() == 0);
    println!(
        "phase1_smoke cycles={cycles} shell_rss_before={before:?} shell_rss_after={:?} window_count={windows}",
        rss()
    );
    eprintln!("phase1_smoke event=destroy_windows");
    for window in app.windows() {
        window.destroy();
    }
    eprintln!("phase1_smoke event=drop_shell");
    drop(shell);
    eprintln!("phase1_smoke event=shell_dropped");
    app.quit();
}
