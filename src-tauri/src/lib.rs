mod capture;
mod cli;
mod hotkey;
mod settings;
mod tray;

use settings::Settings;
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::RotationStrategy;
use tauri_plugin_opener::OpenerExt;

pub struct AppState {
    pub settings: Mutex<Settings>,
    path: PathBuf,
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    update_settings(&app, settings)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    os: &'static str,
    session: String,
    config_dir: String,
    log_dir: String,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    let dir = |d: tauri::Result<PathBuf>| d.map(|p| p.display().to_string()).unwrap_or_default();
    AppInfo {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS,
        // Spec 08 (LNX-1.1) refines Linux session detection.
        session: if cfg!(windows) { "windows".into() } else { std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into()) },
        config_dir: dir(app.path().app_config_dir()),
        log_dir: dir(app.path().app_log_dir()),
    }
}

#[tauri::command]
fn open_folder(app: AppHandle, kind: String) -> Result<(), String> {
    let dir = match kind.as_str() {
        "config" => app.path().app_config_dir(),
        "log" => app.path().app_log_dir(),
        _ => return Err(format!("unknown folder {kind:?}")),
    }
    .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_setup(app: AppHandle) {
    open_window(&app, "setup");
}

/// Saves, then applies the settings everywhere without a restart (APP-4.1).
/// A hotkey that is invalid or can't be registered is refused before anything is saved (SEL-1.2, SEL-1.3).
pub fn update_settings(app: &AppHandle, settings: Settings) -> Result<(), String> {
    let state = app.state::<AppState>();
    let old = state.settings.lock().unwrap().clone();
    hotkey::validate(&settings.hotkeys, &old.hotkeys)?;
    let hotkey_changed = old.hotkeys.ask != settings.hotkeys.ask || old.paused != settings.paused;
    if hotkey_changed {
        hotkey::apply(app, Some(&old), &settings)?;
    }
    if let Err(e) = settings::save(&state.path, &settings) {
        if hotkey_changed {
            let _ = hotkey::apply(app, None, &old);
        }
        return Err(format!("Couldn't save settings: {e}"));
    }
    *state.settings.lock().unwrap() = settings.clone();
    tray::refresh(app, &settings);
    sync_autostart(app, &settings);
    let _ = app.emit("settings-changed", &settings);
    Ok(())
}

/// Also run at startup, so "Start with system" in Settings always matches the OS (APP-3.1).
/// auto-launch is used directly because tauri-plugin-autostart writes the machine-wide (HKLM)
/// Run key whenever it has the rights; this is a per-user setting.
fn sync_autostart(app: &AppHandle, settings: &Settings) {
    let Ok(exe) = std::env::current_exe() else { return };
    #[cfg(target_os = "linux")]
    let exe = app.env().appimage.map(PathBuf::from).unwrap_or(exe);
    let launcher = auto_launch::AutoLaunchBuilder::new()
        .set_app_name(&app.package_info().name)
        .set_app_path(&exe.display().to_string())
        .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser)
        .build();
    let result = launcher.and_then(|l| match (l.is_enabled()?, settings.start_with_system) {
        (false, true) => l.enable(),
        (true, false) => l.disable(),
        _ => Ok(()),
    });
    if let Err(e) = result {
        log::warn!("autostart: {e}");
    }
}

/// Settings and Setup windows are created on demand and destroyed on close, to keep idle RAM low.
pub fn open_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.set_focus();
        return;
    }
    let (title, width, height) = if label == "setup" { ("Cursora setup", 520.0, 480.0) } else { ("Cursora settings", 760.0, 540.0) };
    if let Err(e) = WebviewWindowBuilder::new(app, label, WebviewUrl::default()).title(title).inner_size(width, height).build() {
        log::error!("opening {label} window: {e}");
    }
}

fn handle(app: &AppHandle, action: cli::Action) {
    match action {
        cli::Action::Settings => open_window(app, "settings"),
        cli::Action::Quit => app.exit(0),
        cli::Action::Ask => hotkey::on_ask(app),
        cli::Action::Snip => log::info!("snip is not available yet (spec 10)"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch hands its arguments to this instance and exits (APP-2.2).
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let actions = cli::parse(argv.get(1..).unwrap_or_default());
            if actions.is_empty() {
                open_window(app, "settings"); // APP-2.4
            }
            for action in actions {
                handle(app, action);
            }
        }))
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).max_file_size(1_000_000).rotation_strategy(RotationStrategy::KeepSome(5)).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        hotkey::on_ask(app); // ask is the only hotkey registered so far
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![get_settings, set_settings, app_info, open_folder, open_setup])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let (settings, warning) = settings::load(&path);
            if let Some(w) = warning {
                log::warn!("{w}");
                app.dialog().message(w).title("Cursora").show(|_| {});
            }
            tray::create(app.handle(), &settings)?;
            sync_autostart(app.handle(), &settings);
            if let Err(e) = hotkey::apply(app.handle(), None, &settings) {
                log::warn!("{e}");
                app.dialog().message(e).title("Cursora").show(|_| {}); // SEL-1.3
            }
            let setup_done = settings.setup_done;
            app.manage(AppState { settings: Mutex::new(settings), path });
            if !setup_done {
                open_window(app.handle(), "setup");
            }
            let args: Vec<String> = std::env::args().skip(1).collect();
            for action in cli::parse(&args) {
                handle(app.handle(), action);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Cursora")
        .run(|_app, event| {
            // Closing the last window must not quit: Cursora lives in the tray (APP-1.1).
            if let RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}
