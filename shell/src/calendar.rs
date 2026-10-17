use meridian_protocol::{CalendarEvent, ErrorBody, ErrorCode};
fn error(e: impl ToString) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
fn path() -> std::path::PathBuf {
    gtk::glib::user_config_dir().join("meridian/calendar.json")
}
pub fn list() -> Result<Vec<CalendarEvent>, ErrorBody> {
    match std::fs::read(path()) {
        Ok(data) => serde_json::from_slice(&data).map_err(error),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(error(e)),
    }
}
pub fn save(events: &[CalendarEvent]) -> Result<(), ErrorBody> {
    if events.len() > 10000
        || events.iter().any(|e| {
            e.id.len() > 100
                || e.title.trim().is_empty()
                || e.title.len() > 800
                || e.date.len() != 10
                || e.time.len() != 5
        })
    {
        return Err(error("Invalid calendar events"));
    }
    let path = path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(error)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_vec(events).map_err(error)?).map_err(error)?;
    std::fs::rename(temp, path).map_err(error)
}
