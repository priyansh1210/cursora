//! Run as administrator (APP-3.3–3.7). Windows only: there a normal app can't read admin apps (SEL-6.1).
//! `startup`, `task_xml`, `starts` and `inside` are plain functions, tested on every OS.
// Only the Windows code calls the plain functions; they are built everywhere so their tests run on Linux too.
#![cfg_attr(not(windows), allow(dead_code))]

/// Passed by a normal copy turning the setting on: the admin copy waits for that pid to quit (APP-3.4).
pub const AFTER: &str = "--after=";

#[derive(Debug, PartialEq)]
pub enum Startup {
    Nothing,
    Elevate,
    Install,
    Remove,
}

/// What a start does, from the setting and whether this copy is admin (APP-3.7).
pub fn startup(on: bool, elevated: bool) -> Startup {
    match (on, elevated) {
        (true, false) => Startup::Elevate,
        (true, true) => Startup::Install,
        (false, true) => Startup::Remove,
        (false, false) => Startup::Nothing,
    }
}

/// On demand only, and without the defaults that would stop Cursora: a 72-hour limit, AC power only,
/// and priority 7 (below normal). Parallel lets a relaunch reach the running copy (APP-3.6, APP-2.4).
pub fn task_xml(exe: &str, user: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Principals>
    <Principal id="Author">
      <UserId>{}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>Parallel</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
    </Exec>
  </Actions>
</Task>
"#,
        esc(user),
        esc(exe)
    )
}

/// Whether a task's XML starts this exe. Task Scheduler reports success even when its exe is gone, and a task
/// left by a copy installed elsewhere would start that copy, or nothing (APP-3.3).
pub fn starts(xml: &str, exe: &str) -> bool {
    xml.contains(&format!("<Command>{}</Command>", esc(exe)))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Whether `exe` is inside one of `dirs`. Windows paths ignore case (APP-3.8).
pub fn inside(exe: &str, dirs: &[String]) -> bool {
    let exe = exe.to_lowercase();
    dirs.iter().filter(|d| !d.is_empty()).any(|d| exe.starts_with(&format!("{}\\", d.trim_end_matches('\\').to_lowercase())))
}

#[cfg(windows)]
pub use win::*;
#[cfg(not(windows))]
pub use other::*;

/// Linux needs no admin rights to read other apps, so there is nothing to do.
#[cfg(not(windows))]
mod other {
    pub fn available() -> bool {
        false
    }
    pub fn elevated() -> bool {
        false
    }
    pub fn in_program_files() -> bool {
        false
    }
    pub fn before_start(_identifier: &str) -> Option<String> {
        None
    }
    pub fn runas(_args: &str) -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn sync(_on: bool) {}
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::path::PathBuf;
    use windows::core::{w, BSTR, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, E_FAIL, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevationType, TokenElevationTypeDefault, TOKEN_ELEVATION_TYPE, TOKEN_QUERY};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::System::TaskScheduler::{ITaskFolder, ITaskService, TaskScheduler, TASK_CREATE_OR_UPDATE, TASK_LOGON_INTERACTIVE_TOKEN};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, ShellExecuteW, FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, KF_FLAG_DEFAULT};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    /// Whether this copy runs as administrator: the check SEL-6.1 uses.
    pub fn elevated() -> bool {
        unsafe { crate::capture::is_elevated(GetCurrentProcess()) }
    }

    /// An administrator account with UAC on. A standard account's prompt asks for another user, and Cursora
    /// would run as that user, with that user's settings; with UAC off everything already runs as admin.
    pub fn available() -> bool {
        let mut token = HANDLE::default();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
            return false;
        }
        let mut kind = TOKEN_ELEVATION_TYPE::default();
        let mut len = 0u32;
        let ok = unsafe { GetTokenInformation(token, TokenElevationType, Some(&mut kind as *mut _ as *mut _), size_of::<TOKEN_ELEVATION_TYPE>() as u32, &mut len) }.is_ok();
        let _ = unsafe { CloseHandle(token) };
        ok && kind != TokenElevationTypeDefault
    }

    /// Only administrators can change Program Files; anywhere else the task would hand admin rights to whatever
    /// program replaced Cursora's files (APP-3.8). The folders come from HKLM: `%ProgramFiles%` is the user's to set.
    pub fn in_program_files() -> bool {
        std::env::current_exe().is_ok_and(|exe| inside(&exe.to_string_lossy(), &program_files()))
    }

    fn program_files() -> [String; 2] {
        [FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86].map(|id| unsafe {
            SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None).map_or(String::new(), |p| {
                let dir = p.to_string().unwrap_or_default();
                CoTaskMemFree(Some(p.0 as _));
                dir
            })
        })
    }

    #[test]
    fn app_3_8_program_files_come_from_windows() {
        let [x64, x86] = program_files();
        assert!(x64.ends_with(r":\Program Files") && x86.ends_with(r":\Program Files (x86)"), "{x64} / {x86}");
        assert!(!in_program_files(), "the test exe is in target");
    }

    /// First thing in `run()`, before the single-instance plugin: a normal copy can't open an admin copy's lock,
    /// so it must hand over before taking its own (APP-3.3). Returns what to log once logging runs.
    pub fn before_start(identifier: &str) -> Option<String> {
        if let Some(pid) = std::env::args().find_map(|a| a.strip_prefix(AFTER)?.parse::<u32>().ok()) {
            wait_for(pid);
        }
        if !available() {
            return None;
        }
        let on = setting_on(identifier);
        if on && !in_program_files() {
            return Some("run as administrator: off, because Cursora isn't installed in Program Files".into());
        }
        if startup(on, elevated()) != Startup::Elevate {
            return None;
        }
        // shortcut: an action given to this launch (--ask, --snip, --settings) is dropped, as the task takes no arguments; pass them as the task's $(Arg0) if someone needs it.
        let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let task = with_tasks(move |f| unsafe {
            let registered = f.GetTask(&task())?;
            if !starts(&registered.Xml()?.to_string(), &exe) {
                return Err(windows::core::Error::new(E_FAIL, "the task starts another copy of Cursora"));
            }
            registered.Run(&VARIANT::default()).map(drop)
        });
        match task.map_err(|e| e.message()).or_else(|t| runas("").map_err(|r| format!("{t}; {r}"))) {
            Ok(()) => std::process::exit(0),
            Err(e) => Some(format!("run as administrator: {e}; running as a normal app")),
        }
    }

    /// The UAC prompt; returns once the user answered (APP-3.4).
    pub fn runas(args: &str) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let started = unsafe { ShellExecuteW(None, w!("runas"), &HSTRING::from(exe.as_os_str()), &HSTRING::from(args), PCWSTR::null(), SW_SHOWNORMAL) };
        // Above 32 means it started; the user's No comes back as 5 (access denied).
        if started.0 as isize > 32 { Ok(()) } else { Err(format!("ShellExecute returned {}", started.0 as isize)) }
    }

    /// Creates or updates the task from an admin copy, or deletes it; a normal copy may do neither (APP-3.7).
    /// Runs on its own thread: Task Scheduler takes a moment, and COM must not touch the main thread.
    pub fn sync(on: bool) {
        if !available() {
            return;
        }
        let action = startup(on && in_program_files(), elevated());
        std::thread::spawn(move || match action {
            Startup::Install => {
                if let Err(e) = std::env::current_exe().map_err(|e| e.to_string()).and_then(|exe| install(&exe)) {
                    log::warn!("run as administrator: couldn't create the task: {e}");
                }
            }
            // Fails when there is no task, the usual case.
            Startup::Remove => drop(with_tasks(|f| unsafe { f.DeleteTask(&task(), 0) })),
            Startup::Elevate | Startup::Nothing => {}
        });
    }

    fn install(exe: &std::path::Path) -> Result<(), String> {
        let xml = task_xml(&exe.to_string_lossy(), &format!("{}\\{}", var("USERDOMAIN"), var("USERNAME")));
        // The XML goes straight to Task Scheduler: a file would let any normal program swap in its own command.
        with_tasks(move |f| unsafe {
            let none = VARIANT::default();
            f.RegisterTask(&task(), &BSTR::from(xml), TASK_CREATE_OR_UPDATE.0, &none, &none, TASK_LOGON_INTERACTIVE_TOKEN, &none).map(drop)
        })
        .map_err(|e| e.message())
    }

    /// The task that starts Cursora as administrator with no UAC prompt. One per Windows account: two administrators
    /// on one PC would otherwise overwrite each other's.
    fn task() -> BSTR {
        BSTR::from(format!("Cursora (administrator, {})", var("USERNAME")))
    }

    fn var(key: &str) -> String {
        std::env::var(key).unwrap_or_default()
    }

    /// Task Scheduler's root folder on a thread of its own, so COM never meets the main thread's own setup.
    fn with_tasks<T: Send + 'static>(f: impl FnOnce(&ITaskFolder) -> windows::core::Result<T> + Send + 'static) -> windows::core::Result<T> {
        std::thread::spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let result = (|| {
                let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)?;
                let none = VARIANT::default();
                service.Connect(&none, &none, &none, &none)?;
                f(&service.GetFolder(&BSTR::from("\\"))?)
            })();
            CoUninitialize();
            result
        })
        .join()
        .unwrap_or_else(|_| Err(E_FAIL.into()))
    }

    /// Tauri's config dir isn't known before the app is built; on Windows it is %APPDATA%\<identifier>.
    fn setting_on(identifier: &str) -> bool {
        let path = std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join(identifier).join("settings.json"));
        path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| crate::settings::parse(&t).ok()).is_some_and(|s| s.run_as_admin)
    }

    /// The copy that asked for this one holds the single-instance lock until it quits.
    fn wait_for(pid: u32) {
        if let Ok(process) = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) } {
            unsafe { WaitForSingleObject(process, 10_000) };
            let _ = unsafe { CloseHandle(process) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_3_6_task_runs_on_demand_as_admin_without_limits() {
        let xml = task_xml(r"C:\Program Files\A&B <x>\cursora.exe", r"PC\kabra");
        assert!(!xml.contains("<Triggers"), "on demand only");
        for part in [
            "<UserId>PC\\kabra</UserId>",
            "<LogonType>InteractiveToken</LogonType>",
            "<RunLevel>HighestAvailable</RunLevel>",
            "<MultipleInstancesPolicy>Parallel</MultipleInstancesPolicy>",
            "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
            "<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>",
            "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>",
            "<Priority>5</Priority>",
            r"<Command>C:\Program Files\A&amp;B &lt;x&gt;\cursora.exe</Command>",
        ] {
            assert!(xml.contains(part), "missing {part}");
        }
    }

    #[test]
    fn app_3_3_task_must_start_this_exe() {
        let exe = r"C:\Program Files\A&B\cursora.exe";
        let xml = task_xml(exe, r"PC\kabra");
        assert!(starts(&xml, exe));
        assert!(!starts(&xml, r"C:\Users\kabra\AppData\Local\Cursora\cursora.exe"), "a copy installed elsewhere");
        assert!(!starts(&xml, r"C:\Program Files\A&B\cursor"), "only part of the path");
    }

    #[test]
    fn app_3_8_only_from_program_files() {
        let dirs = [r"C:\Program Files".to_string(), r"C:\Program Files (x86)\".to_string()];
        assert!(inside(r"C:\Program Files\Cursora\cursora.exe", &dirs));
        assert!(inside(r"c:\program files (x86)\Cursora\cursora.exe", &dirs), "any letter case");
        assert!(!inside(r"C:\Program Files (x86)\Cursora\cursora.exe", &dirs[..1]), "a folder whose name only starts the same");
        assert!(!inside(r"C:\Users\kabra\AppData\Local\Cursora\cursora.exe", &dirs), "a per-user install");
        assert!(!inside(r"D:\cursora\src-tauri\target\debug\cursora.exe", &dirs), "a dev build");
        assert!(!inside(r"\\server\share\cursora.exe", &[String::new()]), "a folder that couldn't be read");
    }

    #[test]
    fn app_3_7_start_follows_setting_and_elevation() {
        assert_eq!(startup(true, false), Startup::Elevate);
        assert_eq!(startup(true, true), Startup::Install);
        assert_eq!(startup(false, true), Startup::Remove);
        assert_eq!(startup(false, false), Startup::Nothing);
    }
}
