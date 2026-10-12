use crate::{settings::Settings, AppState};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

const ID: &str = "main";

pub fn create(app: &AppHandle, s: &Settings) -> tauri::Result<()> {
    TrayIconBuilder::with_id(ID)
        .tooltip("Cursora")
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
    }
}

fn menu(app: &AppHandle, s: &Settings) -> tauri::Result<Menu<Wry>> {
    // Persona list arrives with spec 05.
    let personas = Submenu::with_items(app, "Persona", false, &[])?;
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "settings", "Open Settings", true, None::<&str>)?,
            &personas,
            &CheckMenuItem::with_id(app, "personality", "Personality", true, s.personality_on, None::<&str>)?,
            &CheckMenuItem::with_id(app, "local_only", "Local-only", true, s.local_only, None::<&str>)?,
            &CheckMenuItem::with_id(app, "pause", "Pause Cursora", true, s.paused, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
        ],
    )
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let mut s = app.state::<AppState>().settings.lock().unwrap().clone();
    match event.id.as_ref() {
        "settings" => return crate::open_window(app, "settings"),
        "quit" => return app.exit(0),
        "personality" => s.personality_on = !s.personality_on,
        "local_only" => s.local_only = !s.local_only,
        "pause" => s.paused = !s.paused,
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
