//! Test-only executable: WebKit/GTK must initialize on the OS main thread.
fn main() {
    meridian_shell::init_logging();
    meridian_shell::shell::run_nested_smoke();
}
