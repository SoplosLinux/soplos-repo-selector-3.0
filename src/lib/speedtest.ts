// Mirrors the Rust MirrorResult struct in src-tauri/src/commands/speedtest.rs.
export interface MirrorResult {
  url: string;
  country: string;
  status: string;
  speedMbps: number;
  latencyMs: number;
  downloadedBytes: number;
  progress: number;
}

// Matches 2.x's _on_speed_test_finished exactly (ui/views/sources_generator_view.py):
// the backend's own combined latency/speed score is only used for its
// internal return order — the UI re-sorts independently, successes by raw
// speed_mbps descending, failures left after them in whatever order they
// came in. This is what actually decides display order and auto-selection,
// not the weighted formula.
export function sortMirrors(results: MirrorResult[]): MirrorResult[] {
  const success = results.filter((r) => r.status === "success").sort((a, b) => b.speedMbps - a.speedMbps);
  const failed = results.filter((r) => r.status !== "success");
  return [...success, ...failed];
}

export const KNOWN_DISTROS = ["trixie", "forky", "testing", "sid", "unstable", "experimental"] as const;
export const KNOWN_COMPONENTS = ["main", "contrib", "non-free", "non-free-firmware"] as const;
