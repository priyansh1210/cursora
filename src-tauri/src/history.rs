//! Conversation history in SQLite (BUB-9). The file is created only by a save, so with saving off nothing reaches disk (BUB-9.3).
use crate::chat::Conversation;
use crate::settings::HistorySettings;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const DAY_MS: i64 = 86_400_000;
const PAGE: usize = 50;

pub struct History {
    path: PathBuf,
    conn: Mutex<Option<Connection>>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: i64,
    pub updated: i64,
    pub title: String,
    pub app: String,
    pub model: Option<String>,
}

impl History {
    pub fn new(path: PathBuf) -> Self {
        Self { path, conn: Mutex::new(None) }
    }

    /// Saves (or rewrites) the conversation and prunes old ones, only while saving is on (BUB-9.1, 9.3, 9.4).
    pub fn record(&self, cfg: &HistorySettings, conv: &Conversation, now: i64) -> Result<(), String> {
        if !cfg.save {
            return Ok(());
        }
        let title: String = conv.turns.first().map_or("", |t| t.question.as_str()).chars().take(120).collect();
        let data = serde_json::to_string(conv).map_err(|e| e.to_string())?;
        let search = searchable(conv);
        self.with(true, |c| {
            c.execute(
                "INSERT OR REPLACE INTO conversations (id, updated, title, app, model, data, search) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![conv.id, now, title, conv.app.name, conv.model.as_ref().map(|m| &m.name), data, search],
            )?;
            prune(c, cfg, now)
        })
        .map(|_| ())
    }

    /// Newest first; `query` matches questions, answers and context, ignoring case (BUB-9.2).
    pub fn list(&self, query: &str, offset: usize) -> Result<Vec<Summary>, String> {
        let query = query.to_lowercase();
        self.with(false, |c| {
            let mut stmt = c.prepare(
                "SELECT id, updated, title, app, model FROM conversations WHERE instr(search, ?1) > 0 ORDER BY updated DESC LIMIT ?2 OFFSET ?3",
            )?;
            let rows = stmt.query_map(params![query, PAGE as i64, offset as i64], |r| {
                Ok(Summary { id: r.get(0)?, updated: r.get(1)?, title: r.get(2)?, app: r.get(3)?, model: r.get(4)? })
            })?;
            rows.collect()
        })
        .map(Option::unwrap_or_default)
    }

    pub fn get(&self, id: i64) -> Result<Option<Conversation>, String> {
        let data: Option<String> = self
            .with(false, |c| c.query_row("SELECT data FROM conversations WHERE id = ?1", [id], |r| r.get(0)).optional())?
            .flatten();
        data.map(|d| serde_json::from_str(&d).map_err(|e| e.to_string())).transpose()
    }

    pub fn delete(&self, id: i64) -> Result<(), String> {
        self.with(false, |c| c.execute("DELETE FROM conversations WHERE id = ?1", [id])).map(|_| ())
    }

    pub fn clear(&self) -> Result<(), String> {
        self.with(false, |c| c.execute("DELETE FROM conversations", [])).map(|_| ())
    }

    /// Startup pruning (BUB-9.4); reads only, never creates the file.
    pub fn prune(&self, cfg: &HistorySettings, now: i64) -> Result<(), String> {
        self.with(false, |c| prune(c, cfg, now)).map(|_| ())
    }

    /// Runs `f` on the open database. Without `create`, a missing file means "nothing stored" and `f` doesn't run.
    fn with<T>(&self, create: bool, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<Option<T>, String> {
        let mut conn = self.conn.lock().unwrap();
        if conn.is_none() {
            if !create && !self.path.exists() {
                return Ok(None);
            }
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let c = Connection::open(&self.path).map_err(|e| e.to_string())?;
            c.execute_batch(
                "CREATE TABLE IF NOT EXISTS conversations (
                    id INTEGER PRIMARY KEY, updated INTEGER NOT NULL, title TEXT NOT NULL, app TEXT NOT NULL,
                    model TEXT, data TEXT NOT NULL, search TEXT NOT NULL);
                 CREATE INDEX IF NOT EXISTS conversations_updated ON conversations(updated);",
            )
            .map_err(|e| e.to_string())?;
            *conn = Some(c);
        }
        f(conn.as_ref().unwrap()).map(Some).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn history_list(app: AppHandle, query: String, offset: usize) -> Result<Vec<Summary>, String> {
    app.state::<History>().list(&query, offset)
}

/// Reopens a conversation in the expanded window (BUB-9.2). Async because it may create a window (wry#583).
#[tauri::command(async)]
pub fn history_open(app: AppHandle, id: i64) -> Result<(), String> {
    let conv = app.state::<History>().get(id)?.ok_or("This conversation no longer exists.")?;
    crate::chat::open_in_chat(&app, conv)
}

#[tauri::command]
pub fn history_delete(app: AppHandle, id: i64) -> Result<(), String> {
    app.state::<History>().delete(id)
}

#[tauri::command]
pub fn history_clear(app: AppHandle) -> Result<(), String> {
    app.state::<History>().clear()
}

fn prune(c: &Connection, cfg: &HistorySettings, now: i64) -> rusqlite::Result<usize> {
    match cfg.retention_days {
        Some(days) => c.execute("DELETE FROM conversations WHERE updated < ?1", [now - i64::from(days) * DAY_MS]),
        None => Ok(0),
    }
}

fn searchable(conv: &Conversation) -> String {
    let mut s = String::new();
    for t in &conv.turns {
        s += &t.question;
        s.push('\n');
        s += &t.answer;
        s.push('\n');
    }
    if let crate::capture::Content::Text { text, .. } = &conv.content {
        s += text;
    }
    s.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{Content, SourceApp};
    use crate::chat::{Turn, TurnStatus};

    fn temp_db(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cursora-history-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("history.sqlite")
    }

    fn conv(id: i64, question: &str, answer: &str, status: TurnStatus) -> Conversation {
        Conversation {
            id,
            app: SourceApp { name: "notepad".into(), ..Default::default() },
            content: Content::Text { text: "The Selected Text".into(), truncated: false },
            notice: None,
            model: None,
            persona: Some("Cursora".into()),
            turns: vec![Turn { question: question.into(), answer: answer.into(), status, error: None, asked: id, answered: Some(id + 5) }],
            dropped: 0,
            unavailable: None,
        }
    }

    #[test]
    fn bub_9_1_conversations_round_trip_and_are_searchable() {
        let h = History::new(temp_db("roundtrip"));
        let on = HistorySettings::default();
        let first = conv(1_000, "What is this?", "A test.", TurnStatus::Done);
        let cancelled = conv(2_000, "Explain more", "It was half", TurnStatus::Cancelled); // BUB-2.6: partial answers are kept
        h.record(&on, &first, 10_000).unwrap();
        h.record(&on, &cancelled, 20_000).unwrap();

        let all = h.list("", 0).unwrap();
        assert_eq!(all.iter().map(|s| s.id).collect::<Vec<_>>(), [2_000, 1_000], "newest first");
        assert_eq!(all[1].title, "What is this?");
        assert_eq!(all[1].app, "notepad");
        assert_eq!(h.list("SELECTED text", 0).unwrap().len(), 2, "search covers the context, ignoring case");
        assert_eq!(h.list("half", 0).unwrap()[0].id, 2_000);
        assert_eq!(h.get(2_000).unwrap(), Some(cancelled));

        h.delete(1_000).unwrap();
        assert_eq!(h.list("", 0).unwrap().len(), 1);
        h.clear().unwrap();
        assert!(h.list("", 0).unwrap().is_empty());
    }

    #[test]
    fn bub_9_3_with_saving_off_nothing_is_written() {
        let path = temp_db("off");
        let h = History::new(path.clone());
        let off = HistorySettings { save: false, ..Default::default() };
        h.record(&off, &conv(1, "q", "a", TurnStatus::Done), 10).unwrap();
        assert_eq!(h.list("", 0).unwrap(), vec![]);
        assert_eq!(h.get(1).unwrap(), None);
        h.prune(&off, 10).unwrap();
        assert!(!path.exists(), "reads and pruning must not create the file");
    }

    #[test]
    fn bub_9_4_conversations_past_retention_are_deleted() {
        let h = History::new(temp_db("prune"));
        let now = 1_000 * DAY_MS;
        let ninety = HistorySettings::default();
        h.record(&HistorySettings { retention_days: None, ..ninety.clone() }, &conv(1, "old", "", TurnStatus::Done), now - 91 * DAY_MS).unwrap();
        h.record(&HistorySettings { retention_days: None, ..ninety.clone() }, &conv(2, "recent", "", TurnStatus::Done), now - 89 * DAY_MS).unwrap();

        h.prune(&HistorySettings { retention_days: None, ..ninety.clone() }, now).unwrap();
        assert_eq!(h.list("", 0).unwrap().len(), 2, "forever keeps everything");
        h.prune(&ninety, now).unwrap();
        assert_eq!(h.list("", 0).unwrap().iter().map(|s| s.id).collect::<Vec<_>>(), [2]);
    }
}
