//! The chat bubble window (spec 03).
#[cfg(not(windows))]
mod other;
#[cfg(windows)]
mod windows;
#[cfg(not(windows))]
use other as os;
#[cfg(windows)]
use self::windows as os;

use serde::Serialize;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const LABEL: &str = "bubble";
const WIDTH: f64 = 380.0;
const MAX_HEIGHT: f64 = 520.0;
const START_HEIGHT: f64 = 160.0;
/// From the cursor to the bubble's corner, in physical pixels (BUB-1.1).
const OFFSET: i32 = 16;

/// Physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn contains(&self, (x, y): (i32, i32)) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub area: Rect,
    pub work: Rect,
    pub scale: f64,
}

/// Where the bubble hangs: decided once per open, so growing never makes it jump sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    cursor: Option<(i32, i32)>,
    screen: Screen,
    left: bool,
    up: bool,
}

/// The screen under the cursor (BUB-1.3); without a cursor, the fallback (active or primary, BUB-1.4).
pub fn screen_for(cursor: Option<(i32, i32)>, screens: &[Screen], fallback: usize) -> Option<Screen> {
    cursor.and_then(|c| screens.iter().find(|s| s.area.contains(c))).or(screens.get(fallback)).copied()
}

/// Flips by the largest size the bubble can grow to, so a long answer can't push it off screen (BUB-1.2).
pub fn anchor(cursor: Option<(i32, i32)>, screen: Screen) -> Anchor {
    let (w, max_h) = (px(WIDTH, screen.scale), px(MAX_HEIGHT, screen.scale));
    let work = screen.work;
    let (left, up) = cursor.map_or((false, false), |(x, y)| (x + OFFSET + w > work.x + work.w, y + OFFSET + max_h > work.y + work.h));
    Anchor { cursor, screen, left, up }
}

/// The bubble's rectangle for a content height in logical px: beside the cursor (BUB-1.1), clamped into the work area (BUB-1.2).
pub fn rect(a: &Anchor, height: f64) -> Rect {
    let (work, scale) = (a.screen.work, a.screen.scale);
    let (w, h) = (px(WIDTH, scale), px(height.min(MAX_HEIGHT), scale));
    let (x, y) = match a.cursor {
        Some((cx, cy)) => (if a.left { cx - OFFSET - w } else { cx + OFFSET }, if a.up { cy - OFFSET - h } else { cy + OFFSET }),
        None => (work.x + (work.w - w) / 2, work.y + (work.h - h) / 2), // BUB-1.4
    };
    Rect { x: x.min(work.x + work.w - w).max(work.x), y: y.min(work.y + work.h - h).max(work.y), w, h }
}

fn px(logical: f64, scale: f64) -> i32 {
    (logical * scale).round() as i32
}

#[derive(Default)]
pub struct BubbleState(Mutex<Placed>);

fn placed(app: &AppHandle) -> MutexGuard<'_, Placed> {
    app.state::<BubbleState>().inner().0.lock().unwrap()
}

#[derive(Default)]
struct Placed {
    anchor: Option<Anchor>,
    height: f64,
    /// The window that was in front before the bubble, to give focus back on close (BUB-2.3).
    previous: isize,
}

/// Created once and kept hidden, so opening it is only move + show (BUB-2.1, constitution §8).
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .additional_browser_args(crate::BROWSER_ARGS)
        .title("Cursora")
        .inner_size(WIDTH, START_HEIGHT)
        .decorations(false)
        .shadow(true) // Windows 11 rounds the corners of a frameless window that has a shadow
        .always_on_top(true)
        .skip_taskbar(true)
        // GTK keeps a non-resizable window at its natural height (200 px), so a short bubble hung off the bottom
        // edge on Linux (BUB-1.2). Frameless, it has no resize edges either way.
        .resizable(cfg!(not(windows)))
        .visible(false)
        .focused(false)
        .on_navigation(|url| is_app_url(url))
        .build()?;
    os::hide_from_alt_tab(&window);
    Ok(())
}

/// Links in answers open in the browser (BUB-4.2); Cursora's own windows never navigate away from the app.
pub fn is_app_url(url: &Url) -> bool {
    let dev_server = cfg!(debug_assertions) && url.host_str() == Some("localhost") && url.port() == Some(1420);
    url.scheme() == "tauri" || url.host_str() == Some("tauri.localhost") || dev_server
}

pub fn is_open(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL).and_then(|w| w.is_visible().ok()).unwrap_or(false)
}

/// Places the bubble by the cursor, hands the view its payload, then shows and focuses it (BUB-2.2).
pub fn open(app: &AppHandle, cursor: Option<(i32, i32)>, previous: isize, payload: impl Serialize + Clone) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let monitors = app.available_monitors().unwrap_or_default();
    let primary = app.primary_monitor().ok().flatten().and_then(|p| monitors.iter().position(|m| m.position() == p.position())).unwrap_or(0);
    let screens: Vec<Screen> = monitors.iter().map(screen).collect();
    let Some(screen) = screen_for(cursor, &screens, primary) else {
        return log::warn!("bubble: no monitor found");
    };
    let anchor = anchor(cursor, screen);
    {
        let mut placed = placed(app);
        *placed = Placed { anchor: Some(anchor), height: START_HEIGHT, previous };
    }
    apply(&window, rect(&anchor, START_HEIGHT));
    let _ = window.emit_to(LABEL, "bubble-open", payload);
    let _ = window.show();
    // On X11 the window manager places the window as it appears, after this move; so the place is set again a moment later (BUB-1.1).
    #[cfg(not(windows))]
    {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            reapply(&app);
        });
    }
    os::focus(&window);
}

/// Hides the bubble and cancels its running answer (BUB-2.6); if it had focus, focus goes back to the app it opened over (BUB-2.3).
pub fn close(app: &AppHandle) {
    crate::chat::cancel(app, LABEL);
    if hide(app) {
        let previous = placed(app).previous; // not held while activating
        os::activate(previous);
    }
}

/// Hides the bubble and clears its view. Returns whether it had focus.
pub fn hide(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window(LABEL) else { return false };
    let had_focus = os::foreground() == os::handle(&window);
    let _ = window.hide();
    let _ = window.emit_to(LABEL, "bubble-hidden", ());
    crate::llm::engine::release(app); // "Only while answering" (LLM-3.5)
    had_focus
}

/// The view reports its content height; the bubble grows up to MAX_HEIGHT and then scrolls (BUB-2.5).
#[tauri::command]
pub fn bubble_resize(app: AppHandle, height: f64) {
    placed(&app).height = height;
    reapply(&app);
}

/// Puts the bubble where its anchor and current height say.
/// The lock is let go first: on Linux, moving the window waits for the main thread, which may be waiting for this lock in `bubble_resize`.
fn reapply(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let (anchor, height) = {
        let placed = placed(app);
        (placed.anchor, placed.height)
    };
    if let Some(anchor) = anchor {
        apply(&window, rect(&anchor, height));
    }
}

/// `r` is the visible bubble. Windows adds an invisible frame around a frameless window with a shadow,
/// so the window's outer position is shifted by that frame.
/// Position, then size, then position again: moving onto a monitor with another scale makes Windows resize the window.
fn apply(window: &WebviewWindow, r: Rect) {
    let frame = match (window.inner_position(), window.outer_position()) {
        (Ok(inner), Ok(outer)) => (inner.x - outer.x, inner.y - outer.y),
        _ => (0, 0),
    };
    let position = PhysicalPosition::new(r.x - frame.0, r.y - frame.1);
    let _ = window.set_position(position);
    let _ = window.set_size(PhysicalSize::new(r.w as u32, r.h as u32));
    let _ = window.set_position(position);
}

fn screen(m: &Monitor) -> Screen {
    let (pos, size, work) = (m.position(), m.size(), m.work_area());
    Screen {
        area: Rect { x: pos.x, y: pos.y, w: size.width as i32, h: size.height as i32 },
        work: Rect { x: work.position.x, y: work.position.y, w: work.size.width as i32, h: work.size.height as i32 },
        scale: m.scale_factor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1920×1080 at 100 %, taskbar 40 px at the bottom.
    fn laptop() -> Screen {
        Screen { area: Rect { x: 0, y: 0, w: 1920, h: 1080 }, work: Rect { x: 0, y: 0, w: 1920, h: 1040 }, scale: 1.0 }
    }

    /// 2880×1620 at 150 %, left of the laptop, so its coordinates are negative.
    fn left_monitor() -> Screen {
        let r = Rect { x: -2880, y: 0, w: 2880, h: 1620 };
        Screen { area: r, work: r, scale: 1.5 }
    }

    fn inside(r: Rect, work: Rect) -> bool {
        r.x >= work.x && r.y >= work.y && r.x + r.w <= work.x + work.w && r.y + r.h <= work.y + work.h
    }

    #[test]
    fn bub_1_1_opens_at_cursor_plus_16() {
        let r = rect(&anchor(Some((500, 300)), laptop()), 200.0);
        assert_eq!(r, Rect { x: 516, y: 316, w: 380, h: 200 });
    }

    #[test]
    fn bub_1_2_flips_then_clamps_near_edges_and_corners() {
        let s = laptop();
        // Right edge: flipped left, right edge of the bubble 16 px left of the cursor.
        let r = rect(&anchor(Some((1800, 300)), s), 200.0);
        assert_eq!((r.x, r.y), (1800 - 16 - 380, 316));
        // Bottom edge: flipped up, bottom of the bubble 16 px above the cursor.
        let r = rect(&anchor(Some((500, 1000)), s), 200.0);
        assert_eq!((r.x, r.y + r.h), (516, 1000 - 16));
        // Every corner and edge: always fully inside the work area.
        for cursor in [(0, 0), (1919, 0), (0, 1079), (1919, 1079), (960, 0), (960, 1079), (0, 520), (1919, 520), (5, 1035)] {
            for height in [100.0, 520.0, 900.0] {
                let r = rect(&anchor(Some(cursor), s), height);
                assert!(inside(r, s.work), "{cursor:?} h={height} → {r:?}");
            }
        }
    }

    #[test]
    fn bub_1_2_growing_never_changes_the_side() {
        // Room below for a small bubble but not for a full-height one: it opens above and grows upward.
        let a = anchor(Some((500, 700)), laptop());
        let (small, big) = (rect(&a, 120.0), rect(&a, 520.0));
        assert!(small.y + small.h <= 700 && big.y + big.h <= 700);
        assert_eq!(small.y + small.h, big.y + big.h);
    }

    #[test]
    fn bub_1_3_uses_the_scale_of_the_cursors_monitor() {
        let screens = [laptop(), left_monitor()];
        let s = screen_for(Some((-1000, 500)), &screens, 0).unwrap();
        assert_eq!(s, left_monitor());
        let r = rect(&anchor(Some((-1000, 500)), s), 200.0);
        assert_eq!(r, Rect { x: -984, y: 516, w: 570, h: 300 });
        // Right edge of the left monitor flips with the scaled width.
        let r = rect(&anchor(Some((-100, 500)), s), 200.0);
        assert_eq!(r.x, -100 - 16 - 570);
    }

    #[test]
    fn bub_1_4_without_a_cursor_it_is_centered_on_the_fallback_screen() {
        let screens = [laptop(), left_monitor()];
        let s = screen_for(None, &screens, 1).unwrap();
        assert_eq!(s, left_monitor());
        let r = rect(&anchor(None, s), 200.0);
        assert_eq!(r, Rect { x: -2880 + (2880 - 570) / 2, y: (1620 - 300) / 2, w: 570, h: 300 });
        // A cursor outside every screen also falls back.
        assert_eq!(screen_for(Some((99_999, 0)), &screens, 0), Some(laptop()));
    }
}
