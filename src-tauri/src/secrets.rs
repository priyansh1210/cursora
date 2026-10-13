//! API keys in the OS keychain, under service "cursora" and account = provider id (LLM-6.1).
//! Keys never reach the webview, the logs or the settings file.
use keyring_core::{Entry, Error};
use serde::Serialize;

const SERVICE: &str = "cursora";

/// What the webview may know about a key (LLM-6.2).
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub has_key: bool,
    /// The last 4 characters.
    pub key_hint: Option<String>,
}

pub fn set(provider: &str, key: &str) -> Result<(), String> {
    entry(provider)?.set_password(key.trim()).map_err(|e| explain(&e))
}

/// The key, or `None` if none is stored.
pub fn get(provider: &str) -> Result<Option<String>, String> {
    match entry(provider)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(explain(&e)),
    }
}

pub fn delete(provider: &str) {
    let result = entry(provider).and_then(|e| match e.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(e) => Err(explain(&e)),
    });
    if let Err(e) = result {
        log::warn!("keychain: couldn't delete a key: {e}");
    }
}

pub fn status(provider: &str) -> KeyStatus {
    let key = get(provider).ok().flatten();
    KeyStatus { has_key: key.is_some(), key_hint: key.map(|k| k.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect()) }
}

/// Deletes the keys of providers that are gone, whichever screen removed them.
pub fn forget_removed<'a>(old: impl IntoIterator<Item = &'a str>, new: &[&str]) {
    for id in old {
        if !new.contains(&id) {
            delete(id);
        }
    }
}

fn entry(provider: &str) -> Result<Entry, String> {
    ensure_store()?;
    Entry::new(SERVICE, provider).map_err(|e| explain(&e))
}

/// Picks the OS keychain on first use, and again after a failure, so the user can start one and retry (LLM-6.3).
fn ensure_store() -> Result<(), String> {
    if keyring_core::get_default_store().is_some() {
        return Ok(());
    }
    // Tests never touch the real keychain.
    #[cfg(test)]
    let store = keyring_core::mock::Store::new();
    #[cfg(all(not(test), windows))]
    let store = windows_native_keyring_store::Store::new();
    #[cfg(all(not(test), not(windows)))]
    let store = zbus_secret_service_keyring_store::Store::new();
    keyring_core::set_default_store(store.map_err(|e| explain(&e))?);
    Ok(())
}

/// There is no plaintext fallback: without a keychain, keys can't be saved (LLM-6.3).
/// On Linux the user gets what to do; the error itself ("Platform failure: zbus error: …") goes to the log.
fn explain(e: &Error) -> String {
    if cfg!(windows) {
        format!("Windows Credential Manager refused the key: {e}")
    } else {
        log::warn!("keychain: {e}");
        "Cursora keeps API keys in the system keyring, and none is available. Install and unlock GNOME Keyring or KWallet, then try again.".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_6_1_keys_round_trip_under_service_cursora_and_the_provider_id() {
        set("p-61", "  sk-test-61  ").unwrap();
        assert_eq!(get("p-61").unwrap().as_deref(), Some("sk-test-61"));
        assert_eq!(Entry::new("cursora", "p-61").unwrap().get_password().unwrap(), "sk-test-61");
        delete("p-61");
        assert_eq!(get("p-61").unwrap(), None);
        assert_eq!(get("p-never-set").unwrap(), None);
    }

    #[test]
    fn llm_6_1_removing_a_provider_deletes_its_key() {
        set("p-a", "key-a").unwrap();
        set("p-b", "key-b").unwrap();
        forget_removed(["p-a", "p-b"], &["p-b"]);
        assert_eq!(get("p-a").unwrap(), None);
        assert_eq!(get("p-b").unwrap().as_deref(), Some("key-b"));
    }

    #[test]
    fn llm_6_2_the_webview_sees_only_has_key_and_the_last_4_characters() {
        set("p-62", "sk-ant-secret-9f3K").unwrap();
        let json = serde_json::to_string(&status("p-62")).unwrap();
        assert_eq!(json, r#"{"hasKey":true,"keyHint":"9f3K"}"#);
        assert_eq!(status("p-62-none"), KeyStatus { has_key: false, key_hint: None });
    }

    #[cfg(not(windows))]
    #[test]
    fn llm_6_3_the_no_keyring_message_says_what_to_do_without_the_raw_error() {
        let e = Error::PlatformFailure("zbus error: org.freedesktop.DBus.Error.ServiceUnknown".into());
        assert_eq!(explain(&e), "Cursora keeps API keys in the system keyring, and none is available. Install and unlock GNOME Keyring or KWallet, then try again.");
    }
}
