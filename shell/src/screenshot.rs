//! Native screenshot preview, non-destructive crop, pen annotation and export.
use gtk::{gdk, gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, PartialEq)]
struct Edit {
    crop: (i32, i32, i32, i32),
    strokes: Vec<Vec<(f64, f64)>>,
}
fn paint(cr: &gtk::cairo::Context, image: &gtk::gdk_pixbuf::Pixbuf, edit: &Edit) {
    cr.set_source_pixbuf(image, 0.0, 0.0);
    let _ = cr.paint();
    cr.set_source_rgb(0.85, 0.08, 0.08);
    cr.set_line_width(4.0);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    for stroke in &edit.strokes {
        if let Some((x, y)) = stroke.first() {
            cr.move_to(*x, *y);
            for (x, y) in stroke.iter().skip(1) {
                cr.line_to(*x, *y);
            }
            let _ = cr.stroke();
        }
    }
}
fn texture(image: &gtk::gdk_pixbuf::Pixbuf, edit: &Edit) -> Result<gdk::MemoryTexture, String> {
    let (x, y, w, h) = edit.crop;
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, w, h).map_err(|e| e.to_string())?;
    {
        let cr = gtk::cairo::Context::new(&surface).map_err(|e| e.to_string())?;
        cr.translate(-f64::from(x), -f64::from(y));
        paint(&cr, image, edit);
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let bytes = glib::Bytes::from_owned(surface.data().map_err(|e| e.to_string())?.to_vec());
    let format = if cfg!(target_endian = "little") {
        gdk::MemoryFormat::B8g8r8a8Premultiplied
    } else {
        gdk::MemoryFormat::A8r8g8b8Premultiplied
    };
    Ok(gdk::MemoryTexture::new(w, h, format, &bytes, stride))
}
thread_local! { static CAPTURING: Cell<bool> = const { Cell::new(false) }; }
pub fn capture(app: &gtk::Application, mode: &'static str) {
    if CAPTURING.with(|busy| busy.replace(true)) {
        return;
    }
    let app = app.clone();
    glib::spawn_future_local(async move {
        let result = crate::files::blocking(move || {
            let output = std::process::Command::new("sh")
                .args(["-c", include_str!("../../session/screenshot.sh"), "meridian-screenshot", mode])
                .output()
                .map_err(|e| meridian_protocol::ErrorBody::new(meridian_protocol::ErrorCode::Failed, e.to_string()))?;
            if !output.status.success() {
                return Err(meridian_protocol::ErrorBody::new(
                    meridian_protocol::ErrorCode::Failed,
                    "Screenshot capture failed; check grim/slurp availability.",
                ));
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        })
        .await;
        CAPTURING.with(|busy| busy.set(false));
        match result {
            Ok(path) if !path.is_empty() => {
                if let Err(error) = preview(&app, &path) {
                    let notification = gio::Notification::new("Screenshot saved; preview unavailable");
                    notification.set_body(Some(&error));
                    app.send_notification(None, &notification);
                }
            }
            Ok(_) => {}
            Err(error) => {
                let notification = gio::Notification::new("Screenshot failed");
                notification.set_body(Some(&error.message));
                app.send_notification(None, &notification);
            }
        }
    });
}
fn preview(app: &gtk::Application, path: &str) -> Result<(), String> {
    let (_, width, height) =
        gtk::gdk_pixbuf::Pixbuf::file_info(path).ok_or("Could not inspect screenshot dimensions")?;
    if width < 1 || height < 1 || i64::from(width) * i64::from(height) > 32_000_000 {
        return Err("The original was saved. Preview is limited to 32 megapixels.".into());
    }
    let image = gtk::gdk_pixbuf::Pixbuf::from_file(path).map_err(|e| e.to_string())?;
    let edit = Rc::new(RefCell::new(Edit { crop: (0, 0, width, height), strokes: vec![] }));
    let saved = Rc::new(RefCell::new(edit.borrow().clone()));
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Screenshot")
        .default_width(960)
        .default_height(700)
        .build();
    {
        let edit = edit.clone();
        let saved = saved.clone();
        let confirming = Rc::new(Cell::new(false));
        window.connect_close_request(move |window| {
            if *edit.borrow() == *saved.borrow() {
                return glib::Propagation::Proceed;
            }
            if confirming.replace(true) {
                return glib::Propagation::Stop;
            }
            let dialog = gtk::MessageDialog::builder()
                .transient_for(window)
                .modal(true)
                .text("Discard screenshot edits?")
                .secondary_text("The original capture is already saved. Unsaved crop and annotations will be lost.")
                .build();
            dialog.add_button("Keep editing", gtk::ResponseType::Cancel);
            dialog.add_button("Discard edits", gtk::ResponseType::Accept);
            dialog.set_default_response(gtk::ResponseType::Cancel);
            let window = window.downgrade();
            let saved = saved.clone();
            let edit = edit.clone();
            let confirming = confirming.clone();
            dialog.connect_response(move |dialog, response| {
                confirming.set(false);
                dialog.destroy();
                if response == gtk::ResponseType::Accept {
                    *saved.borrow_mut() = edit.borrow().clone();
                    if let Some(window) = window.upgrade() {
                        window.close();
                    }
                }
            });
            dialog.present();
            glib::Propagation::Stop
        });
    }
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    let status = gtk::Label::new(Some(&format!(
        "Original saved to {path}. Drag to draw; crop with the numeric controls. Save a copy to keep edits."
    )));
    status.set_wrap(true);
    status.set_selectable(true);
    root.append(&status);
    let area = gtk::DrawingArea::new();
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.set_focusable(true);
    area.set_tooltip_text(Some(
        "Screenshot preview. Drag to draw red annotations. Crop and export controls are keyboard accessible.",
    ));
    let scale = Rc::new(Cell::new(1.0f64));
    {
        let image = image.clone();
        let edit = edit.clone();
        let scale = scale.clone();
        area.set_draw_func(move |_, cr, w, h| {
            let factor = (f64::from(w) / f64::from(width)).min(f64::from(h) / f64::from(height)).min(1.0);
            scale.set(factor);
            cr.scale(factor, factor);
            paint(cr, &image, &edit.borrow());
            let (x, y, w, h) = edit.borrow().crop;
            cr.set_source_rgb(0.1, 0.7, 0.25);
            cr.set_line_width(2.0 / factor.max(0.01));
            cr.rectangle(f64::from(x), f64::from(y), f64::from(w), f64::from(h));
            let _ = cr.stroke();
        });
    }
    let gesture = gtk::GestureDrag::new();
    gesture.set_button(1);
    let start = Rc::new(Cell::new((0.0, 0.0)));
    let drawing = Rc::new(Cell::new(false));
    {
        let edit = edit.clone();
        let start = start.clone();
        let scale = scale.clone();
        let drawing = drawing.clone();
        gesture.connect_drag_begin(move |_, x, y| {
            let x = x / scale.get();
            let y = y / scale.get();
            drawing.set(false);
            if x < 0.0
                || y < 0.0
                || x >= f64::from(width)
                || y >= f64::from(height)
                || edit.borrow().strokes.len() >= 100
            {
                return;
            }
            start.set((x, y));
            edit.borrow_mut().strokes.push(vec![(x, y)]);
            drawing.set(true);
        });
    }
    {
        let edit = edit.clone();
        let start = start.clone();
        let scale = scale.clone();
        let drawing = drawing.clone();
        let area = area.downgrade();
        gesture.connect_drag_update(move |_, dx, dy| {
            if !drawing.get() {
                return;
            }
            let (x, y) = start.get();
            if let Some(stroke) = edit.borrow_mut().strokes.last_mut() {
                if stroke.len() < 4096 {
                    stroke.push((
                        (x + dx / scale.get()).clamp(0.0, f64::from(width)),
                        (y + dy / scale.get()).clamp(0.0, f64::from(height)),
                    ));
                }
            }
            if let Some(area) = area.upgrade() {
                area.queue_draw();
            }
        });
    }
    area.add_controller(gesture);
    root.append(&area);
    let crop = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mut spins = vec![];
    for (label, value, max) in
        [("Left", 0, width - 1), ("Top", 0, height - 1), ("Width", width, width), ("Height", height, height)]
    {
        let field = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let title = gtk::Label::new(Some(label));
        let spin = gtk::SpinButton::with_range(0.0, f64::from(max), 1.0);
        spin.set_value(f64::from(value));
        spin.set_tooltip_text(Some(label));
        title.set_mnemonic_widget(Some(&spin));
        field.append(&title);
        field.append(&spin);
        crop.append(&field);
        spins.push(spin);
    }
    let apply = gtk::Button::with_label("Apply crop");
    {
        let edit = edit.clone();
        let area = area.clone();
        let status = status.clone();
        let spins = spins.clone();
        apply.connect_clicked(move |_| {
            let (x, y, w, h) =
                (spins[0].value_as_int(), spins[1].value_as_int(), spins[2].value_as_int(), spins[3].value_as_int());
            if w < 1 || h < 1 || x + w > width || y + h > height {
                status.set_label("Crop must stay inside the image and have positive width and height.");
                return;
            }
            edit.borrow_mut().crop = (x, y, w, h);
            area.queue_draw();
            status.set_label("Crop selected. Copy or Save a copy exports the outlined area.");
        });
    }
    crop.append(&apply);
    let reset = gtk::Button::with_label("Reset crop");
    {
        let edit = edit.clone();
        let area = area.clone();
        reset.connect_clicked(move |_| {
            edit.borrow_mut().crop = (0, 0, width, height);
            for (spin, value) in spins.iter().zip([0, 0, width, height]) {
                spin.set_value(f64::from(value));
            }
            area.queue_draw();
        });
    }
    crop.append(&reset);
    root.append(&crop);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let undo = gtk::Button::with_label("Undo annotation");
    {
        let edit = edit.clone();
        let area = area.clone();
        undo.connect_clicked(move |_| {
            edit.borrow_mut().strokes.pop();
            area.queue_draw();
        });
    }
    actions.append(&undo);
    let copy = gtk::Button::with_label("Copy image");
    {
        let edit = edit.clone();
        let image = image.clone();
        let status = status.clone();
        copy.connect_clicked(move |_| match texture(&image, &edit.borrow()) {
            Ok(texture) => {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_texture(&texture);
                    status.set_label("Edited image copied to the clipboard.");
                }
            }
            Err(error) => status.set_label(&error),
        });
    }
    actions.append(&copy);
    let save = gtk::Button::with_label("Save a copy…");
    {
        let saved = saved.clone();
        let window = window.downgrade();
        let edit = edit.clone();
        let image = image.clone();
        let status = status.clone();
        save.connect_clicked(move |button| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let saved = saved.clone();
            let image = image.clone();
            let edit = edit.borrow().clone();
            let status = status.clone();
            let button = button.clone();
            button.set_sensitive(false);
            glib::spawn_future_local(async move {
                let dialog = gtk::FileDialog::builder()
                    .title("Save edited screenshot")
                    .initial_name("Screenshot-edited.png")
                    .build();
                let chosen = dialog.save_future(Some(&window)).await;
                if let Err(error) = &chosen {
                    if !error.matches(gtk::DialogError::Dismissed) {
                        status.set_label(&format!("Could not open Save dialog: {error}"));
                    }
                }
                if let Ok(file) = chosen {
                    match texture(&image, &edit) {
                        Ok(texture) => {
                            let bytes = texture.save_to_png_bytes();
                            match file.replace_contents_future(bytes, None, false, gio::FileCreateFlags::PRIVATE).await
                            {
                                Ok(_) => {
                                    *saved.borrow_mut() = edit.clone();
                                    status.set_label("Edited screenshot saved.");
                                }
                                Err((_, error)) => status.set_label(&format!("Could not save: {error}")),
                            }
                        }
                        Err(error) => status.set_label(&error),
                    }
                }
                button.set_sensitive(true);
            });
        });
    }
    actions.append(&save);
    root.append(&actions);
    window.set_child(Some(&root));
    window.present();
    Ok(())
}
