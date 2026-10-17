//! Standalone browser. Web pages have no access to Meridian shell IPC.
use gtk::prelude::*;
use webkit6::prelude::*;
fn destination(input: &str) -> String {
    let t = input.trim();
    if t.starts_with("https://") || t.starts_with("http://") {
        return t.into();
    }
    if !t.contains(char::is_whitespace) && !t.contains(':') && (t.contains('.') || t == "localhost") {
        return format!("https://{t}");
    }
    let q: String = t
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("https://duckduckgo.com/?q={q}")
}
fn icon(name: &str, tip: &str) -> gtk::Button {
    let b = gtk::Button::from_icon_name(name);
    b.set_tooltip_text(Some(tip));
    b
}
fn history_path() -> std::path::PathBuf {
    glib::user_data_dir().join("meridian/drift/history.json")
}
fn remember(entries: &mut Vec<(String, String)>, title: String, uri: String) {
    entries.retain(|(_, existing)| existing != &uri);
    entries.insert(0, (title, uri));
    entries.truncate(200);
}
fn tab(
    book: &gtk::Notebook,
    session: &webkit6::NetworkSession,
    uri: &str,
    related: Option<&webkit6::WebView>,
) -> webkit6::WebView {
    let builder = webkit6::WebView::builder().hexpand(true).vexpand(true);
    let view = if let Some(parent) = related {
        builder.related_view(parent).build()
    } else {
        builder.network_session(session).build()
    };
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.set_margin_start(10);
    bar.set_margin_end(10);
    bar.set_margin_top(8);
    bar.set_margin_bottom(8);
    let back = icon("go-previous-symbolic", "Back");
    let forward = icon("go-next-symbolic", "Forward");
    let reload = icon("view-refresh-symbolic", "Reload");
    let address = gtk::Entry::builder().hexpand(true).placeholder_text("Search or enter address").build();
    let star = icon("star-new-symbolic", "Bookmark page");
    let search = icon("edit-find-symbolic", "Find in page (Ctrl+F)");
    let previous = icon("go-up-symbolic", "Previous match");
    let next = icon("go-down-symbolic", "Next match");
    let dismiss = icon("window-close-symbolic", "Close find bar");
    let find_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    find_bar.set_margin_start(12);
    find_bar.set_margin_end(12);
    find_bar.set_margin_bottom(8);
    find_bar.set_visible(false);
    let find = gtk::SearchEntry::builder().placeholder_text("Find in page").hexpand(true).build();
    for b in [&back, &forward, &reload] {
        bar.append(b);
    }
    bar.append(&address);
    bar.append(&star);
    bar.append(&search);
    let progress = gtk::ProgressBar::new();
    page.append(&bar);
    find_bar.append(&find);
    find_bar.append(&previous);
    find_bar.append(&next);
    find_bar.append(&dismiss);
    page.append(&find_bar);
    let (f, v) = (find_bar.clone(), view.clone());
    dismiss.connect_clicked(move |_| {
        f.set_visible(false);
        if let Some(controller) = v.find_controller() {
            controller.search_finish();
        }
    });
    for (button, backwards) in [(previous, true), (next, false)] {
        let v = view.clone();
        button.connect_clicked(move |_| {
            if let Some(controller) = v.find_controller() {
                if backwards {
                    controller.search_previous();
                } else {
                    controller.search_next();
                }
            }
        });
    }
    page.append(&progress);
    page.append(&view);
    let label = gtk::Label::new(Some("New tab"));
    label.set_max_width_chars(20);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let title = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    title.append(&label);
    let close = icon("window-close-symbolic", "Close tab");
    close.add_css_class("flat");
    title.append(&close);
    let n = book.append_page(&page, Some(&title));
    book.set_tab_reorderable(&page, true);
    book.set_current_page(Some(n));
    let b = book.downgrade();
    let p = page.downgrade();
    close.connect_clicked(move |_| {
        if let (Some(b), Some(p)) = (b.upgrade(), p.upgrade())
            && let Some(n) = b.page_num(&p)
        {
            b.remove_page(Some(n));
        }
    });
    let v = view.clone();
    address.connect_activate(move |e| v.load_uri(&destination(&e.text())));
    back.set_sensitive(false);
    forward.set_sensitive(false);
    let v = view.clone();
    back.connect_clicked(move |_| v.go_back());
    let v = view.clone();
    forward.connect_clicked(move |_| v.go_forward());
    let v = view.clone();
    reload.connect_clicked(move |_| {
        if v.is_loading() {
            v.stop_loading();
        } else {
            v.reload();
        }
    });
    let keys = gtk::EventControllerKey::new();
    let (entry, f, v, fb) = (address.clone(), find.clone(), view.clone(), find_bar.clone());
    keys.connect_key_pressed(move |_, key, _, mods| {
        if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            if key == gtk::gdk::Key::l {
                entry.grab_focus();
                entry.select_region(0, -1);
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::f {
                fb.set_visible(true);
                f.grab_focus();
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::r {
                v.reload();
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    page.add_controller(keys);
    let error = gtk::Label::new(None);
    error.set_wrap(true);
    error.set_visible(false);
    page.append(&error);
    let e = error.clone();
    view.connect_load_failed(move |_, _, _, reason| {
        e.set_text(&format!("Couldn’t load this page. {reason}"));
        e.set_visible(true);
        true
    });
    view.connect_load_changed(move |_, event| {
        if event == webkit6::LoadEvent::Started {
            error.set_visible(false);
        }
    });
    view.connect_is_loading_notify(move |v| {
        reload.set_icon_name(if v.is_loading() { "process-stop-symbolic" } else { "view-refresh-symbolic" });
        reload.set_tooltip_text(Some(if v.is_loading() { "Stop loading" } else { "Reload" }));
    });
    view.connect_uri_notify(move |v| {
        address.set_text(v.uri().as_deref().unwrap_or(""));
        back.set_sensitive(v.can_go_back());
        forward.set_sensitive(v.can_go_forward());
    });
    view.connect_title_notify(move |v| label.set_text(v.title().as_deref().unwrap_or("New tab")));
    view.connect_estimated_load_progress_notify(move |v| {
        progress.set_fraction(v.estimated_load_progress());
        progress.set_visible(v.estimated_load_progress() < 1.0);
    });
    let (f, fb) = (find.clone(), find_bar.clone());
    search.connect_clicked(move |_| {
        fb.set_visible(!fb.is_visible());
        if fb.is_visible() {
            f.grab_focus();
        }
    });
    let v = view.clone();
    find.connect_next_match(move |_| {
        if let Some(f) = v.find_controller() {
            f.search_next();
        }
    });
    let v = view.clone();
    find.connect_search_changed(move |e| {
        if e.text().is_empty() {
            if let Some(f) = v.find_controller() {
                f.search_finish();
            }
        } else if let Some(f) = v.find_controller() {
            f.search(
                &e.text(),
                (webkit6::FindOptions::CASE_INSENSITIVE | webkit6::FindOptions::WRAP_AROUND).bits(),
                1000,
            );
        }
    });
    let v = view.clone();
    let fb = find_bar.clone();
    find.connect_stop_search(move |_| {
        fb.set_visible(false);
        if let Some(f) = v.find_controller() {
            f.search_finish();
        }
    });
    let v = view.clone();
    star.connect_clicked(move |button| {
        if let Some(uri) = v.uri().filter(|u| u.starts_with("https://") || u.starts_with("http://")) {
            let path = glib::user_data_dir().join("meridian/drift/bookmarks.json");
            let mut entries: Vec<(String, String)> =
                std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
            if !entries.iter().any(|(_, u)| u == uri.as_str()) {
                entries.push((v.title().unwrap_or_else(|| uri.clone()).to_string(), uri.to_string()));
            }
            if let Some(p) = path.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            if serde_json::to_vec(&entries).ok().is_some_and(|b| std::fs::write(path, b).is_ok()) {
                button.set_tooltip_text(Some("Bookmark saved"));
            }
        }
    });
    let b = book.downgrade();
    let s = session.clone();
    view.connect_create(move |parent, _| b.upgrade().map(|b| tab(&b, &s, "about:blank", Some(parent)).upcast()));
    view.connect_decide_policy(|_, decision, kind| {
        if kind == webkit6::PolicyDecisionType::Response
            && let Some(response) = decision.downcast_ref::<webkit6::ResponsePolicyDecision>()
            && !response.is_mime_type_supported()
        {
            decision.download();
            return true;
        }
        false
    });
    view.connect_permission_request(|_, r| {
        r.deny();
        true
    });
    if !session.is_ephemeral() {
        view.connect_load_changed(|view, event| {
            if event != webkit6::LoadEvent::Finished {
                return;
            }
            let Some(uri) = view.uri().filter(|uri| uri.starts_with("https://") || uri.starts_with("http://")) else {
                return;
            };
            let path = history_path();
            let mut entries =
                std::fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
            remember(&mut entries, view.title().unwrap_or_else(|| uri.clone()).to_string(), uri.to_string());
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(bytes) = serde_json::to_vec(&entries) {
                let _ = std::fs::write(path, bytes);
            }
        });
    }
    view.load_uri(uri);
    view
}
fn window(app: &gtk::Application, private: bool) {
    meridian_shell::install_app_css();
    let data = glib::user_data_dir().join("meridian/drift");
    let cache = glib::user_cache_dir().join("meridian/drift");
    let session = if private {
        webkit6::NetworkSession::new_ephemeral()
    } else {
        webkit6::NetworkSession::new(data.to_str(), cache.to_str())
    };
    let win = gtk::ApplicationWindow::builder()
        .application(app)
        .title(if private { "Drift — Private" } else { "Drift" })
        .default_width(1100)
        .default_height(760)
        .build();
    win.add_css_class("meridian-app");
    let header = gtk::HeaderBar::new();
    let new = icon("list-add-symbolic", "New tab");
    header.pack_start(&new);
    let private_button = icon("view-conceal-symbolic", "New private window");
    header.pack_end(&private_button);
    let marks = gtk::MenuButton::builder().icon_name("user-bookmarks-symbolic").tooltip_text("Bookmarks").build();
    header.pack_end(&marks);
    let history =
        gtk::MenuButton::builder().icon_name("document-open-recent-symbolic").tooltip_text("Recent history").build();
    history.set_visible(!private);
    header.pack_end(&history);
    let reset = gtk::Button::with_label("100%");
    reset.set_tooltip_text(Some("Reset zoom"));
    header.pack_end(&reset);
    let minus = gtk::Button::with_label("−");
    minus.set_tooltip_text(Some("Zoom out"));
    let plus = gtk::Button::with_label("+");
    plus.set_tooltip_text(Some("Zoom in"));
    header.pack_end(&plus);
    header.pack_end(&minus);
    win.set_titlebar(Some(&header));
    let book = gtk::Notebook::new();
    book.set_scrollable(true);
    win.set_child(Some(&book));
    let weak_window = win.downgrade();
    book.connect_page_removed(move |book, _, _| {
        if book.n_pages() == 0
            && let Some(window) = weak_window.upgrade()
        {
            window.close();
        }
    });
    tab(&book, &session, "https://duckduckgo.com/", None);
    let (b, s) = (book.clone(), session.clone());
    new.connect_clicked(move |_| {
        tab(&b, &s, "https://duckduckgo.com/", None);
    });
    let a = app.clone();
    private_button.connect_clicked(move |_| window(&a, true));
    for (button, delta) in [(minus, -0.1), (plus, 0.1), (reset, 0.0)] {
        let b = book.clone();
        button.connect_clicked(move |_| {
            if let Some(n) = b.current_page()
                && let Some(page) = b.nth_page(Some(n)).and_downcast::<gtk::Box>()
            {
                let mut child = page.first_child();
                while let Some(widget) = child {
                    child = widget.next_sibling();
                    if let Ok(view) = widget.downcast::<webkit6::WebView>() {
                        view.set_zoom_level(if delta == 0.0 {
                            1.0
                        } else {
                            (view.zoom_level() + delta).clamp(0.3, 3.0)
                        });
                        break;
                    }
                }
            }
        });
    }
    let b = book.clone();
    let s = session.clone();
    marks.set_popover(Some(&gtk::Popover::new()));
    marks.connect_active_notify(move |menu| {
        if !menu.is_active() {
            return;
        }
        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let entries: Vec<(String, String)> = std::fs::read(glib::user_data_dir().join("meridian/drift/bookmarks.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if entries.is_empty() {
            list.append(&gtk::Label::new(Some("No bookmarks")));
        }
        for (name, uri) in entries {
            let button = gtk::Button::with_label(&name);
            let (b, s) = (b.clone(), s.clone());
            button.connect_clicked(move |_| {
                tab(&b, &s, &uri, None);
            });
            list.append(&button);
        }
        if let Some(pop) = menu.popover() {
            pop.set_child(Some(&list));
        }
    });
    let (b, s) = (book.clone(), session.clone());
    history.set_popover(Some(&gtk::Popover::new()));
    history.connect_active_notify(move |menu| {
        if !menu.is_active() {
            return;
        }
        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        for set in [
            gtk::Widget::set_margin_top,
            gtk::Widget::set_margin_bottom,
            gtk::Widget::set_margin_start,
            gtk::Widget::set_margin_end,
        ] {
            set(list.upcast_ref(), 10);
        }
        let entries: Vec<(String, String)> = std::fs::read(history_path())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        if entries.is_empty() {
            list.append(&gtk::Label::new(Some("Your recent pages will appear here")));
        }
        for (name, uri) in entries.iter().take(50) {
            let label = gtk::Label::new(Some(name));
            label.set_max_width_chars(44);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_xalign(0.0);
            let button = gtk::Button::builder().child(&label).tooltip_text(uri).build();
            let (b, s, uri, popover) = (b.clone(), s.clone(), uri.clone(), menu.popover());
            button.connect_clicked(move |_| {
                tab(&b, &s, &uri, None);
                if let Some(popover) = &popover {
                    popover.popdown();
                }
            });
            list.append(&button);
        }
        if !entries.is_empty() {
            let clear = gtk::Button::with_label("Clear browsing history");
            let weak_list = list.downgrade();
            clear.connect_clicked(move |_| {
                let Some(list) = weak_list.upgrade() else {
                    return;
                };
                if let Err(error) = std::fs::write(history_path(), b"[]") {
                    list.append(&gtk::Label::new(Some(&format!("Couldn’t clear history: {error}"))));
                    return;
                }
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                list.append(&gtk::Label::new(Some("Browsing history cleared")));
            });
            list.append(&clear);
        }
        let scroll = gtk::ScrolledWindow::builder()
            .max_content_height(420)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .build();
        if let Some(popover) = menu.popover() {
            popover.set_child(Some(&scroll));
        }
    });
    let weak = win.downgrade();
    session.connect_download_started(move |_, download| {
        let weak = weak.clone();
        download.connect_decide_destination(move |d, name| {
            let Some(win) = weak.upgrade() else {
                d.cancel();
                return true;
            };
            let dialog = gtk::FileDialog::builder()
                .title("Save download")
                .initial_name(std::path::Path::new(name).file_name().and_then(|n| n.to_str()).unwrap_or("Download"))
                .build();
            let d = d.clone();
            glib::spawn_future_local(async move {
                match dialog.save_future(Some(&win)).await {
                    Ok(file) => d.set_destination(&file.uri()),
                    Err(_) => d.cancel(),
                }
            });
            true
        });
    });
    let keys = gtk::EventControllerKey::new();
    let (b, s) = (book.clone(), session.clone());
    keys.connect_key_pressed(move |_, key, _, mods| {
        if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            if key == gtk::gdk::Key::t {
                tab(&b, &s, "https://duckduckgo.com/", None);
                return glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::w {
                if let Some(n) = b.current_page() {
                    b.remove_page(Some(n));
                }
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    win.add_controller(keys);
    win.present();
}
fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id("org.meridian.Drift").build();
    app.connect_activate(|app| window(app, false));
    app.run_with_args::<&str>(&[])
}
#[cfg(test)]
mod tests {
    use super::{destination, remember};
    #[test]
    fn history_is_recent_unique_and_bounded() {
        let mut entries = Vec::new();
        for n in 0..220 {
            remember(&mut entries, format!("Page {n}"), format!("https://example.com/{n}"));
        }
        assert_eq!(entries.len(), 200);
        remember(&mut entries, "Updated title".into(), "https://example.com/100".into());
        assert_eq!(entries.len(), 200);
        assert_eq!(entries[0], ("Updated title".into(), "https://example.com/100".into()));
        assert_eq!(entries.iter().filter(|(_, uri)| uri.ends_with("/100")).count(), 1);
    }
    #[test]
    fn routes_addresses_and_searches() {
        assert_eq!(destination("example.com"), "https://example.com");
        assert_eq!(destination("cats & dogs"), "https://duckduckgo.com/?q=cats%20%26%20dogs");
        assert!(destination("javascript:alert(1)").starts_with("https://duckduckgo.com/?q="));
    }
}
