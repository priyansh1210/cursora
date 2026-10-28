//! The dedicated graphics card the built-in engine may use (spec 23). llama.cpp's Vulkan build would otherwise fall back
//! to integrated graphics, which on the laptop was slower than the processor (GPU-1.1), so Cursora asks the Vulkan
//! driver itself which kind of device each one is.
use ash::vk;
use std::sync::OnceLock;

/// The first dedicated card's name; None without one, or without Vulkan (GPU-1.3). Asked once: about 50 ms, and cards
/// don't change while Cursora runs.
pub fn dedicated() -> Option<String> {
    static FOUND: OnceLock<Option<String>> = OnceLock::new();
    FOUND
        .get_or_init(|| {
            let found = find().unwrap_or_else(|e| {
                log::info!("gpu: {e}");
                None
            });
            log::info!("gpu: dedicated card {found:?}");
            found
        })
        .clone()
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

/// GPU-1.4: the card Settings names, or None.
#[tauri::command]
pub fn engine_gpu() -> Option<String> {
    dedicated()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_1_3_asking_for_the_card_never_fails() {
        // Whatever this computer has: a card's name, or None without one or without Vulkan (CI, WSL), never a panic.
        let found = dedicated();
        eprintln!("dedicated graphics card: {found:?}");
        assert!(found.as_deref().is_none_or(|n| !n.is_empty()));
    }
}
