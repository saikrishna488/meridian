//! Native GTK installer. No WebView, Node runtime, password collection, or tests.
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    sync::mpsc::{self, Receiver, SyncSender},
    time::Duration,
};

const MODES: [&str; 6] = ["--install", "--update", "--repair", "--check", "--uninstall", "--compile"];
enum Event {
    Log(String),
    Finished(bool, String),
}
struct Ui {
    source: RefCell<Option<PathBuf>>,
    running: Cell<bool>,
    events: RefCell<Option<Receiver<Event>>>,
    folder: gtk::Button,
    source_label: gtk::Label,
    mode: gtk::DropDown,
    prepared: gtk::CheckButton,
    confirm_remove: gtk::CheckButton,
    run: gtk::Button,
    status: gtk::Label,
    progress: gtk::ProgressBar,
    logs: gtk::TextBuffer,
}
fn valid_source(path: &Path) -> bool {
    ["Cargo.toml", "install.sh", "tools/install.sh", "tools/release-manifest.sh"]
        .iter()
        .all(|file| path.join(file).is_file())
}
fn source_record() -> PathBuf {
    glib::user_config_dir().join("meridian/installer-source.json")
}
fn initial_source() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--source" {
            return args.next().map(PathBuf::from).filter(|path| valid_source(path));
        }
    }
    let saved = std::fs::File::open(source_record()).ok().and_then(|file| {
        let mut bytes = Vec::new();
        file.take(16385).read_to_end(&mut bytes).ok()?;
        (bytes.len() <= 16384).then(|| serde_json::from_slice::<PathBuf>(&bytes).ok()).flatten()
    });
    [
        std::env::var_os("MERIDIAN_SOURCE_DIR").map(PathBuf::from),
        saved,
        std::env::current_dir().ok(),
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().map(Path::to_path_buf),
    ]
    .into_iter()
    .flatten()
    .find(|path| valid_source(path))
}
fn remember_source(path: &Path) {
    let file = source_record();
    let Some(parent) = file.parent() else { return };
    let Ok(bytes) = serde_json::to_vec(path) else { return };
    if std::fs::create_dir_all(parent).is_ok() {
        let _ = gio::File::for_path(file).replace_contents(
            &bytes,
            None,
            false,
            gio::FileCreateFlags::PRIVATE | gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::Cancellable::NONE,
        );
    }
}
impl Ui {
    fn controls(&self) {
        let busy = self.running.get();
        let mode = self.mode.selected() as usize;
        self.folder.set_sensitive(!busy);
        self.mode.set_sensitive(!busy);
        self.prepared.set_visible(mode < 3);
        self.prepared.set_sensitive(!busy);
        self.confirm_remove.set_visible(mode == 4);
        self.confirm_remove.set_sensitive(!busy);
        self.run.set_label(
            [
                "Install Meridian",
                "Update Meridian",
                "Repair Meridian",
                "Check requirements",
                "Remove Meridian",
                "Compile release",
            ]
            .get(mode)
            .copied()
            .unwrap_or("Choose an action"),
        );
        self.run
            .set_sensitive(!busy && self.source.borrow().is_some() && (mode != 4 || self.confirm_remove.is_active()));
    }
    fn choose_source(&self, path: PathBuf) {
        if !valid_source(&path) {
            self.status.set_text("Choose a complete Meridian source checkout containing install.sh.");
            return;
        }
        let path = path.canonicalize().unwrap_or(path);
        self.source_label.set_text(&path.to_string_lossy());
        self.prepared.set_active(path.join("target/release/meridian-build.sha256").is_file());
        remember_source(&path);
        *self.source.borrow_mut() = Some(path);
        self.status.set_text("Ready. Prepared builds skip compilation; source builds can take several minutes.");
        self.controls();
    }
    fn append_log(&self, text: &str) {
        self.logs.insert(&mut self.logs.end_iter(), text);
        let excess = self.logs.char_count() - 100_000;
        if excess > 0 {
            self.logs.delete(&mut self.logs.start_iter(), &mut self.logs.iter_at_offset(excess));
        }
        if let Some(step) = text.lines().rev().find(|line| line.trim_start().starts_with("==>")) {
            self.status.set_text(step.trim().trim_start_matches("==>").trim());
        }
    }
    fn start(&self) {
        if self.running.get() {
            return;
        }
        let mode = self.mode.selected() as usize;
        let Some(argument) = MODES.get(mode) else { return };
        if mode == 4 && !self.confirm_remove.is_active() {
            return;
        }
        let Some(source) = self.source.borrow().clone() else { return };
        if !valid_source(&source) {
            self.status.set_text("The source checkout is no longer available. Choose its folder again.");
            return;
        }
        let prepared = mode < 3 && self.prepared.is_active();
        let (sender, receiver) = mpsc::sync_channel(128);
        *self.events.borrow_mut() = Some(receiver);
        self.running.set(true);
        self.logs.set_text("");
        self.status.set_text(if mode == 3 {
            "Checking requirements…"
        } else {
            "Starting… Authentication opens through your desktop when required."
        });
        self.progress.set_visible(true);
        self.controls();
        let argument = *argument;
        std::thread::spawn(move || execute(source, argument, prepared, sender));
    }
    fn poll(&self) {
        let mut finished = None;
        if let Some(receiver) = self.events.borrow().as_ref() {
            // Bound both the queue and each main-loop batch to keep memory/UI latency stable.
            for _ in 0..32 {
                match receiver.try_recv() {
                    Ok(Event::Log(text)) => self.append_log(&text),
                    Ok(Event::Finished(ok, message)) => {
                        finished = Some((ok, message));
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = Some((
                            false,
                            "Installer worker stopped unexpectedly. Review the log before retrying.".into(),
                        ));
                        break;
                    }
                }
            }
        }
        if let Some((ok, message)) = finished {
            self.running.set(false);
            if ok && self.mode.selected() == 5 {
                self.prepared.set_active(
                    self.source
                        .borrow()
                        .as_ref()
                        .is_some_and(|source| source.join("target/release/meridian-build.sha256").is_file()),
                );
            }
            self.events.borrow_mut().take();
            self.progress.set_visible(false);
            self.status.set_text(&message);
            self.confirm_remove.set_active(false);
            self.controls();
            if !ok {
                self.append_log("\nOperation did not complete successfully.\n");
            }
        } else if self.running.get() {
            self.progress.pulse();
        }
    }
}
fn stream(mut pipe: impl Read, sender: SyncSender<Event>) {
    let mut bytes = [0u8; 4096];
    let mut escape = false;
    loop {
        match pipe.read(&mut bytes) {
            Ok(0) => break,
            Ok(count) => {
                let mut clean = String::with_capacity(count);
                for ch in String::from_utf8_lossy(&bytes[..count]).chars() {
                    if ch == '\u{1b}' {
                        escape = true;
                        continue;
                    }
                    if escape {
                        if ch.is_ascii_alphabetic() {
                            escape = false;
                        }
                        continue;
                    }
                    if ch == '\r' {
                        clean.push('\n');
                    } else if !ch.is_control() || ch == '\n' || ch == '\t' {
                        clean.push(ch);
                    }
                }
                if sender.send(Event::Log(clean)).is_err() {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                let _ = sender.send(Event::Log(format!("\nCannot read installer output: {error}\n")));
                break;
            }
        }
    }
}
fn execute(source: PathBuf, mode: &'static str, prepared: bool, sender: SyncSender<Event>) {
    let mut command = Command::new("bash");
    command
        .arg(source.join("install.sh"))
        .arg("--cli")
        .arg(mode)
        .arg("--yes")
        .current_dir(&source)
        .env("MERIDIAN_GUI_AUTH", "1")
        .env("CARGO_BUILD_JOBS", "1")
        .env("NODE_OPTIONS", "--max-old-space-size=1024")
        .env("CARGO_TERM_COLOR", "never")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if prepared {
        command.arg("--prepared");
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = sender.send(Event::Finished(false, format!("Could not start installer: {error}")));
            return;
        }
    };
    let stdout = child.stdout.take().map(|pipe| {
        let sender = sender.clone();
        std::thread::spawn(move || stream(pipe, sender))
    });
    let stderr = child.stderr.take().map(|pipe| {
        let sender = sender.clone();
        std::thread::spawn(move || stream(pipe, sender))
    });
    let status = child.wait();
    if let Some(worker) = stdout {
        let _ = worker.join();
    }
    if let Some(worker) = stderr {
        let _ = worker.join();
    }
    let (ok, message) = match status {
        Ok(status) if status.success() => (
            true,
            match mode {
                "--check" => "Requirements check passed. No installation was performed.",
                "--compile" => "Release compiled. Choose Install and ‘Use prepared release’ to install it.",
                "--uninstall" => "Meridian removed. Personal settings and dependency packages were preserved.",
                _ => "Complete. Sign out and select Meridian at the login screen to load this version.",
            }
            .into(),
        ),
        Ok(status) => (
            false,
            if prepared {
                format!(
                    "Operation stopped ({status}). Review the log. If the prepared build is stale, disable ‘Use prepared release’ to rebuild."
                )
            } else {
                format!("Operation stopped ({status}). Review the log before retrying.")
            },
        ),
        Err(error) => (false, format!("Could not wait for the installer: {error}")),
    };
    let _ = sender.send(Event::Finished(ok, message));
}
fn build_window(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Meridian Installer")
        .default_width(700)
        .default_height(650)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(28);
    content.set_margin_bottom(24);
    content.set_margin_start(28);
    content.set_margin_end(28);
    let title = gtk::Label::new(Some("Install. Update. Back to work."));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    content.append(&title);
    let intro = gtk::Label::new(Some(
        "Manage Meridian from a trusted source checkout. Prepared releases skip compilation. Source builds use one build job and do not run tests. Updates use your current checkout; Git changes are never fetched or reset.",
    ));
    intro.set_wrap(true);
    intro.set_xalign(0.0);
    content.append(&intro);
    let source_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let source_label = gtk::Label::new(Some("Choose your Meridian source checkout"));
    source_label.set_hexpand(true);
    source_label.set_xalign(0.0);
    source_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let folder = gtk::Button::with_label("Choose folder…");
    source_row.append(&source_label);
    source_row.append(&folder);
    content.append(&source_row);
    let mode =
        gtk::DropDown::from_strings(&["Install", "Update", "Repair", "Check requirements", "Remove", "Compile only"]);
    if Path::new("/usr/local/bin/meridian-shell").is_file() {
        mode.set_selected(1);
    }
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let Some(action) = arguments.windows(2).find(|pair| pair[0] == "--action") {
        if let Some(index) = MODES.iter().position(|mode| action[1] == mode.trim_start_matches("--")) {
            mode.set_selected(index as u32);
        }
    }
    content.append(&mode);
    let prepared = gtk::CheckButton::with_label("Use prepared release · skip build and development dependencies");
    content.append(&prepared);
    let details = gtk::Label::new(Some(
        "Source builds and installation may install dependency packages and request administrator authentication. Arch may perform a system upgrade. Your login manager stays unchanged. Keep this window open until completion.",
    ));
    details.set_wrap(true);
    details.set_xalign(0.0);
    details.add_css_class("dim-label");
    content.append(&details);
    let confirm_remove =
        gtk::CheckButton::with_label("Confirm removal of Meridian's installed files (personal settings stay)");
    content.append(&confirm_remove);
    let status = gtk::Label::new(Some("Choose a source checkout to begin."));
    status.set_wrap(true);
    status.set_xalign(0.0);
    status.set_selectable(true);
    content.append(&status);
    let progress = gtk::ProgressBar::new();
    progress.set_visible(false);
    content.append(&progress);
    let logs = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    // Trimming visible text is insufficient if the undo history retains it.
    logs.set_enable_undo(false);
    let log_view = gtk::TextView::builder()
        .buffer(&logs)
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    let scroll = gtk::ScrolledWindow::builder().child(&log_view).vexpand(true).min_content_height(140).build();
    content.append(&scroll);
    let run = gtk::Button::with_label("Install Meridian");
    run.add_css_class("suggested-action");
    content.append(&run);
    window.set_child(Some(&content));
    let ui = Rc::new(Ui {
        source: RefCell::new(None),
        running: Cell::new(false),
        events: RefCell::new(None),
        folder,
        source_label,
        mode,
        prepared,
        confirm_remove,
        run,
        status,
        progress,
        logs,
    });
    if let Some(path) = initial_source() {
        ui.choose_source(path);
    } else {
        ui.controls();
    }
    if arguments.iter().any(|argument| argument == "--prepared") {
        ui.prepared.set_active(true);
    }
    let weak = Rc::downgrade(&ui);
    ui.run.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.start();
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.mode.connect_selected_notify(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.confirm_remove.set_active(false);
            ui.controls();
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.confirm_remove.connect_toggled(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.controls();
        }
    });
    let weak = Rc::downgrade(&ui);
    let parent = window.downgrade();
    ui.folder.connect_clicked(move |_| {
        let Some(parent) = parent.upgrade() else { return };
        let weak = weak.clone();
        gtk::FileDialog::builder().title("Choose Meridian source checkout").build().select_folder(
            Some(&parent),
            gio::Cancellable::NONE,
            move |result| {
                if let (Some(ui), Ok(file)) = (weak.upgrade(), result) {
                    if let Some(path) = file.path() {
                        ui.choose_source(path);
                    }
                }
            },
        );
    });
    let weak = Rc::downgrade(&ui);
    let timer = Rc::new(RefCell::new(Some(glib::timeout_add_local(Duration::from_millis(100), move || {
        ui.poll();
        glib::ControlFlow::Continue
    }))));
    window.connect_close_request(move |_| {
        if let Some(ui) = weak.upgrade() {
            if ui.running.get() {
                ui.status.set_text("An operation is running. Wait for completion before closing.");
                return glib::Propagation::Stop;
            }
        }
        if let Some(timer) = timer.borrow_mut().take() {
            timer.remove();
        }
        glib::Propagation::Proceed
    });
    window.present();
}
fn main() -> glib::ExitCode {
    // All builds remain unprivileged. Polkit handles authentication for package/system stages.
    if unsafe { libc::geteuid() } == 0 {
        eprintln!("Run Meridian Installer as your regular user, without sudo.");
        return glib::ExitCode::FAILURE;
    }
    let app = gtk::Application::builder().application_id("org.meridian.Installer").build();
    app.connect_activate(|app| {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_decoration_layout(Some("close,minimize,maximize:"));
        }
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            build_window(app);
        }
    });
    app.run_with_args::<&str>(&[])
}
