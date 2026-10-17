//! A small native volume manager. All changes go through UDisks authorization.
use gtk::prelude::*;
use serde_json::Value;

async fn command(args: &[&str]) -> Result<String, String> {
    let child = gio::Subprocess::newv(
        &args.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|e| e.to_string())?;
    let (out, err) = child.communicate_utf8_future(None).await.map_err(|e| e.to_string())?;
    if child.is_successful() {
        Ok(out.unwrap_or_default().to_string())
    } else {
        Err(err.unwrap_or_default().to_string())
    }
}
fn bytes(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}
fn capacity(value: &Value) -> String {
    let n = bytes(value).unwrap_or(0.0);
    if n >= 1e12 {
        format!("{:.1} TB", n / 1e12)
    } else if n >= 1e9 {
        format!("{:.1} GB", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.1} MB", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.1} KB", n / 1e3)
    } else {
        format!("{n:.0} B")
    }
}
fn filter_volumes(list: &gtk::Box, query: &str) {
    let query = query.to_lowercase();
    let mut child = list.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        let mut words = String::new();
        fn labels(widget: &gtk::Widget, text: &mut String) {
            if let Some(label) = widget.downcast_ref::<gtk::Label>() {
                text.push_str(&label.text());
                text.push(' ');
            }
            let mut child = widget.first_child();
            while let Some(w) = child {
                child = w.next_sibling();
                labels(&w, text);
            }
        }
        labels(&widget, &mut words);
        widget.set_visible(query.is_empty() || words.to_lowercase().contains(&query));
    }
}
fn can_unmount(mounts: &[&str]) -> bool {
    mounts.iter().all(|path| ["/media/", "/mnt/", "/run/media/"].iter().any(|prefix| path.starts_with(prefix)))
}
fn refresh(list: &gtk::Box, status: &gtk::Label, refresh: &gtk::Button, filter: &gtk::SearchEntry) {
    refresh.set_sensitive(false);
    let (list, status, refresh, filter) = (list.clone(), status.clone(), refresh.clone(), filter.clone());
    glib::spawn_future_local(async move {
        let result = command(&[
            "lsblk",
            "--json",
            "--bytes",
            "--output",
            "NAME,PATH,TYPE,SIZE,FSTYPE,LABEL,MOUNTPOINTS,MODEL,FSAVAIL",
        ])
        .await;
        match result.and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string())) {
            Ok(data) => {
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                status.set_text("Drives are up to date");
                if let Some(devices) = data["blockdevices"].as_array() {
                    for device in devices.iter().filter(|d| d["type"] == "disk") {
                        let title = device["model"].as_str().unwrap_or("Drive").trim();
                        let heading = gtk::Label::new(Some(&format!("{} · {}", title, capacity(&device["size"]))));
                        heading.set_xalign(0.0);
                        heading.add_css_class("volume-heading");
                        list.append(&heading);
                        volume(&list, &status, &refresh, &filter, device);
                    }
                    if list.first_child().is_none() {
                        status.set_text("No drives found");
                    }
                }
            }
            Err(e) => status.set_text(&e),
        }
        filter_volumes(&list, &filter.text());
        refresh.set_sensitive(true);
    });
}
fn volume(list: &gtk::Box, status: &gtk::Label, reload: &gtk::Button, filter: &gtk::SearchEntry, node: &Value) {
    let path = node["path"].as_str().unwrap_or_default();
    let mounted: Vec<_> = node["mountpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("volume-card");
    let drive_icon = gtk::Image::from_icon_name(if mounted.is_empty() {
        "drive-harddisk-symbolic"
    } else {
        "drive-harddisk-system-symbolic"
    });
    drive_icon.set_pixel_size(32);
    drive_icon.add_css_class("volume-icon");
    row.append(&drive_icon);
    row.set_margin_bottom(12);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    let label = node["label"].as_str().filter(|s| !s.is_empty()).unwrap_or(path);
    let title = gtk::Label::new(Some(label));
    title.set_xalign(0.0);
    text.append(&title);
    let detail = gtk::Label::new(Some(&format!(
        "{} · {} · {}{}",
        path,
        capacity(&node["size"]),
        node["fstype"].as_str().unwrap_or("Unformatted"),
        if mounted.is_empty() {
            String::new()
        } else if bytes(&node["fsavail"]).is_some() {
            format!(" · {} free · {}", capacity(&node["fsavail"]), mounted.join(", "))
        } else {
            format!(" · {}", mounted.join(", "))
        }
    )));
    detail.set_xalign(0.0);
    detail.set_wrap(true);
    detail.add_css_class("dim-label");
    text.append(&detail);
    if let (Some(size), Some(free)) = (bytes(&node["size"]), bytes(&node["fsavail"]))
        && size > 0.0
        && !mounted.is_empty()
    {
        let usage = gtk::ProgressBar::new();
        let fraction = ((size - free) / size).clamp(0.0, 1.0);
        usage.set_fraction(fraction);
        usage.set_tooltip_text(Some(&format!("{:.0}% used", fraction * 100.0)));
        text.append(&usage);
    }
    row.append(&text);
    let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy.set_tooltip_text(Some("Copy device path"));
    let device_path = path.to_string();
    copy.connect_clicked(move |button| {
        button.clipboard().set_text(&device_path);
        button.set_tooltip_text(Some("Device path copied"));
    });
    row.append(&copy);
    if let Some(mount) = mounted.first() {
        let open = gtk::Button::from_icon_name("folder-open-symbolic");
        open.set_tooltip_text(Some("Open volume"));
        let uri = gio::File::for_path(mount).uri();
        let status = status.clone();
        open.connect_clicked(move |_| {
            if let Err(e) = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
                status.set_text(&e.to_string());
            }
        });
        row.append(&open);
    }
    if node["fstype"].as_str().is_some_and(|s| !s.is_empty() && s != "swap" && s != "crypto_LUKS" && s != "LVM2_member")
        && path.starts_with("/dev/")
    {
        let button = gtk::Button::with_label(if mounted.is_empty() { "Mount" } else { "Unmount" });
        button.set_sensitive(can_unmount(&mounted));
        let action = if mounted.is_empty() { "mount" } else { "unmount" };
        let (path, list, status, reload, filter) =
            (path.to_string(), list.clone(), status.clone(), reload.clone(), filter.clone());
        button.connect_clicked(move |button| {
            button.set_sensitive(false);
            let (path, list, status, reload, button, filter) =
                (path.clone(), list.clone(), status.clone(), reload.clone(), button.clone(), filter.clone());
            glib::spawn_future_local(async move {
                match command(&["udisksctl", action, "--block-device", &path]).await {
                    Ok(_) => refresh(&list, &status, &reload, &filter),
                    Err(e) => {
                        status.set_text(&e);
                        button.set_sensitive(true);
                    }
                }
            });
        });
        row.append(&button);
    }
    list.append(&row);
    if let Some(children) = node["children"].as_array() {
        for child in children {
            volume(list, status, reload, filter, child);
        }
    }
}
fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id("org.meridian.Vault").build();
    app.connect_activate(|app| {
        meridian_shell::install_app_css();
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Vault")
            .default_width(760)
            .default_height(560)
            .build();
        window.add_css_class("meridian-app");
        let header = gtk::HeaderBar::new();
        let reload = gtk::Button::from_icon_name("view-refresh-symbolic");
        reload.set_tooltip_text(Some("Refresh drives"));
        header.pack_end(&reload);
        window.set_titlebar(Some(&header));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
        for set in [
            gtk::Widget::set_margin_top,
            gtk::Widget::set_margin_bottom,
            gtk::Widget::set_margin_start,
            gtk::Widget::set_margin_end,
        ] {
            set(content.upcast_ref(), 24);
        }
        let intro = gtk::Label::new(Some("Your storage, at a glance"));
        intro.set_xalign(0.0);
        intro.add_css_class("title-1");
        content.append(&intro);
        let subtitle =
            gtk::Label::new(Some("Browse volumes, check available space, and safely mount external drives."));
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.add_css_class("dim-label");
        content.append(&subtitle);
        let search = gtk::SearchEntry::builder().placeholder_text("Filter by name, device, or filesystem").build();
        content.append(&search);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 16);
        let filtered_list = list.clone();
        search.connect_search_changed(move |entry| filter_volumes(&filtered_list, &entry.text()));
        let status = gtk::Label::new(None);
        status.set_wrap(true);
        content.append(&list);
        content.append(&status);
        let scroll = gtk::ScrolledWindow::builder().child(&content).build();
        window.set_child(Some(&scroll));
        let (l, s, r) = (list.clone(), status.clone(), reload.clone());
        let filter = search.clone();
        reload.connect_clicked(move |_| refresh(&l, &s, &r, &filter));
        refresh(&list, &status, &reload, &search);
        window.present();
    });
    app.run_with_args::<&str>(&[])
}

#[cfg(test)]
mod tests {
    use super::{can_unmount, capacity};
    #[test]
    fn formats_small_volumes_and_lsblk_numeric_strings() {
        assert_eq!(capacity(&serde_json::json!(512)), "512 B");
        assert_eq!(capacity(&serde_json::json!(512000000)), "512.0 MB");
        assert_eq!(capacity(&serde_json::json!("1000000000")), "1.0 GB");
        assert_eq!(capacity(&serde_json::Value::Null), "0 B");
    }
    #[test]
    fn protects_system_and_mixed_mounts() {
        assert!(can_unmount(&[]));
        assert!(can_unmount(&["/media/user/Backup", "/mnt/backup"]));
        for path in ["/", "/boot", "/boot/efi", "/home", "/usr", "/var", "/run"] {
            assert!(!can_unmount(&[path]));
        }
        assert!(!can_unmount(&["/media/user/Backup", "/home"]));
    }
}
