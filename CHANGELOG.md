# Changelog

All notable changes to Soplos Repo Selector are documented in this file.

## [3.0.0-1-beta] - 2026-10-08

### Added

- GPG Keys: unused-key detection. Each key now shows an "Unused" badge when no configured repository (`/etc/apt/sources.list` or any `/etc/apt/sources.list.d/*.sources`/`.list`, by Signed-By path or bare filename) references it anymore and it doesn't belong to any installed package — the leftover a manually imported key becomes when its repository is removed without also deleting the key. A summary banner shows the total count. System keyrings shipped by packages like debian-archive-keyring, debian-keyring or a third-party repo's own `-keyring` package are never flagged, even when nothing references them directly, since dpkg owns and manages those.

### Changed

- Sidebar logo updated to match the app's new icon.

## [3.0.0-alpha] - 2026-10-03

Full architecture rewrite of Soplos Repo Selector, from Python 3 and GTK3 to Tauri 2 with a Rust backend and a Svelte frontend.
