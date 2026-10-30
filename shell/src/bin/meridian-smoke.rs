//! Test-only executable: WebKit/GTK must initialize on the OS main thread.
fn main() {
    meridian_shell::init_logging();
    if std::env::args().nth(1).as_deref() == Some("--scale") {
        assert_eq!(
            std::env::var("MERIDIAN_NESTED_TEST").as_deref(),
            Ok("1"),
            "scaling is only for isolated integration fixtures"
        );
        let scale: f64 = std::env::args().nth(2).expect("fixture scale").parse().expect("numeric scale");
        assert!([1., 1.25, 1.5, 2.].contains(&scale));
        let outputs = meridian_services::displays::list().expect("fixture displays");
        assert!(outputs.iter().all(|o| o.name.starts_with("HEADLESS-")), "scaler refuses physical outputs");
        let mut edits = meridian_services::displays::snapshot(&outputs);
        for edit in &mut edits {
            edit.scale = scale;
        }
        meridian_services::displays::apply(&edits).expect("fixture scale apply");
        return;
    }
    meridian_shell::shell::run_nested_smoke();
}
