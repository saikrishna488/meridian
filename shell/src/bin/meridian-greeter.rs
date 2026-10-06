//! meridian-greeter: Meridian's login screen, run by greetd.
//!
//! greetd starts it (inside a minimal labwc, see session/greeter/) as the
//! unprivileged greeter user, with `GREETD_SOCK` pointing at greetd's IPC
//! socket. It exits after a successful login; greetd then starts the session.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use meridian_shell::{greeter, install_css, paths, preflight};

fn main() -> glib::ExitCode {
    meridian_shell::init_logging();
    let dev = std::env::var_os("MERIDIAN_DEV").is_some_and(|v| v == "1");

    // Not unique: no D-Bus name needed on the login screen.
    let app = gtk::Application::builder()
        .application_id("org.meridian.Greeter")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let greeter: Rc<RefCell<Option<Rc<greeter::Greeter>>>> = Rc::default();
    let failed = Rc::new(Cell::new(false));

    {
        let failed = failed.clone();
        app.connect_activate(move |app| {
            if greeter.borrow().is_some() {
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
            *greeter.borrow_mut() = Some(greeter::Greeter::start(app, roots, dev));
        });
    }

    let code = app.run_with_args::<&str>(&[]);
    if failed.get() { glib::ExitCode::FAILURE } else { code }
}
