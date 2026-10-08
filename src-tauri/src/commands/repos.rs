// APT repository management, ported from 2.x's services/repo_manager.py and
// services/repo_file_manager.py. Parsing/generation logic is kept 1:1 with
// the Python version, including the two fixes documented in its CHANGELOG
// (2.0.3-1): a DEB822 stanza is only "disabled" when EVERY non-empty line
// starts with '#' (not just the first), and GPG key resolution matches the
// full repo hostname before falling back to loose keyword matching.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const SOURCES_LIST: &str = "/etc/apt/sources.list";
const SOURCES_PARTS: &str = "/etc/apt/sources.list.d";
const GPG_DIRS: [&str; 3] = ["/usr/share/keyrings", "/etc/apt/keyrings", "/etc/apt/trusted.gpg.d"];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Repo {
    pub disabled: bool,
    #[serde(rename = "type")]
    pub repo_type: String,
    pub uri: String,
    pub distribution: String,
    pub components: String,
    pub comment: String,
    #[serde(rename = "signedBy")]
    pub signed_by: String,
    pub file: String,
    pub format: String,
    #[serde(rename = "rawOptions", skip_serializing_if = "Option::is_none")]
    pub raw_options: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<usize>,
}

fn is_deb822_format(content: &str) -> bool {
    ["Types:", "URIs:", "Suites:", "Components:"]
        .iter()
        .any(|field| content.lines().any(|l| l.trim_start().starts_with(field)))
}

// Matches the Python regex: ^\s*(deb|deb-src)\s+(\[.*?\]\s+)?(\S+)\s+(\S+)\s*(.*?)\s*$
fn parse_deb_line(line: &str) -> Option<(String, String, String, String, String)> {
    let line = line.trim();
    let (repo_type, rest) = if let Some(r) = line.strip_prefix("deb-src") {
        ("deb-src", r)
    } else if let Some(r) = line.strip_prefix("deb") {
        ("deb", r)
    } else {
        return None;
    };
    let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();

    let (options, rest) = if let Some(r) = rest.strip_prefix('[') {
        let end = r.find(']')?;
        (r[..end].to_string(), r[end + 1..].trim_start().to_string())
    } else {
        (String::new(), rest.to_string())
    };

    let mut parts = rest.splitn(2, char::is_whitespace);
    let uri = parts.next()?.to_string();
    let rest = parts.next().unwrap_or("").trim_start();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let distribution = parts.next()?.to_string();
    let components = parts.next().unwrap_or("").trim().to_string();

    Some((repo_type.to_string(), options, uri, distribution, components))
}

fn extract_signed_by(options: &str) -> String {
    options
        .split(' ')
        .find_map(|tok| tok.strip_prefix("signed-by="))
        .unwrap_or("")
        .to_string()
}

fn parse_legacy_format(content: &str, file_path: &str) -> Vec<Repo> {
    let mut repos = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let mut disabled = false;
        let candidate = if let Some(after_hash) = trimmed.strip_prefix('#') {
            let after_hash = after_hash.trim();
            if parse_deb_line(after_hash).is_some() {
                disabled = true;
                after_hash
            } else {
                continue;
            }
        } else {
            trimmed
        };
        if candidate.is_empty() {
            continue;
        }
        if let Some((repo_type, options, uri, distribution, components)) = parse_deb_line(candidate) {
            repos.push(Repo {
                disabled,
                repo_type,
                uri,
                distribution,
                components,
                comment: String::new(),
                signed_by: extract_signed_by(&options),
                file: file_path.to_string(),
                format: "legacy".to_string(),
                raw_options: Some(options.trim().to_string()),
                block: None,
            });
        }
    }
    repos
}

fn parse_deb822_format(content: &str, file_path: &str) -> Vec<Repo> {
    let mut repos = Vec::new();
    let blocks: Vec<&str> = content.trim().split("\n\n").collect();

    for (block_num, raw_block) in blocks.iter().enumerate() {
        if raw_block.trim().is_empty() {
            continue;
        }
        let non_empty: Vec<&str> = raw_block.lines().filter(|l| !l.trim().is_empty()).collect();
        let disabled = !non_empty.is_empty() && non_empty.iter().all(|l| l.trim_start().starts_with('#'));

        let effective_block = if disabled {
            raw_block
                .lines()
                .map(|l| if let Some(r) = l.trim_start().strip_prefix('#') { r } else { l })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            raw_block.to_string()
        };

        let mut fields: HashMap<String, String> = HashMap::new();
        for line in effective_block.lines() {
            let line = line.trim();
            if let Some((k, v)) = line.split_once(':') {
                fields.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }

        let (Some(types), Some(uris), Some(suites)) = (fields.get("types"), fields.get("uris"), fields.get("suites")) else {
            continue;
        };
        let components = fields.get("components").cloned().unwrap_or_default();
        let signed_by = fields.get("signed-by").cloned().unwrap_or_default();

        for repo_type in types.split_whitespace() {
            for uri in uris.split_whitespace() {
                for suite in suites.split_whitespace() {
                    repos.push(Repo {
                        disabled,
                        repo_type: repo_type.to_string(),
                        uri: uri.to_string(),
                        distribution: suite.to_string(),
                        components: components.clone(),
                        comment: String::new(),
                        signed_by: signed_by.clone(),
                        file: file_path.to_string(),
                        format: "deb822".to_string(),
                        raw_options: None,
                        block: Some(block_num),
                    });
                }
            }
        }
    }
    repos
}

fn read_sources_file(path: &Path) -> Vec<Repo> {
    let Ok(content) = std::fs::read_to_string(path) else { return Vec::new() };
    let path_str = path.to_string_lossy().to_string();
    if path.extension().and_then(|e| e.to_str()) == Some("sources") || is_deb822_format(&content) {
        parse_deb822_format(&content, &path_str)
    } else {
        parse_legacy_format(&content, &path_str)
    }
}

const GENERATED_BY_HEADER: &str = "# Generated by Soplos Repo Selector";
const DO_NOT_EDIT_HEADER: &str = "# Do not edit manually";
const FORMAT_DEB822_HEADER: &str = "# Format: DEB822";

fn generate_legacy_content(repos: &[Repo]) -> String {
    let mut lines = vec![GENERATED_BY_HEADER.to_string(), DO_NOT_EDIT_HEADER.to_string(), String::new()];
    for repo in repos {
        let prefix = if repo.disabled { "# " } else { "" };
        let mut options = repo.raw_options.clone().unwrap_or_default();
        if options.is_empty() && !repo.signed_by.is_empty() {
            options = format!("[signed-by={}] ", repo.signed_by);
        } else if !repo.signed_by.is_empty() && !options.contains("signed-by") {
            options = format!("{} [signed-by={}] ", options.trim(), repo.signed_by);
        }
        if !options.is_empty() && !options.ends_with(' ') {
            options.push(' ');
        }
        let mut line = format!("{prefix}{} {options}{} {}", repo.repo_type, repo.uri, repo.distribution);
        if !repo.components.is_empty() {
            line.push(' ');
            line.push_str(&repo.components);
        }
        if !repo.comment.is_empty() {
            lines.push(format!("# {}", repo.comment));
        }
        lines.push(line);
    }
    lines.join("\n") + "\n"
}

fn generate_deb822_content(repos: &[Repo]) -> String {
    let mut lines = vec![GENERATED_BY_HEADER.to_string(), FORMAT_DEB822_HEADER.to_string(), String::new()];

    if repos.iter().any(|r| r.block.is_some()) {
        let mut blocks: std::collections::BTreeMap<usize, Vec<&Repo>> = std::collections::BTreeMap::new();
        for r in repos {
            blocks.entry(r.block.unwrap_or(0)).or_default().push(r);
        }
        for group in blocks.values() {
            write_deb822_block(&mut lines, group);
        }
        return lines.join("\n");
    }

    // Fallback: group by (signed_by, disabled), matching the Python
    // _group_similar_repos "best effort" strategy.
    let mut groups: Vec<Vec<&Repo>> = Vec::new();
    for repo in repos {
        if let Some(g) = groups.iter_mut().find(|g: &&mut Vec<&Repo>| {
            let first = g[0];
            first.signed_by == repo.signed_by && first.disabled == repo.disabled
        }) {
            g.push(repo);
        } else {
            groups.push(vec![repo]);
        }
    }
    for group in &groups {
        write_deb822_block(&mut lines, group);
    }
    lines.join("\n")
}

fn write_deb822_block(lines: &mut Vec<String>, group: &[&Repo]) {
    let disabled = group.iter().any(|r| r.disabled);
    let prefix = if disabled { "# " } else { "" };

    let mut uris = Vec::new();
    let mut types = Vec::new();
    let mut suites = Vec::new();
    let mut components_seen = std::collections::HashSet::new();
    let mut components = Vec::new();
    let mut signed_by = String::new();

    for r in group {
        if !uris.contains(&r.uri) {
            uris.push(r.uri.clone());
        }
        if !types.contains(&r.repo_type) {
            types.push(r.repo_type.clone());
        }
        if !suites.contains(&r.distribution) {
            suites.push(r.distribution.clone());
        }
        for word in r.components.split_whitespace() {
            if components_seen.insert(word.to_string()) {
                components.push(word.to_string());
            }
        }
        if signed_by.is_empty() && !r.signed_by.is_empty() {
            signed_by = r.signed_by.clone();
        }
    }

    lines.push(format!("{prefix}Types: {}", types.join(" ")));
    lines.push(format!("{prefix}URIs: {}", uris.join(" ")));
    lines.push(format!("{prefix}Suites: {}", suites.join(" ")));
    if !components.is_empty() {
        lines.push(format!("{prefix}Components: {}", components.join(" ")));
    }
    if !signed_by.is_empty() {
        lines.push(format!("{prefix}Signed-By: {signed_by}"));
    }
    lines.push(String::new());
}

fn determine_file_for_repo(repo: &Repo) -> String {
    if repo.uri.contains("soplos") {
        return format!("{SOURCES_PARTS}/soplos.list");
    }
    let domain = repo.uri.split("://").last().unwrap_or("").split('/').next().unwrap_or("custom");
    let name = format!("{domain}-{}", repo.distribution).replace('.', "_").replace('/', "-");
    format!("{SOURCES_PARTS}/{name}.list")
}

/// Resolves a GPG key path, same order as 2.x's RepoManager._find_gpg_key:
/// exact path, basename in the standard dirs, full-hostname substring match
/// (must run before the keyword fallback — a generic keyword like "debian"
/// would otherwise match an unrelated official Debian keyring before the
/// real per-repo key), then a loose keyword-from-URI fallback.
fn find_gpg_key(signed_by: &str, uri: &str) -> String {
    if !signed_by.is_empty() && Path::new(signed_by).exists() {
        return signed_by.to_string();
    }
    if !signed_by.is_empty() {
        let basename = Path::new(signed_by).file_name().and_then(|n| n.to_str()).unwrap_or("");
        for dir in GPG_DIRS {
            let candidate = Path::new(dir).join(basename);
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }

    if uri.is_empty() {
        return signed_by.to_string();
    }
    let after_scheme = uri.split("://").last().unwrap_or(uri);
    let hostname = after_scheme.split('/').next().unwrap_or("").to_lowercase();

    if !hostname.is_empty() {
        for dir in GPG_DIRS {
            let Ok(mut entries) = std::fs::read_dir(dir).map(|e| e.flatten().collect::<Vec<_>>()) else { continue };
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if (name.ends_with(".gpg") || name.ends_with(".asc")) && name.contains(&hostname) {
                    return entry.path().to_string_lossy().to_string();
                }
            }
        }
    }

    const GENERIC: [&str; 15] = [
        "www", "dl", "download", "apt", "deb", "debian", "linux", "stable", "main", "archive", "repo", "release", "com", "org", "net",
    ];
    let keywords: Vec<String> = after_scheme
        .replace('/', ".")
        .split('.')
        .filter(|p| !p.is_empty() && p.len() > 2 && !GENERIC.contains(&p.to_lowercase().as_str()))
        .map(|p| p.to_lowercase())
        .collect();

    if !keywords.is_empty() {
        for dir in GPG_DIRS {
            let Ok(mut entries) = std::fs::read_dir(dir).map(|e| e.flatten().collect::<Vec<_>>()) else { continue };
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if (name.ends_with(".gpg") || name.ends_with(".asc")) && keywords.iter().any(|kw| name.contains(kw)) {
                    return entry.path().to_string_lossy().to_string();
                }
            }
        }
    }

    signed_by.to_string()
}

pub enum BatchTask {
    WriteFile { path: String, content: String, mode: u32 },
    DeleteFile { path: String },
    RunCommand { command: String },
}

/// Builds one `/bin/sh` script covering every task and runs it as root
/// through the polkit-gated privilege::run_elevated(), so a compound
/// operation (save multiple repo files, generate sources, run apt-get
/// update) prompts for the password once instead of per file — same
/// pattern as 2.x's RepoFileManager.run_batch_operation.
fn run_batch_operation(tasks: Vec<BatchTask>) -> Result<(), String> {
    if tasks.is_empty() {
        return Ok(());
    }

    let mut script = String::from("#!/bin/sh\nset -e\n");
    let mut temp_files = Vec::new();

    for task in &tasks {
        match task {
            BatchTask::WriteFile { path, content, mode } => {
                let mut tmp = tempfile_path("tmp");
                let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
                f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
                let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644));
                let dest_dir = Path::new(path).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                script.push_str(&format!("mkdir -p '{dest_dir}'\n"));
                script.push_str(&format!("mv '{}' '{path}'\n", tmp.display()));
                script.push_str(&format!("chmod {:o} '{path}'\n", mode & 0o777));
                temp_files.push(std::mem::take(&mut tmp));
            }
            BatchTask::DeleteFile { path } => {
                script.push_str(&format!("rm -f '{path}'\n"));
            }
            BatchTask::RunCommand { command } => {
                script.push_str(command);
                script.push('\n');
            }
        }
    }

    let result = crate::privilege::run_elevated(&script);
    for t in &temp_files {
        let _ = std::fs::remove_file(t);
    }
    result
}

fn tempfile_path(suffix: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("soplos-repo-selector-{pid}-{nanos}.{suffix}"))
}

fn get_all_repos_blocking() -> Vec<Repo> {
    let mut repos = Vec::new();
    let sources_list = Path::new(SOURCES_LIST);
    if sources_list.exists() {
        repos.extend(read_sources_file(sources_list));
    }
    let parts = Path::new(SOURCES_PARTS);
    if let Ok(entries) = std::fs::read_dir(parts) {
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                ext == "list" || ext == "sources"
            })
            .collect();
        paths.sort();
        for path in paths {
            repos.extend(read_sources_file(&path));
        }
    }
    repos
}

#[tauri::command]
pub async fn list_repos() -> Vec<Repo> {
    tauri::async_runtime::spawn_blocking(get_all_repos_blocking).await.unwrap_or_default()
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn save_repos(repos: Vec<Repo>, filesToDelete: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut repos_by_file: HashMap<String, Vec<Repo>> = HashMap::new();
        for mut repo in repos {
            if repo.file.is_empty() {
                repo.file = determine_file_for_repo(&repo);
            }
            repos_by_file.entry(repo.file.clone()).or_default().push(repo);
        }

        let mut tasks = Vec::new();
        for (path, file_repos) in &repos_by_file {
            let use_deb822 = path.ends_with(".sources") || file_repos.iter().any(|r| r.format == "deb822");
            let content = if use_deb822 { generate_deb822_content(file_repos) } else { generate_legacy_content(file_repos) };
            let mode = std::fs::metadata(path).map(|m| m.permissions().mode()).unwrap_or(0o644);
            tasks.push(BatchTask::WriteFile { path: path.clone(), content, mode });
        }
        for f in filesToDelete {
            if !repos_by_file.contains_key(&f) {
                tasks.push(BatchTask::DeleteFile { path: f });
            }
        }

        run_batch_operation(tasks)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Deserialize)]
pub struct GenerateSourcesInput {
    pub distros: Vec<String>,
    pub components: Vec<String>,
    pub backports: bool,
    #[serde(rename = "debSrc")]
    pub deb_src: bool,
    pub mirror: String,
}

const DEBIAN_KEYRING: &str = "/usr/share/keyrings/debian-archive-keyring.gpg";
const SECURITY_URI: &str = "https://security.debian.org/debian-security/";

fn deb822_file(types: &str, uris: &str, suites: &str, components: &str, signed_by: &str) -> String {
    let mut lines = vec![GENERATED_BY_HEADER.to_string(), FORMAT_DEB822_HEADER.to_string(), String::new()];
    lines.push(format!("Types: {types}"));
    lines.push(format!("URIs: {uris}"));
    lines.push(format!("Suites: {suites}"));
    if !components.is_empty() {
        lines.push(format!("Components: {components}"));
    }
    lines.push(format!("Signed-By: {signed_by}"));
    lines.push(String::new());
    lines.join("\n")
}

/// Generates one independent `.sources` file per suite (updates/security/
/// proposed-updates/backports each get their own file too), matching 2.x's
/// Sources Generator exactly: Types/URIs/Suites/Components/Signed-By per
/// file, pointing non-rolling suites at security.debian.org for the
/// security line, and sid/unstable/experimental never get -updates or
/// -security companions since those suites don't have them.
#[tauri::command]
pub async fn generate_sources(input: GenerateSourcesInput) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let types = if input.deb_src { "deb deb-src" } else { "deb" };
        let components = input.components.join(" ");
        let mut new_files: HashMap<String, String> = HashMap::new();

        const ROLLING: [&str; 3] = ["sid", "unstable", "experimental"];

        for distro in &input.distros {
            let base_name = format!("{SOURCES_PARTS}/debian-{distro}.sources");
            new_files.insert(base_name, deb822_file(types, &input.mirror, distro, &components, DEBIAN_KEYRING));

            if !ROLLING.contains(&distro.as_str()) {
                let updates_name = format!("{SOURCES_PARTS}/debian-{distro}-updates.sources");
                new_files.insert(updates_name, deb822_file(types, &input.mirror, &format!("{distro}-updates"), &components, DEBIAN_KEYRING));

                let security_name = format!("{SOURCES_PARTS}/debian-{distro}-security.sources");
                new_files.insert(security_name, deb822_file(types, SECURITY_URI, &format!("{distro}-security"), &components, DEBIAN_KEYRING));
            }

            if distro == "testing" {
                let proposed_name = format!("{SOURCES_PARTS}/debian-{distro}-proposed-updates.sources");
                new_files.insert(proposed_name, deb822_file(types, &input.mirror, &format!("{distro}-proposed-updates"), &components, DEBIAN_KEYRING));
            }

            if input.backports {
                let backports_name = format!("{SOURCES_PARTS}/debian-{distro}-backports.sources");
                new_files.insert(backports_name, deb822_file(types, &input.mirror, &format!("{distro}-backports"), &components, DEBIAN_KEYRING));
            }
        }

        // Clean up any previously generated debian-*.sources that the new
        // selection no longer covers.
        let mut to_delete = Vec::new();
        if let Ok(entries) = std::fs::read_dir(SOURCES_PARTS) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if name.starts_with("debian-") && name.ends_with(".sources") {
                    let path_str = path.to_string_lossy().to_string();
                    if !new_files.contains_key(&path_str) {
                        to_delete.push(path_str);
                    }
                }
            }
        }

        let mut tasks: Vec<BatchTask> = new_files.into_iter().map(|(path, content)| BatchTask::WriteFile { path, content, mode: 0o644 }).collect();
        for path in to_delete {
            tasks.push(BatchTask::DeleteFile { path });
        }
        tasks.push(BatchTask::RunCommand { command: "apt-get clean".to_string() });
        tasks.push(BatchTask::RunCommand { command: "apt-get autoclean".to_string() });
        tasks.push(BatchTask::RunCommand { command: "apt-get update".to_string() });

        run_batch_operation(tasks)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn modernize_repo(repo: Repo) -> Result<Repo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !repo.file.ends_with(".list") {
            return Err("Not a legacy .list repository".to_string());
        }
        let new_file = repo.file.trim_end_matches(".list").to_string() + ".sources";

        let mut new_repo = repo.clone();
        new_repo.format = "deb822".to_string();
        new_repo.file = new_file.clone();
        let resolved = find_gpg_key(&new_repo.signed_by, &new_repo.uri);
        if !resolved.is_empty() {
            new_repo.signed_by = resolved;
        }

        let backup = format!("{}.bak", repo.file);
        let content = generate_deb822_content(std::slice::from_ref(&new_repo));

        let tasks = vec![
            BatchTask::RunCommand { command: format!("cp '{}' '{backup}'", repo.file) },
            BatchTask::WriteFile { path: new_file, content, mode: 0o644 },
            BatchTask::DeleteFile { path: repo.file.clone() },
        ];

        run_batch_operation(tasks)?;
        Ok(new_repo)
    })
    .await
    .map_err(|e| e.to_string())?
}
