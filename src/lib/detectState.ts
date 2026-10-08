// Ported from 2.x's sources_generator_view.py _detect_system_state (lines
// 42-104): reads both enabled and disabled repos already on disk and
// derives which known distros/components/backports/deb-src are configured,
// so the Sources Generator opens prefilled with the real system state
// instead of a hardcoded guess. Only repos pointing at debian.org mirrors
// are considered — third-party repos (Chrome, Docker...) never match.
import type { Repo } from "./repos";
import { KNOWN_DISTROS } from "./speedtest";

export interface DetectedState {
  distros: Set<string>;
  components: Set<string>;
  backports: boolean;
  debSrc: boolean;
  mirror: string | null;
}

function isDebianMirror(uri: string): boolean {
  return uri.includes("debian.org") || uri.includes("deb.debian.org") || uri.includes("security.debian.org") || uri.endsWith("/debian") || uri.endsWith("/debian-security");
}

export function detectSystemState(repos: Repo[]): DetectedState {
  const distros = new Set<string>();
  const components = new Set<string>();
  let backports = false;
  let debSrc = false;
  let mirror: string | null = null;

  for (const repo of repos) {
    if (!isDebianMirror(repo.uri)) continue;

    const suite = repo.distribution;
    const baseSuite = (KNOWN_DISTROS as readonly string[]).find((d) => suite === d);
    if (baseSuite) {
      distros.add(baseSuite);
      for (const c of repo.components.split(/\s+/).filter(Boolean)) components.add(c);
      if (!mirror && !repo.uri.includes("security.debian.org")) mirror = repo.uri;
    }

    if (suite.endsWith("-backports")) backports = true;
    if (repo.type === "deb-src") debSrc = true;
  }

  return { distros, components, backports, debSrc, mirror };
}
