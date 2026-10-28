//! The dedicated graphics card the built-in engine may use (spec 23). llama.cpp's Vulkan build would otherwise fall back
//! to integrated graphics, which on the laptop was slower than the processor (GPU-1.1), so Cursora asks the Vulkan
//! driver itself which kind of device each one is.
use ash::vk;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// `cursora --gpu` prints the card's name and exits; read before anything else starts.
pub const PROBE: &str = "--gpu";

/// The first dedicated card's name; None without one, or without Vulkan (GPU-1.3). Asked once: cards don't change while
/// Cursora runs.
pub fn dedicated() -> Option<String> {
    static FOUND: OnceLock<Option<String>> = OnceLock::new();
    FOUND
        .get_or_init(|| {
            let found = ask().unwrap_or_else(|e| {
                log::info!("gpu: {e}");
                None
            });
            log::info!("gpu: dedicated card {found:?}");
            found
        })
        .clone()
}

/// Asked in a child process: the Vulkan driver stays loaded in the process that asked, about 60 MB with NVIDIA's, more
/// than Cursora's whole idle budget (constitution §8).
// shortcut: no time limit on the child; add one if a driver is ever seen to hang.
fn ask() -> Result<Option<String>, String> {
    let mut cmd = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    cmd.arg(PROBE).stdin(Stdio::null());
    super::engine::os::prepare(&mut cmd);
    let out = cmd.output().map_err(|e| format!("couldn't ask: {e}"))?;
    if !out.status.success() {
        return Err(format!("{}: {}", out.status, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    Ok((!name.is_empty()).then_some(name))
}

/// The child's side of `ask()`: the name or an empty line, or the error and exit code 1.
pub fn probe() {
    match find() {
        Ok(found) => println!("{}", found.unwrap_or_default()),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn find() -> Result<Option<String>, String> {
    // Loads vulkan-1.dll or libvulkan.so.1 now, so Cursora builds and runs without Vulkan.
    let entry = unsafe { ash::Entry::load() }.map_err(|e| format!("no Vulkan: {e}"))?;
    let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_0);
    let instance = unsafe { entry.create_instance(&vk::InstanceCreateInfo::default().application_info(&app), None) }.map_err(|e| format!("Vulkan: {e}"))?;
    let found = unsafe { instance.enumerate_physical_devices() }.map_err(|e| format!("Vulkan: {e}")).map(|devices| {
        devices
            .into_iter()
            .map(|d| unsafe { instance.get_physical_device_properties(d) })
            .find(|p| p.device_type == vk::PhysicalDeviceType::DISCRETE_GPU)
            .map(|p| p.device_name_as_c_str().map_or_else(|_| "a graphics card".into(), |n| n.to_string_lossy().into_owned()))
    });
    unsafe { instance.destroy_instance(None) };
    found
}

/// GPU-1.4: the card Settings names, or None. Off the main thread, as the first call waits for the child.
#[tauri::command(async)]
pub fn engine_gpu() -> Option<String> {
    dedicated()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_1_3_asking_for_the_card_never_fails() {
        // Whatever this computer has: a card's name, or None without one or without Vulkan (CI, WSL), never a panic.
        let found = find();
        eprintln!("dedicated graphics card: {found:?}");
        assert!(found.is_err() || found.unwrap().is_none_or(|n| !n.is_empty()));
        // Here the child is the test program, which rejects --gpu: that's None too, not a failed question.
        assert_eq!(dedicated(), None);
    }
}
