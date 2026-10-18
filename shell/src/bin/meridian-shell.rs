//! meridian-shell: the Meridian desktop shell (taskbar, menus, wallpaper).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use meridian_shell::{install_css, paths, preflight, shell};

const APP_ID: &str = "org.meridian.Shell";

fn main() -> glib::ExitCode {
    meridian_shell::init_logging();
    if std::env::args_os().nth(1).is_some_and(|arg| arg == "--idle-timeouts") {
        let settings = meridian_shell::power_settings::idle();
        println!("{} {}", settings.lock_after, settings.blank_after);
        return glib::ExitCode::SUCCESS;
    }
    if std::env::args_os().nth(1).is_some_and(|arg| arg == "--autostart") {
        meridian_shell::startup::launch();
        return glib::ExitCode::SUCCESS;
    }
    let _instance = match meridian_shell::instance::InstanceGuard::acquire("shell") {
        Ok(guard) => guard,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            log::info!("component=shell event=duplicate_instance_rejected");
            return glib::ExitCode::SUCCESS;
        }
        Err(error) => {
            log::error!("component=shell event=instance_lock_failed error={error}");
            return glib::ExitCode::FAILURE;
        }
    };
    let dev = std::env::var_os("MERIDIAN_DEV").is_some_and(|v| v == "1");

    let app = gtk::Application::builder().application_id(APP_ID).build();
    let shell: Rc<RefCell<Option<Rc<shell::Shell>>>> = Rc::default();
    let failed = Rc::new(Cell::new(false));

    let shutdown_shell = shell.clone();
    app.connect_shutdown(move |_| {
        if let Some(shell) = shutdown_shell.borrow_mut().take() {
            shell.shutdown();
        }
    });
    {
        let failed = failed.clone();
        app.connect_activate(move |app| {
            // Activation repeats when e.g. `gapplication launch` is used.
            if shell.borrow().is_some() {
                return;
            }
            let roots = match preflight().and_then(|_| paths::roots()) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("{e}");
                    failed.set(true);
                    app.quit();
                    return;
                }
            };
            install_css();
            *shell.borrow_mut() = Some(shell::Shell::start(app, roots, dev));
        });
    }

    // GTK options are not meaningful for the shell; don't parse argv.
    let code = app.run_with_args::<&str>(&[]);
    if failed.get() { glib::ExitCode::FAILURE } else { code }
}
