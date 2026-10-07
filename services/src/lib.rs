//! System integrations used by the Meridian shell.
//!
//! Each service exposes a small domain API (no D-Bus or Wayland details leak
//! out), so services can later move into a separate daemon without changing
//! callers.

pub mod apps;
pub mod search;
pub mod settings;
pub mod windows;

pub mod displays;
