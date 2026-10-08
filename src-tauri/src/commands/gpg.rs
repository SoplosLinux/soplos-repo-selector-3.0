// GPG key management, ported from 2.x's services/gpg_manager.py.

use serde::Serialize;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri_plugin_dialog::DialogExt;

const KEYRINGS_DIR: &str = "/usr/share/keyrings";
const ETC_KEYRINGS_DIR: &str = "/etc/apt/keyrings";

// 2.x's gpg_manager.py calls `gpg --show-keys` with an explicit timeout (2s
// for the list view, 3s for the details popup) specifically because a
// system can have many keyrings in /usr/share/keyrings (every official
// Debian keyring ships there) and a single slow/hanging gpg invocation
// must never freeze the whole list. Rust's Command has no built-in
// timeout, so this is a manual spawn + poll + kill, same pattern as
// soplos-appimage-manager-3.0's extraction timeout.
fn gpg_output_with_timeout(args: &[&str], path: &Path, timeout: Duration) -> Option<String> {
    let mut child = Command::new("gpg").args(args).arg(path).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut bytes = Vec::new();
                use std::io::Read;
                child.stdout.take()?.read_to_end(&mut bytes).ok()?;
                // gpg --show-keys output can contain non-UTF-8 UIDs (2.x
                // CHANGELOG 2.0.3-1 fix) — lossy decode mirrors Python's
                // errors='replace', never aborts the whole listing.
                return Some(String::from_utf8_lossy(&bytes).to_string());
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct GpgKey {
    pub name: String,
    pub path: String,
    pub format: String,
    pub description: String,
    // True when no configured source references this key (by its full path
    // or its bare filename, the same two forms a Signed-By= line can use) —
    // the kind of leftover a removed repo's key becomes when only the
    // repo's .sources/.list file gets deleted. Computed from the same
    // reference check delete_gpg_key already uses to refuse deleting a
    // key that's still in use.
    pub unused: bool,
}

#[derive(Serialize)]
pub struct GpgKeyDetails {
    pub fingerprint: String,
    pub expiry: Option<String>,
    pub uid: String,
}

// True when this file belongs to an installed .deb (e.g. the individual
// per-purpose files debian-archive-keyring/debian-keyring ship alongside
// the merged keyring that sources.list.d files actually reference by
// Signed-By, or a third-party repo's own -keyring package). Those are
// never meant to be referenced directly and must never be flagged as
// "unused" just because nothing points at them — unlike a key a user
// imported by hand for a repo they later removed, which dpkg knows
// nothing about and genuinely is safe to call orphaned.
fn is_dpkg_owned(path: &Path) -> bool {
    Command::new("dpkg").arg("-S").arg(path).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

// Every configured source file that mentions this key, by its full path or
// its bare filename (both are valid in a Signed-By= line or a legacy
// `signed-by=` option). Shared by delete_gpg_key (which refuses to delete a
// referenced key without force=true) and the unused-key scan below.
fn references_to(key_path: &Path) -> Vec<String> {
    let mut referenced_in = Vec::new();
    let key_path_str = key_path.to_string_lossy().to_string();
    let basename = key_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    let mut candidates = vec!["/etc/apt/sources.list".to_string()];
    if let Ok(entries) = std::fs::read_dir("/etc/apt/sources.list.d") {
        candidates.extend(entries.flatten().map(|e| e.path().to_string_lossy().to_string()));
    }
    for conf in &candidates {
        if let Ok(data) = std::fs::read_to_string(conf) {
            if data.contains(&key_path_str) || data.contains(&basename) {
                referenced_in.push(conf.clone());
            }
        }
    }
    referenced_in
}

fn is_ascii_armored(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else { return false };
    let header = String::from_utf8_lossy(&bytes[..bytes.len().min(100)]);
    header.contains("-----BEGIN PGP")
}

// gpg --show-keys output can contain non-UTF-8 UIDs (2.x CHANGELOG 2.0.3-1
// fix) — a lossy read mirrors Python's `errors='replace'`, never aborts.
// The timeout itself is the other half of that same fix: 2.x bounds every
// call to 2s (list) or 3s (details popup) specifically because a system
// can have many keyrings and one slow gpg call must never freeze the list.
fn gpg_show_keys(path: &Path, timeout: Duration) -> Option<String> {
    gpg_output_with_timeout(&["--show-keys", "--with-colons"], path, timeout)
}

fn key_info(path: &Path) -> GpgKey {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let format = if name.ends_with(".asc") { "ASCII" } else { "GPG Binary" }.to_string();
    let mut description = "GPG Key".to_string();

    if let Some(stdout) = gpg_show_keys(path, Duration::from_secs(2)) {
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() > 9 && parts[0] == "uid" {
                description = parts[9].to_string();
                break;
            } else if parts.len() > 4 && parts[0] == "pub" {
                description = format!("ID: {}", parts[4]);
            }
        }
    }

    let unused = references_to(path).is_empty() && !is_dpkg_owned(path);

    GpgKey { name, path: path.to_string_lossy().to_string(), format, description, unused }
}

fn get_all_keys_blocking() -> Vec<GpgKey> {
    let mut paths = Vec::new();
    for dir in [KEYRINGS_DIR, ETC_KEYRINGS_DIR] {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.to_string_lossy().to_lowercase();
            if name.ends_with(".gpg") || name.ends_with(".asc") || name.ends_with(".pgp") {
                paths.push(path);
            }
        }
    }

    // A real Debian system ships a dozen-plus official keyrings in
    // /usr/share/keyrings by default. key_info() calls out to `gpg` per
    // file (up to a 2s timeout each) — doing that one at a time made the
    // list take as long as the sum of every call. Running them in parallel
    // bounds the wait to roughly the slowest single call instead.
    let mut keys = Vec::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = paths.iter().map(|path| scope.spawn(|| key_info(path))).collect();
        for handle in handles {
            if let Ok(key) = handle.join() {
                keys.push(key);
            }
        }
    });
    keys
}

#[tauri::command]
pub async fn list_gpg_keys() -> Vec<GpgKey> {
    tauri::async_runtime::spawn_blocking(get_all_keys_blocking).await.unwrap_or_default()
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn get_gpg_key_details(keyPath: String) -> Option<GpgKeyDetails> {
    tauri::async_runtime::spawn_blocking(move || {
        let stdout = gpg_show_keys(Path::new(&keyPath), Duration::from_secs(3))?;
        let mut fingerprint = String::new();
        let mut expiry = None;
        let mut uid = String::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.is_empty() {
                continue;
            }
            match parts[0] {
                "fpr" if parts.len() >= 10 => fingerprint = parts.last().unwrap().to_string(),
                "pub" if parts.len() > 6 => {
                    if let Ok(ts) = parts[6].parse::<i64>() {
                        expiry = chrono_like_iso(ts);
                    }
                }
                "uid" if parts.len() > 9 => uid = parts[9].to_string(),
                _ => {}
            }
        }

        Some(GpgKeyDetails { fingerprint, expiry, uid })
    })
    .await
    .unwrap_or(None)
}

// No chrono dependency for one timestamp field — a minimal UTC epoch ->
// ISO-8601 conversion is enough and keeps this command's deps small.
fn chrono_like_iso(epoch_secs: i64) -> Option<String> {
    if epoch_secs <= 0 {
        return None;
    }
    let days = epoch_secs / 86400;
    let secs_of_day = epoch_secs % 86400;
    let (h, m, s) = (secs_of_day / 3600, (secs_of_day % 3600) / 60, secs_of_day % 60);

    // Civil-from-days algorithm (Howard Hinnant's public-domain date algorithm).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_month = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m_month <= 2 { y + 1 } else { y };

    Some(format!("{y:04}-{m_month:02}-{d:02}T{h:02}:{m:02}:{s:02}Z"))
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn import_gpg_key(sourcePath: String, keyName: Option<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = Path::new(&sourcePath);
        if !source.exists() {
            return Err(format!("File {sourcePath} does not exist"));
        }
        let mut name = keyName.unwrap_or_else(|| source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        if !(name.ends_with(".gpg") || name.ends_with(".asc") || name.ends_with(".pgp")) {
            name.push_str(".gpg");
        }

        // /etc/apt/keyrings is the preferred destination, created here as
        // part of the SAME elevated script if it doesn't exist yet — one
        // pkexec prompt, not a separate one at every app launch (that
        // earlier version could hang indefinitely on a desktop with no
        // polkit authentication agent running, e.g. bare XFCE).
        let dst = format!("{ETC_KEYRINGS_DIR}/{name}");

        let script = if is_ascii_armored(source) {
            format!("mkdir -p '{ETC_KEYRINGS_DIR}' && cat '{sourcePath}' | gpg --dearmor > '{dst}' && chmod 644 '{dst}'")
        } else {
            format!("mkdir -p '{ETC_KEYRINGS_DIR}' && cp '{sourcePath}' '{dst}' && chmod 644 '{dst}'")
        };

        crate::privilege::run_elevated(&script)?;
        Ok(dst)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn export_gpg_key(srcPath: String, dstPath: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let src = Path::new(&srcPath);
        if !src.exists() {
            return Err(format!("Source {srcPath} does not exist"));
        }
        if let Some(parent) = Path::new(&dstPath).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|_| format!("Permission denied creating destination directory {}", parent.display()))?;
            }
        }

        if is_ascii_armored(src) && dstPath.ends_with(".gpg") {
            let output = Command::new("gpg").args(["--dearmor", "-o", &dstPath]).arg(&srcPath).output().map_err(|e| e.to_string())?;
            if output.status.success() {
                Ok(dstPath)
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        } else {
            std::fs::copy(&srcPath, &dstPath).map_err(|e| e.to_string())?;
            Ok(dstPath)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn delete_gpg_key(keyPath: String, force: bool) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = Path::new(&keyPath);
        if !path.exists() {
            return Err(format!("Key {keyPath} does not exist"));
        }

        let referenced_in = references_to(path);

        if !referenced_in.is_empty() && !force {
            return Err(format!("Key referenced in: {}", referenced_in.join(", ")));
        }

        match std::fs::remove_file(path) {
            Ok(()) => Ok(format!("Deleted {keyPath}")),
            Err(_) => {
                crate::privilege::run_elevated(&format!("rm -f '{keyPath}'"))?;
                Ok(format!("Deleted {keyPath} (via pkexec)"))
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn pick_gpg_key_file(app: tauri::AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select a GPG key")
            .add_filter("GPG key files", &["gpg", "asc", "pgp"])
            .blocking_pick_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn pick_gpg_export_path(app: tauri::AppHandle, suggestedName: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Export GPG key")
            .set_file_name(&suggestedName)
            .blocking_save_file()
            .and_then(|fp| fp.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .unwrap_or(None)
}

