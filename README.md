# Soplos Repo Selector 3.0

![Version](https://img.shields.io/badge/version-3.0.0--3--beta-blue)
![License](https://img.shields.io/badge/license-GPL--3.0--or--later-green)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)
![Tauri](https://img.shields.io/badge/Tauri-2-orange)
![Rust](https://img.shields.io/badge/Rust-1.80+-orange)
![Svelte](https://img.shields.io/badge/Svelte-4-red)

Advanced APT repository manager for Soplos Linux.

Enable or disable repositories, modernize legacy `.list` files to DEB822 `.sources`, find the fastest mirror with a real speed test, and manage GPG signing keys — including flagging keys no repository uses anymore.

## Screenshots

| | |
|---|---|
| ![Repositories](src-tauri/resources/screenshots/screenshot01.png) **Repositories**: manage and toggle software archives per source | ![Sources Generator](src-tauri/resources/screenshots/screenshot02.png) **Sources Generator**: distribution, components and mirror selector |
| ![Mirror speed test](src-tauri/resources/screenshots/screenshot03.png) **Speed test**: live throughput benchmark across worldwide Debian mirrors | ![GPG Keys](src-tauri/resources/screenshots/screenshot04.png) **GPG Keys**: trusted keyrings, fingerprint details and unused key detection |

## Architecture

Version 3.0 is a ground-up rewrite of Soplos Repo Selector. The original app was written in Python 3 with GTK3 (PyGObject). This rewrite replaces that entire stack with Tauri 2: a Rust backend and a Svelte 4 + Tailwind CSS frontend, rendered through the system's WebKitGTK.

- **Backend** (`src-tauri/`): privileged operations (writing repo files, modernizing `.list` to `.sources`, importing or deleting GPG keys, creating `/etc/apt/keyrings`) run through a single polkit-gated helper (`src-tauri/src/privilege/`), authorized by `resources/polkit/org.soplos.reposelector.policy`. The mirror speed test streams actual package index downloads to measure real network throughput rather than raw ping latency.
- **Frontend** (`src/`): three core views — Repositories, Sources Generator and GPG Keys — shared dialogs, and the 8-language dictionaries under `src/i18n/`.
- **Privileges**: root actions require authentication through Polkit; the GUI itself never runs as the superuser.

## Installation

```bash
sudo apt install soplos-repo-selector
```

## Features

- **Repository management**: inspect, enable, and disable every configured APT repository.
- **DEB822 modernization**: convert legacy `/etc/apt/sources.list` and `.list` entries to Debian's modern DEB822 `.sources` format.
- **Sources Generator**: pick the Debian versions (with optional Backports), the components (`main`, `contrib`, `non-free`, `non-free-firmware`), whether to include source packages (`deb-src`) and the mirror. It starts prefilled from your current repositories, writes the repository files and runs `apt update`.
- **Live mirror benchmark**: real-time streaming download tests to detect the fastest official Debian mirrors, from the Sources Generator.
- **Keyring management**: import, export, inspect, and remove GPG signing keys with full fingerprint and expiry details.
- **Unused key detection**: automatically flags imported keys that are no longer referenced by any active repository and do not belong to an installed system package.

## Dependencies

- `pkexec` (Polkit) for every change to the system, and `gnupg` for the keys. Both are installed with the package.
- `curl` and `wget` are recommended.

## Where it writes

| Path | Content |
|---|---|
| `/etc/apt/sources.list` | Main repository file |
| `/etc/apt/sources.list.d/` | Modular repository files, in `.sources` and `.list` format |
| `/etc/apt/keyrings/` | Repository signing keys. Created by the program when missing |

Nothing is written without administrator authentication, and the window itself never runs as root.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| F1 | Open About dialog |
| Ctrl+Q | Quit application |
| Ctrl+Tab | Next section |
| Ctrl+Shift+Tab | Previous section |

## Documentation

A Unix manual page is included and installed with the package:

```bash
man soplos-repo-selector
```

## Languages

The interface is available in 8 languages: English, Español, Deutsch, Français, Italiano, Português, Română and Русский. It starts in the system language and falls back to English.

## Building

```bash
npm install
npm run tauri dev      # development
./package.sh           # builds the .deb and moves it to ~/Descargas/SoplosApps
```

## License

Copyright © 2026 Sergi Perich <info@soploslinux.com>. Published under the GPL-3.0-or-later license. See `LICENSE`.

