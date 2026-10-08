use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set".to_string())
}

/// Detects whether the process is running inside a virtual machine.
///
/// WebKitGTK's DMA-BUF renderer can fail silently on virtual GPUs (VMware,
/// VirtualBox, QEMU/KVM, Hyper-V), leaving the window blank with no error.
/// main.rs uses this to decide whether to force
/// `WEBKIT_DISABLE_DMABUF_RENDERER=1` before the webview initializes. Ported
/// from soplos-welcome-3.0's utils::is_running_in_vm.
pub fn is_running_in_vm() -> bool {
    if detect_virt_via_dmi() {
        return true;
    }
    detect_virt_via_systemd()
}

fn detect_virt_via_dmi() -> bool {
    const NEEDLES: [&str; 6] = ["vmware", "virtualbox", "qemu", "kvm", "xen", "innotek"];

    for path in [
        "/sys/class/dmi/id/sys_vendor",
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/bios_vendor",
    ] {
        if let Ok(content) = std::fs::read_to_string(path) {
            let content = content.to_lowercase();
            if NEEDLES.iter().any(|needle| content.contains(needle)) {
                return true;
            }
        }
    }
    false
}

fn detect_virt_via_systemd() -> bool {
    Command::new("systemd-detect-virt")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
