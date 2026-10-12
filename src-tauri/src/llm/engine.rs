//! The built-in engine: llama.cpp's llama-server, run as a child process on 127.0.0.1 (LLM-3).
use super::net::{self, Policy};
use super::{EngineSettings, Lifetime, Provider};
use crate::chat::{ChatError, ErrorKind};
use serde::Serialize;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// The context size the engine is started with when the provider sets none.
pub const DEFAULT_CONTEXT: u32 = 8192;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Status {
    Stopped,
    Starting,
    Running,
    Failed { reason: String },
}

#[derive(Default)]
pub struct Engine(Mutex<State>);

#[derive(Default)]
struct State {
    running: Option<Running>,
    status: Option<Status>,
    /// Answers in progress; the engine is never stopped for idleness while one runs.
    busy: usize,
}

type Files = (String, Option<String>, u32);

struct Running {
    child: Child,
    /// The model file, the vision projector and the context size; asking for anything else restarts it.
    started: Files,
    port: u16,
    last_used: Instant,
    /// The last 20 lines of its output, kept in memory only (they can contain file paths).
    output: Arc<Mutex<VecDeque<String>>>,
}

fn state(app: &AppHandle) -> MutexGuard<'_, State> {
    app.state::<Engine>().inner().0.lock().unwrap()
}

pub fn status(app: &AppHandle) -> Status {
    state(app).status.clone().unwrap_or(Status::Stopped)
}

fn set_status(app: &AppHandle, s: &mut State, status: Status) {
    s.status = Some(status.clone());
    let _ = app.emit("engine-status", status);
}

/// llama-server's arguments. Bound to 127.0.0.1 only (LLM-3.3).
pub fn args(e: &EngineSettings, context: u32, port: u16) -> Vec<String> {
    let mut a = vec!["-m".into(), e.gguf.clone()];
    if let Some(mmproj) = &e.mmproj {
        a.extend(["--mmproj".into(), mmproj.clone()]);
    }
    a.extend(["--host", "127.0.0.1", "--port", &port.to_string(), "-c", &context.to_string(), "--no-webui"].map(String::from));
    a
}

/// The bundled llama-server, or `src-tauri/llama/` in dev builds (`npm run fetch-llama`).
pub fn program(app: &AppHandle) -> Option<PathBuf> {
    let name = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };
    let bundled = app.path().resource_dir().ok().map(|d| d.join("llama").join(name));
    let dev = cfg!(debug_assertions).then(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("llama").join(name));
    [bundled, dev].into_iter().flatten().find(|p| p.exists())
}

/// Why the engine can't run, without starting it (the health check, LLM-9.1).
/// An earlier crash doesn't count: its reason was shown then (LLM-3.6), and the next question starts the engine again.
pub fn check(app: &AppHandle, p: &Provider) -> Result<(), String> {
    let e = p.engine.as_ref().ok_or("The built-in engine has no model file.")?;
    program(app).ok_or("The built-in engine isn't installed in this build of Cursora.")?;
    if !std::path::Path::new(&e.gguf).is_file() {
        return Err(format!("The model file is missing: {}", e.gguf));
    }
    Ok(())
}

/// Starts the engine for this provider if it isn't already running with the same files, and waits until it is ready (LLM-3.2).
/// Returns its OpenAI-compatible base URL. Stop works while the model loads.
pub fn ensure_running(app: &AppHandle, p: &Provider, policy: &Policy, cancel: &AtomicBool) -> Result<String, ChatError> {
    let fail = |m: String| net::error(ErrorKind::Server, m);
    let e = p.engine.clone().ok_or_else(|| fail("The built-in engine has no model file.".into()))?;
    let wanted: Files = (e.gguf.clone(), e.mmproj.clone(), p.context_size.unwrap_or(DEFAULT_CONTEXT));
    let port = {
        let mut s = state(app);
        let alive = s.running.as_mut().is_some_and(|r| r.started == wanted && matches!(r.child.try_wait(), Ok(None)));
        if !alive {
            stop_locked(app, &mut s, None);
            let program = program(app).ok_or_else(|| fail("The built-in engine isn't installed in this build of Cursora.".into()))?;
            if !std::path::Path::new(&e.gguf).is_file() {
                return Err(fail(format!("The model file is missing: {}", e.gguf)));
            }
            // shortcut: free-port race; retry once with a new port if "address already in use" shows up.
            let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port()).map_err(|e| fail(e.to_string()))?;
            let output = Arc::new(Mutex::new(VecDeque::new()));
            let child = spawn(app, program, args(&e, wanted.2, port), output.clone()).map_err(|err| fail(format!("Couldn't start the built-in engine: {err}")))?;
            log::info!("engine: started on port {port}");
            s.running = Some(Running { child, started: wanted, port, last_used: Instant::now(), output });
            set_status(app, &mut s, Status::Starting);
        }
        let r = s.running.as_mut().unwrap();
        r.last_used = Instant::now();
        r.port
    };
    wait_ready(app, port, policy, cancel)?;
    Ok(format!("http://127.0.0.1:{port}/v1"))
}

/// Polls `/health` every 250 ms until it answers 200 (503 while loading), the engine exits, the first-reply limit passes, or Stop.
fn wait_ready(app: &AppHandle, port: u16, policy: &Policy, cancel: &AtomicBool) -> Result<(), ChatError> {
    let deadline = Instant::now() + Duration::from_secs(policy.timeouts.first_reply_local_secs);
    let url = format!("http://127.0.0.1:{port}/health");
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(reason) = exited(app) {
            return Err(net::error(ErrorKind::Server, format!("The built-in engine stopped: {reason}")));
        }
        let req = net::Request { url: url.parse().unwrap(), headers: vec![], body: None };
        if tauri::async_runtime::block_on(net::json(req, policy, Duration::from_secs(2))).is_ok() {
            let mut s = state(app);
            if s.status != Some(Status::Running) {
                set_status(app, &mut s, Status::Running);
            }
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(net::error(ErrorKind::Network, "The built-in engine didn't finish loading the model in time."));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// If the engine has exited on its own: records and returns why (LLM-3.6).
pub fn exited(app: &AppHandle) -> Option<String> {
    let mut s = state(app);
    let r = s.running.as_mut()?;
    let code = r.child.try_wait().ok()??;
    let lines: Vec<String> = r.output.lock().unwrap().iter().cloned().collect();
    let reason = format!("it exited with {code}.\n{}", lines.join("\n")).trim().to_string();
    s.running = None;
    log::warn!("engine: exited with {code}");
    set_status(app, &mut s, Status::Failed { reason: reason.clone() });
    Some(reason)
}

/// A dropped connection can arrive just before the engine's exit does; waits up to `wait` for it.
pub fn exited_within(app: &AppHandle, wait: Duration) -> Option<String> {
    let deadline = Instant::now() + wait;
    loop {
        if let Some(reason) = exited(app) {
            return Some(reason);
        }
        if Instant::now() > deadline || state(app).running.is_none() {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Holds the engine in use until dropped; the idle time counts from the end of the answer (LLM-3.5).
pub struct Busy(AppHandle);

pub fn busy(app: &AppHandle) -> Busy {
    state(app).busy += 1;
    Busy(app.clone())
}

impl Drop for Busy {
    fn drop(&mut self) {
        let mut s = state(&self.0);
        s.busy -= 1;
        if let Some(r) = s.running.as_mut() {
            r.last_used = Instant::now();
        }
        drop(s);
        release(&self.0);
    }
}

pub fn stop(app: &AppHandle) {
    stop_locked(app, &mut state(app), None);
}

fn stop_locked(app: &AppHandle, s: &mut State, why: Option<&str>) {
    if let Some(mut r) = s.running.take() {
        let _ = r.child.kill();
        let _ = r.child.wait();
        log::info!("engine: stopped{}", why.map(|w| format!(" ({w})")).unwrap_or_default());
        set_status(app, s, Status::Stopped);
    }
}

/// "Always on": starts the default built-in model with Cursora, or when the user picks "Always on" (LLM-3.5).
/// On its own thread, since starting waits for the main thread.
pub fn preload(app: &AppHandle) {
    let s = app.state::<crate::AppState>().settings.lock().unwrap().clone();
    let Ok((p, _)) = super::availability(&s.models, s.local_only) else { return };
    if !p.engine.as_ref().is_some_and(|e| e.lifetime == Lifetime::Always) || p.kind != super::Kind::Builtin {
        return;
    }
    let (app, p, policy) = (app.clone(), p.clone(), Policy::of(&s));
    std::thread::spawn(move || {
        if let Err(e) = ensure_running(&app, &p, &policy, &AtomicBool::new(false)) {
            log::warn!("engine: preload failed ({:?})", e.kind); // the reason can hold the engine's output, kept in memory only
        }
    });
}

/// Whether the engine should stop now (LLM-3.5). Never during an answer.
fn should_stop(busy: usize, e: &EngineSettings, unused_for: Duration, conversation_open: bool) -> bool {
    busy == 0
        && match e.lifetime {
            Lifetime::Always => false,
            Lifetime::Idle => unused_for > Duration::from_secs(u64::from(e.idle_minutes) * 60),
            Lifetime::Answering => !conversation_open,
        }
}

/// Stops the engine if its provider's current lifetime says so; a removed provider means it is not needed.
fn stop_if_unused(app: &AppHandle, why: &str) {
    let settings = app.state::<crate::AppState>().settings.lock().unwrap().models.clone();
    let open = [crate::bubble::LABEL, crate::chat::CHAT_LABEL].iter().any(|l| app.get_webview_window(l).is_some_and(|w| w.is_visible().unwrap_or(false)));
    let mut s = state(app);
    let stop = s.running.as_ref().is_some_and(|r| {
        let current = settings.providers.iter().filter_map(|p| p.engine.as_ref()).find(|e| e.gguf == r.started.0);
        current.is_none_or(|e| should_stop(s.busy, e, r.last_used.elapsed(), open))
    });
    if stop {
        stop_locked(app, &mut s, Some(why));
    }
}

/// Checks once a minute ("Off when idle", LLM-3.5).
pub fn watch_idle(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        stop_if_unused(&app, "idle");
    });
}

/// The conversation windows closed or an answer ended ("Only while answering", LLM-3.5).
/// Checked a moment later, so expanding the bubble into a window doesn't unload the model.
pub fn release(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        stop_if_unused(&app, "not in use");
    });
}

/// "Stop the model" in the tray menu and the Models tab (LLM-3.5).
pub fn stop_by_hand(app: &AppHandle) {
    stop_locked(app, &mut state(app), Some("by hand"));
}

/// Started on the main thread: on Linux the engine is tied to the thread that started it (PR_SET_PDEATHSIG),
/// and the main thread lives as long as Cursora.
fn spawn(app: &AppHandle, program: PathBuf, args: Vec<String>, output: Arc<Mutex<VecDeque<String>>>) -> std::io::Result<Child> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let mut cmd = Command::new(&program);
        cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        if let Some(dir) = program.parent() {
            cmd.current_dir(dir);
        }
        os::prepare(&mut cmd);
        let _ = tx.send(cmd.spawn().inspect(os::adopt));
    });
    let mut child = rx.recv_timeout(Duration::from_secs(10)).map_err(std::io::Error::other)??;
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut out = output.lock().unwrap();
                out.push_back(line);
                if out.len() > 20 {
                    out.pop_front();
                }
            }
        });
    }
    Ok(child)
}

/// Never leave a model in memory: the engine dies with Cursora, crash included.
#[cfg(windows)]
mod os {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command};
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::*;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn prepare(cmd: &mut Command) {
        cmd.creation_flags(CREATE_NO_WINDOW); // no console window flashing up
    }

    /// A Job Object that kills its processes when its last handle closes, which happens when Cursora exits.
    pub fn adopt(child: &Child) {
        static JOB: OnceLock<Option<isize>> = OnceLock::new();
        let job = JOB.get_or_init(|| unsafe {
            let job = CreateJobObjectW(None, windows::core::PCWSTR::null()).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let size = std::mem::size_of_val(&info) as u32;
            SetInformationJobObject(job, JobObjectExtendedLimitInformation, &info as *const _ as _, size).ok()?;
            Some(job.0 as isize) // kept open for Cursora's lifetime on purpose
        });
        if let Some(job) = job {
            if let Err(e) = unsafe { AssignProcessToJobObject(HANDLE(*job as _), HANDLE(child.as_raw_handle())) } {
                log::warn!("engine: no job object: {e}");
            }
        }
    }
}

#[cfg(not(windows))]
mod os {
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    pub fn prepare(cmd: &mut Command) {
        // SAFETY: prctl is async-signal-safe, as required between fork and exec.
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
    }

    pub fn adopt(_: &Child) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_3_3_the_engine_listens_on_127_0_0_1_only() {
        let e = EngineSettings { gguf: "C:/models/qwen.gguf".into(), mmproj: Some("C:/models/mmproj.gguf".into()), ..Default::default() };
        let a = args(&e, 8192, 4321);
        let host = a.iter().position(|x| x == "--host").unwrap();
        assert_eq!(a[host + 1], "127.0.0.1");
        assert!(!a.iter().any(|x| x.contains("0.0.0.0")));
        assert_eq!(a, ["-m", "C:/models/qwen.gguf", "--mmproj", "C:/models/mmproj.gguf", "--host", "127.0.0.1", "--port", "4321", "-c", "8192", "--no-webui"]);
    }

    #[test]
    fn llm_3_5_each_lifetime_stops_the_engine_at_its_moment_but_never_during_an_answer() {
        let with = |lifetime| EngineSettings { lifetime, idle_minutes: 10, ..Default::default() };
        let minutes = |m: u64| Duration::from_secs(m * 60);
        assert_eq!(EngineSettings::default().lifetime, Lifetime::Always, "the default");
        assert!(!should_stop(0, &with(Lifetime::Always), minutes(600), false), "always on: only by hand or on quit");
        assert!(!should_stop(0, &with(Lifetime::Idle), minutes(9), false));
        assert!(should_stop(0, &with(Lifetime::Idle), minutes(11), false));
        assert!(!should_stop(0, &with(Lifetime::Answering), minutes(0), true), "a conversation is on screen");
        assert!(should_stop(0, &with(Lifetime::Answering), minutes(0), false));
        for lifetime in [Lifetime::Idle, Lifetime::Answering] {
            assert!(!should_stop(1, &with(lifetime), minutes(30), false), "{lifetime:?}: an answer in progress keeps it running");
        }
    }
}
