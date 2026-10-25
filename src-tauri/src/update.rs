//! Auto-update (spec 11): a daily check of the releases repo, the tray item and About, and installing
//! through Tauri's updater plugin, which checks the signature first (UPD-2.3).
use crate::{chat, tray, AppState};
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

const RELEASES: &str = "https://github.com/priyansh1210/cursora-releases/releases";

#[derive(Serialize, Clone, Default, PartialEq, Debug)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
    #[default]
    Unknown,
    UpToDate,
    /// `installable`: false on Linux outside an AppImage, where Update opens the release page (UPD-2.6).
    #[serde(rename_all = "camelCase")]
    Available { version: String, notes: String, installable: bool },
    Failed { reason: String },
    Downloading { done: u64, total: Option<u64> },
    /// Update was chosen while an answer is still coming in (UPD-2.4).
    Waiting,
}

/// The last status, and the plugin's update from the last check, so installing doesn't fetch it again.
#[derive(Default)]
pub struct UpdateState(Mutex<(Status, Option<Update>)>);

/// Plain checks on top of the plugin's, which already drops versions that aren't newer (UPD-1.6, UPD-2.5).
fn offer(version: &str, skipped: Option<&str>) -> bool {
    !version.contains('-') && skipped != Some(version)
}

/// What the person reads in About; the plugin's own words go to the log.
fn plain(e: &tauri_plugin_updater::Error) -> &'static str {
    use tauri_plugin_updater::Error::*;
    match e {
        ReleaseNotFound | Reqwest(_) | Network(_) => "Couldn't reach the update server. Check your internet connection, then try again.",
        Minisign(_) | Base64(_) | SignatureUtf8(_) | SignedVersionMismatch { .. } | MissingSignedVersion => {
            "The update's signature didn't match Cursora's, so it wasn't installed."
        }
        AuthenticationFailed => "The update needs administrator permission, which wasn't given.",
        _ => "Something went wrong with the update. Try again later.",
    }
}

fn installable(_app: &AppHandle) -> bool {
    #[cfg(target_os = "linux")]
    return _app.env().appimage.is_some();
    #[cfg(not(target_os = "linux"))]
    true
}

/// The version the tray offers, if any (UPD-2.1). None before setup has created the state.
pub fn offered(app: &AppHandle) -> Option<String> {
    match &app.try_state::<UpdateState>()?.0.lock().unwrap().0 {
        Status::Available { version, .. } => Some(version.clone()),
        _ => None,
    }
}

fn set(app: &AppHandle, status: Status) {
    let before = offered(app);
    app.state::<UpdateState>().0.lock().unwrap().0 = status.clone();
    if offered(app) != before {
        // A copy: the tray waits for the main thread, which may want the settings lock (R35).
        let s = app.state::<AppState>().settings.lock().unwrap().clone();
        tray::refresh(app, &s);
    }
    let _ = app.emit_to("settings", "update-status", status);
}

/// 30 s after start, so the start and the first hotkey aren't slowed, then daily (UPD-1.1).
/// Never before setup is done: its last step is where the check can be turned off first (UPD-1.5).
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            let (setup_done, on) = {
                let state = app.state::<AppState>();
                let s = state.settings.lock().unwrap();
                (s.setup_done, s.updates.check)
            };
            if !setup_done {
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
            if on {
                check(&app).await;
            }
            tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
        }
    });
}

async fn check(app: &AppHandle) -> Status {
    let skipped = app.state::<AppState>().settings.lock().unwrap().updates.skipped.clone();
    let found = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(e) => Err(e),
    };
    let (status, update) = match found {
        Ok(Some(u)) if offer(&u.version, skipped.as_deref()) => {
            let status = Status::Available { version: u.version.clone(), notes: u.body.clone().unwrap_or_default(), installable: installable(app) };
            (status, Some(u))
        }
        Ok(_) => (Status::UpToDate, None),
        // Only About says so; the next check is the retry (UPD-1.4).
        Err(e) => {
            log::info!("update check: {e}");
            (Status::Failed { reason: plain(&e).into() }, None)
        }
    };
    app.state::<UpdateState>().0.lock().unwrap().1 = update;
    set(app, status.clone());
    status
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> Status {
    app.state::<UpdateState>().0.lock().unwrap().0.clone()
}

/// About's button: checks now, whatever the setting (UPD-1.3).
#[tauri::command]
pub async fn update_check(app: AppHandle) -> Status {
    check(&app).await
}

#[tauri::command]
pub fn update_skip(app: AppHandle, version: String) -> Result<(), String> {
    let mut s = app.state::<AppState>().settings.lock().unwrap().clone();
    s.updates.skipped = Some(version);
    crate::update_settings(&app, s)?;
    set(&app, Status::UpToDate);
    Ok(())
}

/// Tray item and About's button (UPD-2.2, 2.4, 2.6).
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    let Some(update) = app.state::<UpdateState>().0.lock().unwrap().1.clone() else {
        return Err("There's no update to install. Check for updates first.".into());
    };
    if !installable(&app) {
        return app.opener().open_url(format!("{RELEASES}/tag/v{}", update.version), None::<&str>).map_err(|e| e.to_string());
    }
    if chat::answering(&app) {
        set(&app, Status::Waiting);
        while chat::answering(&app) {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    let (mut done, mut shown) = (0u64, 0u64);
    let result = update
        .download_and_install(
            |chunk, total| {
                done += chunk as u64;
                if done - shown >= 1 << 20 {
                    shown = done;
                    set(&app, Status::Downloading { done, total });
                }
            },
            || {},
        )
        .await;
    match result {
        // On Windows the plugin has already exited Cursora for the installer, which starts it again.
        Ok(()) => app.restart(),
        Err(e) => {
            log::warn!("update: {e}");
            let reason = plain(&e).to_string();
            set(&app, Status::Failed { reason: reason.clone() });
            Err(reason)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::offer;
    use serde_json::{json, Value};

    fn updater_config() -> Value {
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        conf["plugins"]["updater"].clone()
    }

    #[test]
    fn upd_1_2_the_only_request_is_the_release_file_over_https() {
        let c = updater_config();
        assert_eq!(c["endpoints"], json!(["https://github.com/priyansh1210/cursora-releases/releases/latest/download/latest.json"]));
        assert!(c["pubkey"].as_str().is_some_and(|k| !k.is_empty()), "updates are verified (UPD-2.3)");
        assert_eq!(c["requireSignedVersion"], true, "a signature names its version, so an old release can't pose as a new one");
    }

    // tauri-plugin-updater 2.14's own throwaway key pair, and its signature of the payload "test" for version 2.0.0.
    const OTHER_KEY: &str = "untrusted comment: minisign public key: A49EF4746FA6E6D7\nRWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe";
    const SIGNATURE: &str = "untrusted comment: signature from tauri secret key\nRUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\ntrusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\nDDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==";

    /// The plugin's own check (`verify_signature` in tauri-plugin-updater 2.14): the payload "test" against `pubkey`,
    /// which is base64 of the key's text, as in tauri.conf.json.
    fn verifies(pubkey: &str) -> bool {
        use base64::Engine;
        use minisign_verify::{PublicKey, Signature};
        let text = |b: &[u8]| String::from_utf8(b.to_vec()).unwrap_or_default();
        let Ok(key) = base64::engine::general_purpose::STANDARD.decode(pubkey) else { return false };
        let (Ok(key), Ok(signature)) = (PublicKey::decode(&text(&key)), Signature::decode(SIGNATURE)) else { return false };
        key.verify(b"test", &signature, true).is_ok()
    }

    #[test]
    fn upd_2_3_an_update_signed_with_another_key_is_refused() {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        assert!(verifies(&b64.encode(OTHER_KEY)), "the signature is good for the key that made it");
        let ours = updater_config()["pubkey"].as_str().unwrap_or_default().to_string();
        // A real key, so the refusal below isn't just a key that fails to load.
        let text = b64.decode(&ours).ok().and_then(|k| String::from_utf8(k).ok()).unwrap_or_default();
        assert!(minisign_verify::PublicKey::decode(&text).is_ok(), "Cursora's pubkey is a minisign public key");
        assert!(!verifies(&ours), "Cursora's key refuses it");
    }

    #[test]
    fn upd_1_6_pre_releases_and_the_skipped_version_are_never_offered() {
        assert!(offer("0.2.0", None));
        assert!(!offer("0.2.0-rc.1", None), "a pre-release");
        assert!(!offer("0.2.0", Some("0.2.0")), "skipped");
        assert!(offer("0.3.0", Some("0.2.0")), "a later one is offered again");
    }
}
