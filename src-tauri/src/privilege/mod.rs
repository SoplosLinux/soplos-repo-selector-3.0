// Privileged-execution helper, gated by a real polkit policy
// (resources/polkit/org.soplos.reposelector.policy, installed to
// /usr/share/polkit-1/actions/). Every privileged operation goes through
// run_elevated(): it writes the script to a private temp file, then spawns
// `pkexec <this binary> --run-privileged-script <path>`, which main()
// catches before Tauri starts and routes to run_privileged_script_file
// below. polkit authorizes pkexec to run this specific binary under the
// declared action; nothing else on the system gets that authorization only
// because it also happens to call pkexec. Same pattern as soplos-welcome-3.0.

use std::fs;
use std::io::Write;
use std::process::Command;

/// Reads the script written by run_elevated, deletes the temp file (it may
/// contain repo URIs or paths — no reason to leave it lying around after
/// use) and runs it via `sh -c`. Returns the process exit code.
pub fn run_privileged_script_file(path: &str) -> i32 {
    let script = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("soplos-repo-selector: could not read privileged script at {path}: {e}");
            return 1;
        }
    };
    let _ = fs::remove_file(path);

    match Command::new("sh").arg("-c").arg(&script).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("soplos-repo-selector: failed to run privileged script: {e}");
            1
        }
    }
}

fn self_exe_path() -> String {
    std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "soplos-repo-selector".to_string())
}

fn write_script(script: &str) -> std::io::Result<std::path::PathBuf> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    let path = std::env::temp_dir().join(format!("soplos-repo-selector-privileged-{}-{nanos}.sh", std::process::id()));
    let mut file = fs::File::create(&path)?;
    file.write_all(script.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}

/// Runs `script` as root via the polkit-gated self-exec, blocking until it
/// finishes. Used for every pkexec operation in this app (saving repo
/// files, modernizing, GPG key import/delete, creating /etc/apt/keyrings).
pub fn run_elevated(script: &str) -> Result<(), String> {
    let path = write_script(script).map_err(|e| e.to_string())?;
    let result = Command::new("pkexec").arg(self_exe_path()).arg("--run-privileged-script").arg(&path).output();
    // run_privileged_script_file on the other side deletes the temp file
    // after reading it; this cleans it up on our side too in case pkexec
    // itself failed (auth cancelled) before ever invoking that path.
    let _ = fs::remove_file(&path);

    match result {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).to_string()),
        Err(e) => Err(e.to_string()),
    }
}
