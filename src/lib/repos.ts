// Mirrors the Rust `Repo` struct in src-tauri/src/commands/repos.rs.
export interface Repo {
  disabled: boolean;
  type: string;
  uri: string;
  distribution: string;
  components: string;
  comment: string;
  signedBy: string;
  file: string;
  format: string;
  rawOptions?: string;
  block?: number;
}

export function emptyRepo(): Repo {
  return {
    disabled: false,
    type: "deb",
    uri: "",
    distribution: "",
    components: "main",
    comment: "",
    signedBy: "",
    file: "",
    format: "legacy",
  };
}

export function matchesSearch(repo: Repo, query: string): boolean {
  if (!query) return true;
  const q = query.toLowerCase();
  return (
    repo.uri.toLowerCase().includes(q) ||
    repo.distribution.toLowerCase().includes(q) ||
    repo.components.toLowerCase().includes(q) ||
    repo.comment.toLowerCase().includes(q)
  );
}
