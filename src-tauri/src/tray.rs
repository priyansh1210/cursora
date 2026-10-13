use crate::capture::{self, Rule, SourceApp};
use crate::{settings::Settings, AppState};
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

const ID: &str = "main";
/// The tray's word for local-only mode while it is on (PRI-1.2).
const LOCAL_ONLY: &str = "🔒 Local-only";

/// The app the bubble last opened over. While the tray menu is open, the app in front is the taskbar (PRI-2.4).
static LAST_APP: Mutex<Option<SourceApp>> = Mutex::new(None);

pub fn create(app: &AppHandle, s: &Settings) -> tauri::Result<()> {
    TrayIconBuilder::with_id(ID)
        .tooltip(tooltip(s))
        .icon(icon(app, s.paused))
        .menu(&menu(app, s)?)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu)
        .build(app)?;
    Ok(())
}

/// Rebuilds the menu and icon so they match the settings (APP-1.2, APP-1.3).
pub fn refresh(app: &AppHandle, s: &Settings) {
    if let Some(tray) = app.tray_by_id(ID) {
        if let Err(e) = menu(app, s).and_then(|m| tray.set_menu(Some(m))) {
            log::error!("tray menu: {e}");
        }
        let _ = tray.set_icon(Some(icon(app, s.paused)));
        let _ = tray.set_tooltip(Some(tooltip(s)));
    }
}

fn tooltip(s: &Settings) -> String {
    if s.local_only { format!("Cursora · {LOCAL_ONLY}") } else { "Cursora".into() }
}

/// After each capture. The menu is rebuilt only when the app changed, so "Block Cursora in <app>" names it.
pub fn seen(app: &AppHandle, source: &SourceApp) {
    let changed = LAST_APP.lock().unwrap().replace(source.clone()).is_none_or(|last| last.exe != source.exe);
    if changed {
        // A copy: the lock must not be held while the menu waits for the main thread, which may want it too.
        let s = app.state::<AppState>().settings.lock().unwrap().clone();
        refresh(app, &s);
    }
}

fn menu(app: &AppHandle, s: &Settings) -> tauri::Result<Menu<Wry>> {
    // PER-3.2: every persona, the active one checked.
    let active = crate::persona::active(s).id;
    let choices = crate::persona::all(s)
        .into_iter()
        .map(|p| CheckMenuItem::with_id(app, format!("persona:{}", p.id), &p.name, true, p.id == active, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let personas = Submenu::with_items(app, "Persona", true, &choices.iter().map(|c| c as &dyn IsMenuItem<Wry>).collect::<Vec<_>>())?;
    let open = MenuItem::with_id(app, "settings", "Open Settings", true, None::<&str>)?;
    let personality = CheckMenuItem::with_id(app, "personality", "Personality", true, s.personality_on, None::<&str>)?;
    let local_only = CheckMenuItem::with_id(app, "local_only", if s.local_only { LOCAL_ONLY } else { "Local-only" }, true, s.local_only, None::<&str>)?;
    // PRI-2.4: unless the app is unknown or already blocked.
    let last = LAST_APP.lock().unwrap().clone().filter(|a| !a.exe.is_empty() && !capture::blocked(&s.capture.blocklist, a));
    let block = last.map(|a| MenuItem::with_id(app, "block", format!("Block Cursora in {}", if a.name.is_empty() { &a.exe } else { &a.name }), true, None::<&str>)).transpose()?;
    let pause = CheckMenuItem::with_id(app, "pause", "Pause Cursora", true, s.paused, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop_model", "Stop the model", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![&open, &personas, &personality, &local_only, &pause];
    if let Some(block) = &block {
        items.push(block);
    }
    // Only with a built-in model set up (LLM-3.5).
    if s.models.providers.iter().any(|p| p.engine.is_some()) {
        items.push(&stop);
    }
    items.extend([&separator as &dyn IsMenuItem<Wry>, &quit]);
    Menu::with_items(app, &items)
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let mut s = app.state::<AppState>().settings.lock().unwrap().clone();
    match event.id.as_ref() {
        "settings" => return crate::open_window(app, "settings"),
        "quit" => return app.exit(0),
        "stop_model" => return crate::llm::engine::stop_by_hand(app),
        "personality" => s.personality_on = !s.personality_on,
        "local_only" => s.local_only = !s.local_only,
        "block" => match LAST_APP.lock().unwrap().clone() {
            Some(a) => s.capture.blocklist.push(Rule::Process(a.exe)),
            None => return,
        },
        "pause" => s.paused = !s.paused,
        // Choosing a persona means wanting it, so personality comes on too (like `/persona`).
        id if id.starts_with("persona:") => (s.active_persona, s.personality_on) = (id["persona:".len()..].into(), true),
        _ => return,
    }
    if let Err(e) = crate::update_settings(app, s) {
        log::error!("saving settings from tray: {e}");
    }
}

/// The paused icon is the app icon in grayscale, so no extra asset is needed.
fn icon(app: &AppHandle, paused: bool) -> Image<'static> {
    let base = app.default_window_icon().expect("bundle icon").clone().to_owned();
    if !paused {
        return base;
    }
    let mut rgba = base.rgba().to_vec();
    for px in rgba.chunks_exact_mut(4) {
        let gray = ((px[0] as u32 * 30 + px[1] as u32 * 59 + px[2] as u32 * 11) / 100) as u8;
        px[..3].fill(gray);
    }
    Image::new_owned(rgba, base.width(), base.height())
}
