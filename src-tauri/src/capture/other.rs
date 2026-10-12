use super::{Clip, CopyKey, Platform, SourceApp};
use std::time::Duration;

// shortcut: captures nothing outside Windows, so the bubble opens as a general chat; spec 08 adds X11 PRIMARY and Wayland.
pub struct NullPlatform;

impl NullPlatform {
    pub fn new() -> Self {
        NullPlatform
    }
}

impl Platform for NullPlatform {
    type Snapshot = ();
    fn foreground(&self) -> SourceApp {
        SourceApp::default()
    }
    fn accessible_selection(&self, _: usize) -> Option<String> {
        None
    }
    fn extra_modifiers_released(&self, _: Duration) -> bool {
        false // never simulate keys here
    }
    fn clipboard_seq(&self) -> u32 {
        0
    }
    fn clipboard_snapshot(&self) -> Option<()> {
        None
    }
    fn clipboard_restore(&self, _: ()) {}
    fn clipboard_read(&self) -> Clip {
        Clip::Nothing
    }
    fn send_copy(&self, _: CopyKey) {}
}
