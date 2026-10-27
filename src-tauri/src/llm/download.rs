//! One-click model download (spec 15): a pinned model file, fetched with resume and checked before use.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: &'static str,
    pub label: &'static str,
    pub note: &'static str,
    pub file: &'static str,
    pub size: u64,
    #[serde(skip)]
    pub url: &'static str,
    #[serde(skip)]
    pub sha256: &'static str,
}

/// MDL-1.3: Apache 2.0 instruction models, Q4_K_M, at fixed revisions (requirements, default 1). Sizes and hashes from the
/// Hugging Face API, 2026-10-26.
pub const MODELS: [Choice; 3] = [
    Choice {
        id: "small",
        label: "Small",
        note: "Fastest, for computers with less memory",
        file: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        size: 1_117_320_736,
        url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/91cad51170dc346986eccefdc2dd33a9da36ead9/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        sha256: "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
    },
    Choice {
        id: "medium",
        label: "Medium",
        note: "A good balance of speed and answers",
        file: "Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        size: 2_497_281_120,
        url: "https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/a06e946bb6b655725eafa393f4a9745d460374c9/Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        sha256: "3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597",
    },
    Choice {
        id: "large",
        label: "Large",
        note: "The best answers, slower",
        file: "Qwen2.5-7B-Instruct-Q4_K_M.gguf",
        size: 4_683_074_240,
        url: "https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/8911e8a47f92bac19d6f5c64a2e2095bd2f7d031/Qwen2.5-7B-Instruct-Q4_K_M.gguf",
        sha256: "65b8fcd92af6b4fefa935c625d1ac27ea29dcb6ee14589c55a8f115ceaaa1423",
    },
];

/// VIN-2.2: whisper.cpp's multilingual speech models (MIT license, OpenAI's Whisper weights), at a fixed revision. Sizes
/// and hashes from the Hugging Face API, 2026-10-28.
pub const SPEECH: [Choice; 2] = [
    Choice {
        id: "fast",
        label: "Fast",
        note: "Quick, best for English",
        file: "ggml-base-q5_1.bin",
        size: 59_707_625,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base-q5_1.bin",
        sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    },
    Choice {
        id: "accurate",
        label: "Accurate",
        note: "Better for Hindi and other Indian languages, slower",
        file: "ggml-small-q5_1.bin",
        size: 190_085_487,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small-q5_1.bin",
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    },
];

const UNREACHABLE: &str = "Couldn't reach the download server. Check the internet connection and try again.";
const STALL: Duration = Duration::from_secs(60);

/// MDL-1.2: rounded to whole GB, since an 8 GB computer reports a little less than 8 GB.
pub fn recommended(total_bytes: Option<u64>) -> &'static str {
    match total_bytes.map(|b| (b + (1 << 29)) >> 30) {
        Some(0..8) => "small",
        _ => "medium",
    }
}

#[cfg(windows)]
pub fn total_ram() -> Option<u64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m) }.ok()?;
    Some(m.ullTotalPhys)
}

#[cfg(not(windows))]
pub fn total_ram() -> Option<u64> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = info.lines().find_map(|l| l.strip_prefix("MemTotal:"))?.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb * 1024)
}

fn disk(e: std::io::Error) -> String {
    format!("Couldn't save the model: {e}")
}

async fn sha256_of(path: &Path) -> Result<String, String> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut h = Sha256::new();
        std::io::copy(&mut std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(&path)?), &mut h)?;
        Ok(format!("{:x}", h.finalize()))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(disk)
}

/// Downloads `url` to `path` through `<path>.part`, continuing a part left by a cancel or an error (MDL-1.5, 1.7), and keeps
/// the file only if its size and SHA-256 match (MDL-1.3). A complete, matching file already at `path` is used as it is (MDL-1.6).
/// `progress` gets the bytes done, at most 4 times a second. On cancel it returns `Err` with the part kept.
pub async fn fetch(url: &str, size: u64, sha256: &str, path: &Path, cancel: &AtomicBool, progress: &mut (dyn FnMut(u64) + Send)) -> Result<(), String> {
    if path.metadata().is_ok_and(|m| m.len() == size) && sha256_of(path).await? == sha256 {
        return Ok(());
    }
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let mut have = part.metadata().map_or(0, |m| m.len());
    if have > size {
        have = 0;
    }
    if have < size {
        // MDL-2.1: the file's address, Cursora's user agent and the range; redirects lead to Hugging Face's CDN.
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .user_agent(concat!("Cursora/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?;
        let mut req = client.get(url);
        if have > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
        }
        let mut resp = req.send().await.map_err(|_| UNREACHABLE.to_string())?;
        let append = match resp.status().as_u16() {
            206 => true,
            200 => false, // the server sent the whole file: start again
            s => return Err(format!("The download server answered {s}. Try again later.")),
        };
        let mut file = std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(&part).map_err(disk)?;
        let (mut done, mut shown) = (if append { have } else { 0 }, Instant::now());
        // shortcut: blocking writes of each chunk on the async thread; fine for one download at a time.
        loop {
            // Cancel works at once, even while the server sends nothing.
            let next = tokio::select! {
                next = tokio::time::timeout(STALL, resp.chunk()) => next,
                _ = async { while !cancel.load(Ordering::SeqCst) { tokio::time::sleep(Duration::from_millis(100)).await } } => return Err("Cancelled.".into()),
            };
            let next = next.map_err(|_| "The download stopped: no data for a minute. Download again to continue.".to_string())?;
            let Some(chunk) = next.map_err(|_| UNREACHABLE.to_string())? else { break };
            file.write_all(&chunk).map_err(disk)?;
            done += chunk.len() as u64;
            if shown.elapsed() >= Duration::from_millis(250) {
                progress(done);
                shown = Instant::now();
            }
        }
        progress(done);
    }
    let got = part.metadata().map_or(0, |m| m.len());
    if got < size {
        return Err("The download was interrupted. Download again to continue.".into());
    }
    if got > size || sha256_of(&part).await? != sha256 {
        let _ = std::fs::remove_file(&part);
        return Err("The downloaded file didn't match, so it was deleted. Try again.".into());
    }
    std::fs::rename(&part, path).map_err(disk)
}

/// One download at a time (MDL-1.4), and its Cancel.
#[derive(Default)]
pub struct Download {
    running: AtomicBool,
    cancel: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    models: &'static [Choice],
    speech: &'static [Choice],
    recommended: &'static str,
    folder: String,
}

/// MDL-1.1, 1.2.
#[tauri::command]
pub fn model_choices(app: AppHandle) -> Choices {
    let folder = app.path().app_local_data_dir().map(|d| d.join("models").to_string_lossy().into_owned()).unwrap_or_default();
    Choices { models: &MODELS, speech: &SPEECH, recommended: recommended(total_ram()), folder }
}

#[derive(Serialize, Clone)]
struct Progress {
    done: u64,
    total: u64,
}

/// The model's path once it's ready; `None` when cancelled. Progress goes to every window as `model-progress` (MDL-1.4).
#[tauri::command]
pub async fn model_download(app: AppHandle, id: String, folder: String) -> Result<Option<String>, String> {
    let m = MODELS.iter().chain(&SPEECH).find(|m| m.id == id).ok_or("There's no such model.")?;
    let state = app.state::<Download>();
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("A model is already downloading.".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    let path: PathBuf = Path::new(&folder).join(m.file);
    let result = match std::fs::create_dir_all(&folder) {
        Ok(()) => {
            let mut progress = |done| {
                let _ = app.emit("model-progress", Progress { done, total: m.size });
            };
            fetch(m.url, m.size, m.sha256, &path, &state.cancel, &mut progress).await
        }
        Err(e) => Err(disk(e)),
    };
    state.running.store(false, Ordering::SeqCst);
    match result {
        Ok(()) => Ok(Some(path.to_string_lossy().into_owned())),
        Err(_) if state.cancel.load(Ordering::SeqCst) => Ok(None),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn model_download_cancel(app: AppHandle) {
    app.state::<Download>().cancel.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::sync::{Arc, Mutex};

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cursora-download-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn block<T>(f: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    /// An HTTP server on 127.0.0.1 for one file: `status` for every request, else the file, from the asked offset when
    /// `ranges` is on. 16 KB at a time, `pace_ms` apart. Returns the file's URL and each request's head.
    fn serve(body: Vec<u8>, ranges: bool, status: u16, pace_ms: u64) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}/model.gguf", listener.local_addr().unwrap().port());
        let heads = Arc::new(Mutex::new(vec![]));
        let seen = heads.clone();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut head = String::new();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                while reader.read_line(&mut head).unwrap_or(0) > 2 {}
                let from = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("range: bytes=")?.trim_end_matches('-').parse::<usize>().ok());
                seen.lock().unwrap().push(head);
                let start = if ranges { from.unwrap_or(0) } else { 0 };
                let reply = match status {
                    200 if start > 0 => format!("HTTP/1.1 206 Partial\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len() - start),
                    200 => format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()),
                    s => format!("HTTP/1.1 {s} No\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
                };
                let _ = stream.write_all(reply.as_bytes());
                if status == 200 {
                    for c in body[start..].chunks(16 * 1024) {
                        if stream.write_all(c).is_err() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(pace_ms));
                    }
                }
            }
        });
        (url, heads)
    }

    fn file() -> (Vec<u8>, String) {
        let body: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let sha = format!("{:x}", Sha256::digest(&body));
        (body, sha)
    }

    #[test]
    fn mdl_1_2_the_recommendation_follows_memory() {
        let gb = |n: u64| Some(n << 30);
        assert_eq!(recommended(gb(4)), "small");
        assert_eq!(recommended(gb(7)), "small");
        assert_eq!(recommended(Some((8 << 30) - (300 << 20))), "medium", "8 GB reported as 7.7 GB");
        assert_eq!(recommended(Some((16 << 30) - (300 << 20))), "medium");
        assert_eq!(recommended(gb(64)), "medium", "Large is never recommended");
        assert_eq!(recommended(None), "medium");
        assert!(total_ram().is_some_and(|b| b > 1 << 30));
    }

    #[test]
    fn mdl_1_3_the_models_are_pinned_and_a_wrong_file_is_deleted() {
        for m in &MODELS {
            let commit = m.url.strip_prefix("https://huggingface.co/").and_then(|r| r.split("/resolve/").nth(1)).and_then(|r| r.split('/').next()).unwrap_or_default();
            assert!(commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()), "{}: a fixed revision on huggingface.co", m.id);
            assert!(m.url.ends_with(&format!("/{}", m.file)) && m.file.ends_with(".gguf"), "{}", m.id);
            assert!(m.sha256.len() == 64 && m.sha256.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')), "{}", m.id);
            assert!(m.size > 500 << 20, "{}", m.id);
        }
        let (body, _) = file();
        let (url, _) = serve(body.clone(), true, 200, 0);
        let d = temp_dir("wrong");
        let path = d.join("model.gguf");
        let got = block(fetch(&url, body.len() as u64, &"0".repeat(64), &path, &AtomicBool::new(false), &mut |_| {}));
        assert_eq!(got, Err("The downloaded file didn't match, so it was deleted. Try again.".into()));
        assert!(!path.exists() && !path.with_extension("gguf.part").exists());
    }

    #[test]
    fn vin_2_2_the_speech_models_are_pinned_and_a_wrong_file_is_deleted() {
        for m in &SPEECH {
            let rest = m.url.strip_prefix("https://huggingface.co/ggerganov/whisper.cpp/resolve/").unwrap_or_default();
            let commit = rest.split('/').next().unwrap_or_default();
            assert!(commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()), "{}: a fixed revision of whisper.cpp's models", m.id);
            assert!(m.url.ends_with(&format!("/{}", m.file)) && m.file.starts_with("ggml-") && m.file.ends_with(".bin"), "{}", m.id);
            assert!(m.sha256.len() == 64 && m.sha256.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')), "{}", m.id);
            assert!(m.size > 30 << 20, "{}", m.id);
        }
        let (body, _) = file();
        let (url, _) = serve(body.clone(), true, 200, 0);
        let d = temp_dir("wrong-speech");
        let path = d.join("ggml-model.bin");
        let got = block(fetch(&url, body.len() as u64, &"0".repeat(64), &path, &AtomicBool::new(false), &mut |_| {}));
        assert_eq!(got, Err("The downloaded file didn't match, so it was deleted. Try again.".into()));
        assert!(!path.exists() && !d.join("ggml-model.bin.part").exists());
    }

    #[test]
    fn mdl_1_5_cancel_keeps_the_part_and_the_next_download_continues_it() {
        let (body, sha) = file();
        let (url, heads) = serve(body.clone(), true, 200, 20);
        let d = temp_dir("resume");
        let (path, part) = (d.join("model.gguf"), d.join("model.gguf.part"));
        let cancel = Arc::new(AtomicBool::new(false));
        let (stop, started) = (cancel.clone(), part.clone());
        // Cancel once data has arrived: a fixed delay raced the client's start when all tests run at once (issue 53).
        std::thread::spawn(move || {
            while started.metadata().map_or(0, |m| m.len()) == 0 {
                std::thread::sleep(Duration::from_millis(5));
            }
            stop.store(true, Ordering::SeqCst);
        });
        assert_eq!(block(fetch(&url, body.len() as u64, &sha, &path, &cancel, &mut |_| {})), Err("Cancelled.".into()));
        let kept = part.metadata().unwrap().len();
        assert!(kept > 0 && kept < body.len() as u64, "a part: {kept}");
        cancel.store(false, Ordering::SeqCst);
        let mut last = 0;
        assert_eq!(block(fetch(&url, body.len() as u64, &sha, &path, &cancel, &mut |done| last = done)), Ok(()));
        assert_eq!((std::fs::read(&path).unwrap() == body, part.exists(), last), (true, false, body.len() as u64));
        assert!(heads.lock().unwrap()[1].to_ascii_lowercase().contains(&format!("range: bytes={kept}-")), "only the rest was asked for");

        // A server that ignores the range sends the whole file, and the download starts again.
        let (url, _) = serve(body.clone(), false, 200, 0);
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&part, &body[..1000]).unwrap();
        assert_eq!(block(fetch(&url, body.len() as u64, &sha, &path, &cancel, &mut |_| {})), Ok(()));
        assert!(std::fs::read(&path).unwrap() == body);
    }

    #[test]
    fn mdl_1_5_cancel_works_while_the_server_sends_nothing() {
        let (body, sha) = file();
        let (url, _) = serve(body.clone(), true, 200, 5000); // one piece, then 5 s of silence
        let d = temp_dir("stalled");
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            stop.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();
        assert_eq!(block(fetch(&url, body.len() as u64, &sha, &d.join("model.gguf"), &cancel, &mut |_| {})), Err("Cancelled.".into()));
        assert!(started.elapsed() < Duration::from_secs(2), "took {:?}", started.elapsed());
    }

    #[test]
    fn mdl_1_6_a_complete_file_is_used_without_downloading() {
        let (body, sha) = file();
        let d = temp_dir("complete");
        let path = d.join("model.gguf");
        std::fs::write(&path, &body).unwrap();
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // nothing listens once dropped
        let url = format!("http://127.0.0.1:{closed}/model.gguf");
        assert_eq!(block(fetch(&url, body.len() as u64, &sha, &path, &AtomicBool::new(false), &mut |_| {})), Ok(()));
    }

    #[test]
    fn mdl_1_7_a_failed_download_says_why_and_keeps_the_part() {
        let (body, sha) = file();
        let d = temp_dir("failed");
        let (path, part) = (d.join("model.gguf"), d.join("model.gguf.part"));
        std::fs::write(&part, &body[..1000]).unwrap();
        let (url, _) = serve(body.clone(), true, 404, 0);
        let got = block(fetch(&url, body.len() as u64, &sha, &path, &AtomicBool::new(false), &mut |_| {}));
        assert_eq!(got, Err("The download server answered 404. Try again later.".into()));
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let got = block(fetch(&format!("http://127.0.0.1:{closed}/m.gguf"), body.len() as u64, &sha, &path, &AtomicBool::new(false), &mut |_| {}));
        assert_eq!(got, Err(UNREACHABLE.into()));
        assert_eq!(part.metadata().unwrap().len(), 1000);
    }

    #[test]
    fn mdl_2_1_only_the_file_is_requested() {
        let (body, sha) = file();
        let (url, heads) = serve(body.clone(), true, 200, 0);
        let d = temp_dir("request");
        std::fs::write(d.join("model.gguf.part"), &body[..10]).unwrap();
        assert_eq!(block(fetch(&url, body.len() as u64, &sha, &d.join("model.gguf"), &AtomicBool::new(false), &mut |_| {})), Ok(()));
        let heads = heads.lock().unwrap();
        let head = heads[0].to_ascii_lowercase();
        assert_eq!(heads.len(), 1);
        assert!(head.starts_with("get /model.gguf http/1.1\r\n"), "{head}");
        let mut names: Vec<&str> = head.lines().skip(1).filter_map(|l| l.split(':').next()).filter(|n| !n.is_empty()).collect();
        names.sort();
        assert_eq!(names, ["accept", "host", "range", "user-agent"], "no cookies, keys or content");
        assert!(head.contains(concat!("user-agent: cursora/", env!("CARGO_PKG_VERSION"))));
    }
}
