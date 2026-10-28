# Changelog

All notable changes to Soplos Repo Selector are documented in this file.

## [3.0.0-6-beta] - 2026-10-28

### Changed

- New application icon, in the sizes of the package, the window and the interface.
- New hero image, with the circuit style of the other Soplos 3.0 programs.
- New screenshots of every page, taken with the interface in English.

## [3.0.0-5-beta] - 2026-10-24

### Fixed

- The list of mirrors of the speed test has its own scroll inside the page, and when the wheel reached its end it passed to the scroll of the page, so the two fought each other. The list now keeps the wheel to itself (`overscroll-contain`), the same as the results box of the hardware scan of Welcome.

## [3.0.0-4-beta] - 2026-10-23

### Added

- The window follows the light or dark theme of the desktop. It starts in the theme of the desktop and notices when it changes. The theme is read from the XDG portal and, when that says nothing, from each desktop: the colors of `kdeglobals` on Plasma, the name of the GTK theme on Xfce (a guess from words such as "dark") and the color scheme of GNOME. The button of the sidebar still switches the theme and keeps what it chose; switching to the theme the desktop has goes back to following it.

## [3.0.0-3-beta] - 2026-10-22

### Added

- `CHANGELOG.md` is installed as `/usr/share/doc/soplos-repo-selector/changelog.gz`.
- AppStream metainfo: categories and keywords in the eight languages, the same ones as the desktop entry.
- The Debian copyright now declares Lucide (ISC), used for the interface icons.
- README: version, license, platform, Tauri, Rust and Svelte badges. The Sources Generator, one of the three views, is now described among the features, together with new sections on the dependencies (`pkexec`, `gnupg`) and on the files the program writes.

### Changed

- Manual page rewritten: the three views (Repositories, Sources Generator and GPG Keys), the internal `--run-privileged-script` option, keyboard shortcuts and files. It is installed compressed, as `/usr/share/man/man1/soplos-repo-selector.1.gz`.

### Fixed

- AppStream metainfo: the developer and project group identifiers were `org.soploslinux`. They are now `org.soplos`, matching the soplos.org domain and the other Soplos apps.
- AppStream metainfo: the screenshots were bare file names instead of web addresses, so software centers could not show them and the file did not validate. They now use their `raw.githubusercontent.com` address.

## [3.0.0-2-beta] - 2026-10-19

### Added

- Added Unix manual page documentation (`soplos-repo-selector.1`) installed to `/usr/share/man/man1/`.
- Full 8-language translations for desktop entry (`soplos-repo-selector.desktop.hbs`) and AppStream metainfo.
- Updated Debian packaging metadata and bumped project release to `3.0.0-2-beta`.

## [3.0.0-1-beta] - 2026-10-08

### Added

- GPG Keys: unused-key detection. Each key now shows an "Unused" badge when no configured repository (`/etc/apt/sources.list` or any `/etc/apt/sources.list.d/*.sources`/`.list`, by Signed-By path or bare filename) references it anymore and it doesn't belong to any installed package — the leftover a manually imported key becomes when its repository is removed without also deleting the key. A summary banner shows the total count. System keyrings shipped by packages like debian-archive-keyring, debian-keyring or a third-party repo's own `-keyring` package are never flagged, even when nothing references them directly, since dpkg owns and manages those.

### Changed

- Sidebar logo updated to match the app's new icon.

## [3.0.0-alpha] - 2026-10-03

Full architecture rewrite of Soplos Repo Selector, from Python 3 and GTK3 to Tauri 2 with a Rust backend and a Svelte frontend. See the Architecture section of the README for details.

### Added

- Rust backend (`src-tauri/`) replacing the Python 3 and GTK3 codebase. Every change to the system (writing repository files, converting `.list` files to `.sources`, importing or deleting GPG keys, creating `/etc/apt/keyrings`) goes through a single polkit-gated helper; the window itself never runs as root.
- Svelte 4 and Tailwind CSS frontend (`src/`) with three views: Repositories (inspect, enable and disable every configured repository, and convert legacy `.list` files to the DEB822 `.sources` format), Sources Generator (Debian versions, components, source packages and mirror) and GPG Keys (import, export, inspect and remove signing keys, with fingerprint and expiry).
- Live mirror speed test that streams real package index downloads to measure network throughput instead of ping latency.
- Interface in eight languages: English, Spanish, German, French, Italian, Portuguese, Romanian and Russian. It starts in the system language.
