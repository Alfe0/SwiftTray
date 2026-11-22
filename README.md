<p align="center">
  <img src="assets/SwiftTray-logo.svg" alt="SwiftTray logo" width="128">
</p>

<h1 align="center">SwiftTray</h1>

<p align="center">
  A lightweight, native audio utility for Windows 11, built with Rust.
</p>

<p align="center">
  <a href="https://github.com/Alfe0/SwiftTray/releases/latest">Download</a> ·
  <a href="CHANGELOG.md">Changelog</a> ·
  <a href="LICENSE">MIT License</a>
</p>

SwiftTray keeps essential audio controls within easy reach of the Windows system tray. Adjust individual application volumes, switch between selected output devices, and open Windows screen capture—all from a compact native interface.

**Source version:** `v1.0.1`  
**Latest published download:** [GitHub Releases](https://github.com/Alfe0/SwiftTray/releases/latest)

> **Microsoft Store:** SwiftTray v1.0.1 has been submitted for certification. The Store listing will be available after certification and the developer's publication approval. Until then, use the GitHub Releases page for publicly available downloads.

## Features

- **Per-app audio mixer** — Adjust and mute individual application volumes from a compact flyout, with live activity indicators.
- **Audio output switching** — Choose which output devices participate in switching, then cycle between them with a customizable shortcut.
- **Global keyboard shortcuts** — Configure shortcuts to open SwiftTray, switch output devices, or invoke Windows screen capture.
- **System tray integration** — Keep audio controls available without a large application window.
- **Windows 11 design** — A dark, compact native interface with SwiftTray gradient or Windows accent color options.
- **Personal preferences** — Configure device aliases, output notifications, shortcuts, and optional launch at sign-in.
- **Small native application** — Built with Rust and Win32/GDI+, without an Electron or WebView-based interface.
- **Open source** — Released under the MIT License.

## What's new in v1.0.1

- Unified the app icon, About page artwork, and notification-area icon around a single interlocking-ribbon SwiftTray symbol.
- Refined the icon resources for different Windows display sizes, including a dedicated tray icon tuned for small sizes.
- Added a reproducible MSIX packaging workflow and Windows build artifacts in GitHub Actions.
- Kept the existing audio controls, keyboard shortcuts, settings, and user preferences intact.

The Windows application and Microsoft Store listing now share the same visual identity. The Microsoft Store package is distributed through the Store after approval; **unsigned MSIX files produced by CI are build artifacts, not direct-install releases**.

## Download and run

1. Open [GitHub Releases](https://github.com/Alfe0/SwiftTray/releases/latest).
2. Download `SwiftTray.exe` from the release assets, if provided for that release.
3. Keep the executable in a permanent folder, then run it on Windows 11.

Rust, Visual Studio, and `BUILD.bat` are **not** required to run a published executable.

**Note:** An unsigned executable may trigger a Windows SmartScreen warning. Confirm that you downloaded it from this repository's official release page and verify the published checksum when available. A warning is not proof that a file is safe or unsafe.

## Build from source

**Requirements:** Windows with the Rust MSVC x64 toolchain, Visual Studio C++ Build Tools, and Windows SDK.

Use `BUILD.bat`, or run:

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo test --release
cargo build --release
```

`BUILD.bat` copies the built executable to `dist\SwiftTray.exe`.

### MSIX packaging

The repository includes the Windows MSIX manifest and packaging script under `packaging/msix/`:

```powershell
./packaging/msix/Build-MSIX.ps1 -RepoRoot (Get-Location).Path
```

The script uses the Windows SDK packaging tools to create a package for submission/testing. GitHub Actions also builds an **unsigned** MSIX artifact. Do not distribute an unsigned MSIX as an ordinary end-user installer; Store distribution and signing are handled through the Microsoft Store submission process.

See [`packaging/msix/README.md`](packaging/msix/README.md) for packaging details.

## Manual update checks

Selecting **Check for updates** queries the latest published, non-prerelease GitHub Release. SwiftTray does not silently install or download updates: it offers a link for the user to review the release.

Official GitHub Actions builds embed this repository address for manual update checks. For a local build, set `SWIFTTRAY_GITHUB_REPO` before building:

```cmd
set SWIFTTRAY_GITHUB_REPO=Alfe0/SwiftTray
BUILD.bat
```

Builds without that setting may display `Update source not configured` when the user selects **Check for updates**.

## Configuration

Settings are stored in `%LOCALAPPDATA%\SwiftTray\settings.json`. A `settings.json.bak` recovery copy may be retained. Existing preferences, including user-selected shortcuts, are preserved across upgrades.

**Launch at startup** is optional. For the portable executable, startup registration points to the executable's current path; if the file is moved, disable and re-enable that setting.

## Compatibility and known limitations

- **Target platform:** Windows 11 x64. ARM64 and earlier Windows versions have not been fully validated.
- Audio output switching uses the Windows `IPolicyConfig` COM interface, which is undocumented and may change in future Windows releases.
- Screen capture invokes the Windows `Win + Shift + S` behavior. Elevated or protected windows may restrict simulated keyboard input.
- Audio sessions can appear or disappear as applications start, stop, or produce sound. The activity indicator is an estimate, not a calibrated loudness measurement.
- The selected Windows accent affects supported controls; SwiftTray does not switch to a separate light UI simply because Windows is in Light mode.
- No administrator rights, Windows service, or third-party audio utility are required for normal operation.

## Project and license

- [Changelog](CHANGELOG.md)
- [Windows release checklist](RELEASE.md)
- [MIT License](LICENSE)

SwiftTray is an independent, open-source Windows audio utility.
