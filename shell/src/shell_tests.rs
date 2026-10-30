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

// A persistent virtual pointer exercises compositor hit testing, not DOM-generated hover.
struct TestPointer { child: std::process::Child, output: std::io::BufReader<std::process::ChildStdout> }
impl TestPointer {
    fn new(connector: &str, width: i32, height: i32) -> Self {
        use std::io::BufRead;
        let mut child = std::process::Command::new(std::env::var("MERIDIAN_TEST_POINTER").unwrap())
            .args([connector, &width.to_string(), &height.to_string()])
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new(); output.read_line(&mut line).unwrap(); assert_eq!(line.trim(), "ready");
        Self { child, output }
    }
    fn send(&mut self, command: &str) {
        use std::io::{BufRead, Write};
        writeln!(self.child.stdin.as_mut().unwrap(), "{command}").unwrap();
        let mut line = String::new(); self.output.read_line(&mut line).unwrap(); assert_eq!(line.trim(), "ok");
    }
}
impl Drop for TestPointer { fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); } }

fn validate_dock(shell: &Rc<Shell>, preferences: &Rc<WebSurface>) {
    let panel = shell.state.borrow().panel.clone().unwrap();
    let monitor = shell.state.borrow().primary.clone().unwrap();
    let connector = monitor.connector().unwrap().to_string();
    let geometry = monitor.geometry();
    let mut pointer = TestPointer::new(&connector, geometry.width(), geometry.height());
    let surface = panel.window.surface().unwrap();
    assert_eq!(surface.height(), crate::surface::PANEL_HEIGHT + crate::surface::PANEL_ANIMATION_OVERFLOW);
    let capture = std::env::var_os("MERIDIAN_DOCK_CAPTURES").map(std::path::PathBuf::from);
    if let Some(dir) = &capture { std::fs::create_dir_all(dir).unwrap(); }
    let wallpaper = shell.state.borrow().wallpapers.iter().find(|(m, _)| m == &monitor).unwrap().1.clone();
    eval(&wallpaper, "window.__dockPointerPassedThrough = false; window.addEventListener('pointermove', () => window.__dockPointerPassedThrough = true); 'ready'");
    pointer.send(&format!("move {} {}", geometry.width() / 2, geometry.height() - 150));
    wait_until("transparent panel input passes through", || eval(&wallpaper, "window.__dockPointerPassedThrough") == "true");
    for style in ["floating", "full-width", "minimal"] {
        let raw = serde_json::json!({"method":"desktop.dock_style_set", "params":{"id":style}}).to_string();
        assert_eq!(bridge_request(preferences, &raw)["ok"], true);
        wait_until("dock appearance", || eval(&panel, &format!("!!document.querySelector('.dock-{style}')")) == "true");
        for (edge, selector) in [("left", ".dock-items li:first-child button"), ("right", ".dock-items li:last-child button")] {
            let coords = eval(&panel, &format!("JSON.stringify((() => {{ const b=document.querySelector({selector:?}).getBoundingClientRect(); return [b.x+b.width/2,b.y+b.height/2,innerHeight]; }})())"));
            let coords: Vec<f64> = serde_json::from_str(&coords).unwrap();
            let x = coords[0].round() as i32;
            let y = geometry.height() - coords[2].round() as i32 + coords[1].round() as i32;
            pointer.send(&format!("move {x} {y}"));
            wait_until("real pointer dock hover", || eval(&panel, &format!("document.querySelector({selector:?}).parentElement.classList.contains('dock-hovered')")) == "true");
            wait_until("dock tooltip", || eval(&panel, "!!document.querySelector('.dock-preview')") == "true");
            glib::MainContext::default().block_on(glib::timeout_future(Duration::from_millis(250)));
            let fits = eval(&panel, &format!("(() => {{ const i=document.querySelector({selector:?}).querySelector('.dock-icon').getBoundingClientRect(), l=document.querySelector('.windows').getBoundingClientRect(), p=document.querySelector('.dock-preview').getBoundingClientRect(); return i.top>=l.top-1 && i.left>=l.left-1 && i.right<=l.right+1 && i.bottom<=l.bottom+1 && p.top>=0 && p.left>=0 && p.right<=innerWidth && p.bottom<=innerHeight; }})()"));
            assert_eq!(fits, "true", "{style} {edge}: animated icon or tooltip exceeds its clipping boundary");
            if let Some(dir) = &capture {
                let path = dir.join(format!("{style}-{edge}.png"));
                let status = std::process::Command::new("grim").args(["-o", &connector]).arg(path).status().unwrap();
                assert!(status.success(), "compositor screenshot failed");
            }
            if style == "floating" && edge == "right" {
                pointer.send("button 273 1");
                glib::MainContext::default().block_on(glib::timeout_future(Duration::from_millis(50)));
                pointer.send("button 273 0");
                wait_until("dock context menu", || {
                    let menu = shell.state.borrow().popovers.get(&Popover::DesktopMenu).cloned();
                    menu.is_some_and(|menu| menu.is_ready() && eval(&menu, "!!document.querySelector('.dock-item-menu:not(.hidden)')") == "true")
                });
                let menu = shell.state.borrow().popovers[&Popover::DesktopMenu].clone();
                assert_eq!(eval(&menu, "(() => { const r=document.querySelector('.dock-item-menu').getBoundingClientRect(); return r.left>=0 && r.right<=innerWidth && Math.abs(innerHeight-r.bottom-8)<2; })()"), "true");
                if let Some(dir) = &capture { assert!(std::process::Command::new("grim").args(["-o", &connector]).arg(dir.join("context-menu.png")).status().unwrap().success()); }
                shell.hide_popover(Popover::DesktopMenu);
                wait_until("context menu closed", || !menu.window.is_visible());
            }
            pointer.send("move 0 0");
            wait_until("dock hover exits", || eval(&panel, "!document.querySelector('.dock-hovered')") == "true");
            wait_until("dock tooltip dismisses", || eval(&panel, "!document.querySelector('.dock-preview')") == "true");
        }
    }
    assert_eq!(bridge_request(preferences, r#"{"method":"desktop.dock_style_set","params":{"id":"floating"}}"#)["ok"], true);
    wait_until("floating dock restored", || eval(&panel, "!!document.querySelector('.dock-floating')") == "true");
    // Exercise scroll overflow with many independent application groups. The
    // compositor pointer still drives the rendered first and last buttons.
    panel.send(&Event::WindowsChanged((0..35).map(|id| proto::WindowInfo {
        id: 1000 + id, name: format!("Overflow {id}"), title: format!("Window {id}"), active: false,
    }).collect()));
    wait_until("dock overflow", || eval(&panel, "document.querySelectorAll('.dock-items li').length >= 35 && document.querySelector('.windows').scrollWidth > document.querySelector('.windows').clientWidth") == "true");
    for (edge, selector, scroll) in [("left", ".dock-items li:first-child button", "0"), ("right", ".dock-items li:last-child button", "el.scrollWidth")] {
        eval(&panel, &format!("(() => {{ const el=document.querySelector('.windows'); el.scrollLeft={scroll}; }})()"));
        let coords = eval(&panel, &format!("JSON.stringify((() => {{ const b=document.querySelector({selector:?}).getBoundingClientRect(); return [b.x+b.width/2,b.y+b.height/2,innerHeight]; }})())"));
        let coords: Vec<f64> = serde_json::from_str(&coords).unwrap();
        pointer.send(&format!("move {} {}", coords[0].round() as i32, geometry.height() - coords[2].round() as i32 + coords[1].round() as i32));
        wait_until("overflow edge hover", || eval(&panel, &format!("document.querySelector({selector:?}).parentElement.classList.contains('dock-hovered')")) == "true");
        wait_until("overflow tooltip", || eval(&panel, "!!document.querySelector('.dock-preview')") == "true");
        glib::MainContext::default().block_on(glib::timeout_future(Duration::from_millis(250)));
        assert_eq!(eval(&panel, &format!("(() => {{ const i=document.querySelector({selector:?}).querySelector('.dock-icon').getBoundingClientRect(), l=document.querySelector('.windows').getBoundingClientRect(), p=document.querySelector('.dock-preview').getBoundingClientRect(); return i.top>=l.top-1 && i.left>=l.left-1 && i.right<=l.right+1 && p.left>=0 && p.right<=innerWidth; }})()")), "true", "overflow {edge} icon or tooltip clipped");
        if let Some(dir) = &capture {
            assert!(std::process::Command::new("grim").args(["-o", &connector]).arg(dir.join(format!("overflow-{edge}.png"))).status().unwrap().success());
        }
        pointer.send("move 0 0");
        wait_until("overflow hover exits", || eval(&panel, "!document.querySelector('.dock-hovered') && !document.querySelector('.dock-preview')") == "true");
    }
    panel.send(&Event::WindowsChanged(shell.window_infos(&shell.windows.borrow().as_ref().unwrap().windows())));
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
    for popover in if std::env::var_os("MERIDIAN_DOCK_ONLY").is_some() { &[][..] } else { &[Popover::Applications, Popover::Options][..] } {
        let popover = *popover;
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
    validate_dock(&shell, &preferences);
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
    // The production terminal is a separate Kitty Wayland client, not a WebSurface.
    let before_terminal: Vec<_> = shell.windows.borrow().as_ref().unwrap().windows().iter().map(|w| w.id).collect();
    shell.open_terminal();
    wait_until("external Kitty window", || shell.windows.borrow().as_ref().unwrap().windows().iter().any(|w| {
        !before_terminal.contains(&w.id) && w.app_id.to_lowercase().contains("kitty")
    }));
    let kitty = shell.windows.borrow().as_ref().unwrap().windows().into_iter()
        .find(|w| !before_terminal.contains(&w.id) && w.app_id.to_lowercase().contains("kitty")).unwrap();
    assert!(shell.state.borrow().terminal.is_none());
    shell.open_terminal();
    wait_until("second external Kitty window", || shell.windows.borrow().as_ref().unwrap().windows().iter()
        .filter(|w| !before_terminal.contains(&w.id) && w.app_id.to_lowercase().contains("kitty")).count() >= 2);
    let kitty_windows: Vec<_> = shell.windows.borrow().as_ref().unwrap().windows().into_iter()
        .filter(|w| !before_terminal.contains(&w.id) && w.app_id.to_lowercase().contains("kitty")).collect();
    let names = shell.window_infos(&kitty_windows);
    let panel = shell.state.borrow().panel.clone().unwrap();
    let selector = format!("button[aria-label={}]", serde_json::to_string(&names[0].name).unwrap());
    wait_until("grouped Kitty dock item", || eval(&panel, &format!("document.querySelector({selector:?})?.querySelector('.dock-window-count')?.textContent === '2'")) == "true");
    let monitor = shell.state.borrow().primary.clone().unwrap();
    let geometry = monitor.geometry();
    let connector = monitor.connector().unwrap().to_string();
    let mut pointer = TestPointer::new(&connector, geometry.width(), geometry.height());
    let coords = eval(&panel, &format!("JSON.stringify((() => {{ const b=document.querySelector({selector:?}).getBoundingClientRect(); return [b.x+b.width/2,b.y+b.height/2,innerHeight]; }})())"));
    let coords: Vec<f64> = serde_json::from_str(&coords).unwrap();
    pointer.send(&format!("move {} {}", coords[0].round() as i32, geometry.height() - coords[2].round() as i32 + coords[1].round() as i32));
    wait_until("multiple window preview", || eval(&panel, "document.querySelectorAll('.dock-preview-window').length === 2") == "true");
    assert_eq!(eval(&panel, "(() => { const r=document.querySelector('.dock-preview').getBoundingClientRect(); return r.left>=0 && r.right<=innerWidth && r.top>=0; })()"), "true");
    if let Some(dir) = std::env::var_os("MERIDIAN_DOCK_CAPTURES") {
        assert!(std::process::Command::new("grim").args(["-o", &connector]).arg(std::path::Path::new(&dir).join("multiple-windows.png")).status().unwrap().success());
    }
    let preview_coords = eval(&panel, "JSON.stringify((() => { const b=document.querySelectorAll('.dock-preview-window')[1].getBoundingClientRect(); return [b.x+b.width/2,b.y+b.height/2,innerHeight]; })())");
    let preview_coords: Vec<f64> = serde_json::from_str(&preview_coords).unwrap();
    pointer.send(&format!("move {} {}", preview_coords[0].round() as i32, geometry.height() - preview_coords[2].round() as i32 + preview_coords[1].round() as i32));
    wait_until("pointer enters window preview", || eval(&panel, "document.querySelectorAll('.dock-preview-window')[1]?.matches(':hover') === true") == "true");
    pointer.send("button 272 1"); pointer.send("button 272 0");
    wait_until("preview activates selected Kitty window", || shell.windows.borrow().as_ref().unwrap().windows().iter().any(|w| w.id == names[1].id && w.active));
    pointer.send("move 0 0");
    drop(pointer);
    assert!(shell.windows.borrow().as_ref().unwrap().close(kitty.id));
    for window in &kitty_windows { if window.id != kitty.id { assert!(shell.windows.borrow().as_ref().unwrap().close(window.id)); } }
    wait_until("external Kitty closed", || !shell.windows.borrow().as_ref().unwrap().windows().iter().any(|w| kitty_windows.iter().any(|kitty| kitty.id == w.id)));
    shell.open_preferences();
    shell.state.borrow().preferences.as_ref().unwrap().window.set_visible(false);
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
