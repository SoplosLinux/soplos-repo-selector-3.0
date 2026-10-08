// Mirrors the Rust GpgKey/GpgKeyDetails structs in src-tauri/src/commands/gpg.rs.
export interface GpgKey {
  name: string;
  path: string;
  format: string;
  description: string;
  unused: boolean;
}

export interface GpgKeyDetails {
  fingerprint: string;
  expiry: string | null;
  uid: string;
}
