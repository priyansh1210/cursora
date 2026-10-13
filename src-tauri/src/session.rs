//! Which display session Cursora runs in (LNX-1.1). On Linux it decides how the selection is read and whether a hotkey can be registered.
use serde::Serialize;
use std::sync::OnceLock;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Session {
    Windows,
    X11,
    Wayland,
    Unknown,
}

/// `XDG_SESSION_TYPE` first; for any other value (`tty`, unset) the display variables decide.
pub fn detect(xdg_session_type: Option<&str>, wayland_display: bool, display: bool) -> Session {
    match xdg_session_type {
        Some("x11") => Session::X11,
        Some("wayland") => Session::Wayland,
        _ if wayland_display => Session::Wayland,
        _ if display => Session::X11,
        _ => Session::Unknown,
    }
}

/// Read once. The XWayland switch sets no variable that `detect` reads, so a Wayland session stays Wayland.
pub fn current() -> Session {
    static SESSION: OnceLock<Session> = OnceLock::new();
    *SESSION.get_or_init(|| {
        if cfg!(windows) {
            return Session::Windows;
        }
        let set = |name: &str| std::env::var_os(name).is_some_and(|v| !v.is_empty());
        detect(std::env::var("XDG_SESSION_TYPE").ok().as_deref(), set("WAYLAND_DISPLAY"), set("DISPLAY"))
    })
}

/// On native Wayland a GTK window can't be placed, kept on top or kept off the taskbar, and may not get focus.
/// So where XWayland is available, Cursora's own windows use it. Must run before GTK starts.
#[cfg(target_os = "linux")]
pub fn prefer_xwayland() {
    if current() == Session::Wayland && std::env::var_os("DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() {
        // SAFETY: called first thing in `run()`, while no other thread exists to read the environment.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lnx_1_1_the_session_type_comes_from_the_environment() {
        assert_eq!(detect(Some("x11"), true, true), Session::X11, "XDG_SESSION_TYPE wins");
        assert_eq!(detect(Some("wayland"), false, true), Session::Wayland);
        assert_eq!(detect(Some("tty"), true, true), Session::Wayland, "then WAYLAND_DISPLAY");
        assert_eq!(detect(None, false, true), Session::X11, "then DISPLAY");
        assert_eq!(detect(None, false, false), Session::Unknown);
        assert_eq!(serde_json::to_string(&Session::X11).unwrap(), "\"x11\"");
    }
}
