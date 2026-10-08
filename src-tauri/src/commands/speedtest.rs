// Mirror speed test, ported from 2.x's services/speed_test_service.py:
// real TCP latency (a plain connect, not ICMP), a streaming download of a
// file every Debian mirror has (stopping once enough data/time has been
// sampled), 6 parallel workers, and a combined latency/speed score for
// ranking. The mirror list, hostname->country table and ISO-code fallback
// table live in mirrors_data.rs (ported verbatim as data, not logic).

use crate::mirrors_data::{COUNTRY_CODES, COUNTRY_MAP, DEFAULT_MIRRORS, REGION_MIRRORS};
use serde::Serialize;
use std::collections::VecDeque;
use std::io::Read;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(600);
const READ_TIMEOUT: Duration = Duration::from_millis(1000);
const MIN_DOWNLOAD_BYTES: u64 = 512 * 1024;
const MIN_DOWNLOAD_TIME: f64 = 0.8;
const MAX_DOWNLOAD_BYTES: u64 = 3 * 1024 * 1024;
const TEST_FILES: [&str; 2] = ["dists/stable/Release", "ls-lR.gz"];

#[derive(Serialize, Clone)]
pub struct MirrorResult {
    pub url: String,
    pub country: String,
    pub status: String,
    #[serde(rename = "speedMbps")]
    pub speed_mbps: f64,
    #[serde(rename = "latencyMs")]
    pub latency_ms: f64,
    #[serde(rename = "downloadedBytes")]
    pub downloaded_bytes: u64,
    pub progress: f64,
}

pub fn get_mirror_country(url: &str) -> String {
    let hostname = url.split("://").last().unwrap_or(url).split('/').next().unwrap_or("").to_lowercase();

    for (domain, country) in COUNTRY_MAP {
        if hostname.contains(domain) {
            return country.to_string();
        }
    }

    if hostname.contains("debian.org") {
        let parts: Vec<&str> = hostname.split('.').collect();
        if parts.len() >= 3 && parts[0] == "ftp" {
            let code = parts[1].to_lowercase();
            if let Some((_, country)) = COUNTRY_CODES.iter().find(|(c, _)| *c == code) {
                return country.to_string();
            }
            if code.len() == 2 {
                return format!("({})", code.to_uppercase());
            }
        }
    }

    "Unknown".to_string()
}

struct DownloadOutcome {
    speed_mbps: f64,
    downloaded_bytes: u64,
}

fn measure_download_speed(url: &str, on_progress: &dyn Fn(f64, f64, u64)) -> Option<DownloadOutcome> {
    for test_file in TEST_FILES {
        let test_url = format!("{}/{test_file}", url.trim_end_matches('/'));

        let response = ureq::get(&test_url)
            .config()
            .timeout_resolve(Some(CONNECT_TIMEOUT))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_body(Some(READ_TIMEOUT))
            .build()
            .call();

        let Ok(resp) = response else { continue };
        if resp.status() != 200 {
            continue;
        }
        let content_length = resp.body().content_length().unwrap_or(0);

        let mut reader = resp.into_body().into_reader();
        let mut buf = [0u8; 32768];
        let mut downloaded: u64 = 0;
        let start = Instant::now();
        let mut last_emit = start;
        let mut speed_mbps = 0.0;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            downloaded += n as u64;
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                speed_mbps = (downloaded as f64 / (1024.0 * 1024.0)) / elapsed;
            }
            let progress = if content_length > 0 {
                ((downloaded as f64 / content_length as f64) * 0.9 + 0.05).min(0.95)
            } else {
                ((downloaded as f64 / MIN_DOWNLOAD_BYTES as f64) * 0.9 + 0.05).min(0.95)
            };
            if last_emit.elapsed() >= Duration::from_millis(80) {
                on_progress(speed_mbps, progress, downloaded);
                last_emit = Instant::now();
            }
            if downloaded >= MIN_DOWNLOAD_BYTES && elapsed >= MIN_DOWNLOAD_TIME {
                break;
            }
            if downloaded >= MAX_DOWNLOAD_BYTES || elapsed >= 1.5 {
                break;
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        if elapsed > 0.0 && downloaded > 0 {
            speed_mbps = (downloaded as f64 / (1024.0 * 1024.0)) / elapsed;
            return Some(DownloadOutcome { speed_mbps, downloaded_bytes: downloaded });
        }
    }

    None
}

fn sort_key(r: &MirrorResult) -> (u8, f64) {
    let latency_score = r.latency_ms / 1000.0;
    let speed_score = 100.0 / r.speed_mbps.max(0.01);
    (0, latency_score * 0.3 + speed_score * 0.7)
}

const SCAN_BUDGET: Duration = Duration::from_secs(5);
const MAX_WORKERS: usize = 15;

fn url_host_port(url: &str) -> (String, u16) {
    let after_scheme = url.split("://").last().unwrap_or(url);
    let host_part = after_scheme.split('/').next().unwrap_or("");
    match host_part.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(80)),
        None => (host_part.to_string(), 80),
    }
}

#[tauri::command]
pub async fn test_mirrors(app: tauri::AppHandle) -> Vec<MirrorResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let queue: Arc<Mutex<VecDeque<&'static str>>> = Arc::new(Mutex::new(DEFAULT_MIRRORS.iter().copied().collect()));
        let results: Arc<Mutex<Vec<MirrorResult>>> = Arc::new(Mutex::new(Vec::new()));
        let deadline = Instant::now() + SCAN_BUDGET;

        std::thread::scope(|scope| {
            for _ in 0..MAX_WORKERS {
                let queue = Arc::clone(&queue);
                let results = Arc::clone(&results);
                let app = app.clone();

                scope.spawn(move || loop {
                    if Instant::now() >= deadline {
                        break;
                    }
                    let next = { queue.lock().unwrap().pop_front() };
                    let Some(url) = next else { break };
                    let country = get_mirror_country(url);

                    let (hostname, port) = url_host_port(url);
                    let start_tcp = Instant::now();

                    let latency = match (hostname.as_str(), port).to_socket_addrs() {
                        Ok(mut addrs) => match addrs.next() {
                            Some(addr) => match std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
                                Ok(_) => (start_tcp.elapsed().as_secs_f64() * 1000.0 * 10.0).round() / 10.0,
                                Err(_) => -1.0,
                            },
                            None => -1.0,
                        },
                        Err(_) => -1.0,
                    };

                    if latency < 0.0 {
                        continue;
                    }

                    let app_for_progress = app.clone();
                    let url_owned = url.to_string();
                    let country_for_progress = country.clone();

                    let outcome = measure_download_speed(url, &|speed_mbps, progress, downloaded| {
                        let _ = app_for_progress.emit(
                            "speedtest-progress",
                            MirrorResult {
                                url: url_owned.clone(),
                                country: country_for_progress.clone(),
                                status: "downloading".to_string(),
                                speed_mbps,
                                latency_ms: latency,
                                downloaded_bytes: downloaded,
                                progress,
                            },
                        );
                    });

                    let Some(outcome) = outcome else { continue };

                    let res = MirrorResult {
                        url: url.to_string(),
                        country,
                        status: "success".to_string(),
                        speed_mbps: outcome.speed_mbps,
                        latency_ms: latency,
                        downloaded_bytes: outcome.downloaded_bytes,
                        progress: 1.0,
                    };
                    let _ = app.emit("speedtest-progress", res.clone());
                    results.lock().unwrap().push(res);
                });
            }
        });

        let mut all_results = Arc::try_unwrap(results).map(|m| m.into_inner().unwrap()).unwrap_or_default();
        all_results.sort_by(|a, b| sort_key(a).partial_cmp(&sort_key(b)).unwrap_or(std::cmp::Ordering::Equal));
        all_results
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub fn get_mirror_count() -> usize {
    DEFAULT_MIRRORS.len()
}

#[tauri::command]
pub fn get_region_mirrors() -> Vec<(String, Vec<String>)> {
    REGION_MIRRORS.iter().map(|(region, urls)| (region.to_string(), urls.iter().map(|u| u.to_string()).collect())).collect()
}
