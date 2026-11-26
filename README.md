<p align="center">
  <img src="assets/SwiftTray-logo.svg" alt="SwiftTray logo" width="128">
</p>

<h1 align="center">SwiftTray</h1>

<p align="center">
  A lightweight, native audio utility for Windows 10/11, built with Rust.
</p>

<p align="center">
  <a href="https://github.com/Alfe0/SwiftTray/releases/latest">Download</a> ·
  <a href="CHANGELOG.md">Changelog</a> ·
  <a href="LICENSE">MIT License</a>
</p>

SwiftTray keeps essential audio controls within easy reach of the Windows system tray. Adjust individual application volumes, switch between selected output devices, and open Windows screen capture—all from a compact native interface.

**Source version:** `v1.0.2`  
**Latest published download:** [GitHub Releases](https://github.com/Alfe0/SwiftTray/releases/latest)

> **Microsoft Store:** SwiftTray is available through Microsoft Store. Store installations are updated through Microsoft Store, not by replacing the EXE from GitHub. Production Store updates are distributed via Microsoft Store certification, independently of GitHub Releases.

## Features

- **Per-app audio mixer** — Adjust and mute individual application volumes from a compact flyout, with live activity indicators.
- **Audio output switching** — Choose which output devices participate in switching, then cycle between them with a customizable shortcut.
- **Global keyboard shortcuts** — Configure shortcuts to open SwiftTray, switch output devices, or invoke Windows screen capture.
- **System tray integration** — Keep audio controls available without a large application window.
- **Windows 11 design** — A dark, compact native interface with SwiftTray gradient or Windows accent color options.
- **Personal preferences** — Configure device aliases, output notifications, shortcuts, and optional launch at sign-in.
- **Small native application** — Built with Rust and Win32/GDI+, without an Electron or WebView-based interface.
- **Open source** — Released under the MIT License.

## What's new in v1.0.2

- Corrected Windows 10 icon rendering and validated the core audio flyout on a Windows 10 VM (exact build unrecorded) and Windows 11 on a physical PC.
- Preserved output switching, per-app mixer, and output notifications; improved notification visibility in borderless games.
- Added opt-in GitHub portable self-update flow with pre-download SHA256 checks, rollback and duplicate-installer protection; the release operator must validate live installation/rollback before publishing.
- Routed Store installs to Microsoft Store updates and registered a packaged startup task; the release operator must validate the MSIX on Windows before publishing.
- Tightened GitHub release gates to prevent unreviewed publishing. Fixed the singular "1 more app" label.

Prior v1.0.1 polished the app/tray icon and added MSIX/icon build assets. The Windows application and Microsoft Store listing share the same visual identity. **Unsigned MSIX files produced by CI are build artifacts, not direct-install releases**.

## Download and run

1. Open [GitHub Releases](https://github.com/Alfe0/SwiftTray/releases/latest).
2. Download `SwiftTray.exe` from the release assets, if provided for that release.
3. Keep the executable in a permanent folder, then run it on a supported x64 Windows PC.

Rust, Visual Studio, and `BUILD.bat` are **not** required to run a published executable.

**Note:** An unsigned executable may trigger a Windows SmartScreen warning. Confirm that you downloaded it from this repository's official release page and verify the published checksum when available. A warning is not proof that a file is safe or unsafe.

## Build from source

**Requirements:** Windows with the Rust MSVC x64 toolchain, Visual Studio C++ Build Tools, and Windows SDK.

For a **release candidate** with a configured update source use `./tools/Build-Release-Candidate.ps1` on Windows, or run:

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo test --release --locked
cargo build --release --locked
```

`BUILD.bat` copies the built executable to `dist\SwiftTray.exe`.

### MSIX packaging

The repository includes the Windows MSIX manifest and packaging script under `packaging/msix/`:

```powershell
./packaging/msix/Build-MSIX.ps1 -RepoRoot (Get-Location).Path
```

The script uses the Windows SDK packaging tools to create a package for submission/testing. GitHub Actions also builds an **unsigned** MSIX artifact. Do not distribute an unsigned MSIX as an ordinary end-user installer; Store distribution and signing are handled through the Microsoft Store submission process.

See [`packaging/msix/README.md`](packaging/msix/README.md) for packaging details.

## User-initiated updates

In an **official portable GitHub build**, selecting **Check for updates** queries the latest published stable GitHub Release. If it finds a newer version, SwiftTray downloads that release's EXE and checksum, checks the SHA256 hash, closes itself and attempts to replace/restart its EXE. It keeps a rollback copy. Automatic installation does not occur in the background; the flow still requires controlled release-candidate validation. SHA256 from the same GitHub release does **not** independently authenticate the publisher; signed updates are a future improvement.

**Experimental builds** compiled with `SWIFTTRAY_EXPERIMENTAL_BUILD=1` can query release information but cannot self-install. **Microsoft Store** installations show **Open Store updates** and delegate updates to Microsoft Store.

Official GitHub Actions builds embed this repository address. For a local build, set `SWIFTTRAY_GITHUB_REPO` before building:

```cmd
set SWIFTTRAY_GITHUB_REPO=Alfe0/SwiftTray
BUILD.bat
```

Builds without that setting may display `Update source not configured` when the user selects **Check for updates**. The separate QA archive provides isolated test instructions; ordinary end users should not build or run release tools.

## Configuration

Settings are stored in `%LOCALAPPDATA%\SwiftTray\settings.json`. A `settings.json.bak` recovery copy may be retained. Existing preferences, including user-selected shortcuts, are preserved across upgrades.

**Launch at startup** is optional. For the portable executable, startup registration points to the executable's current path; if the file is moved, disable and re-enable that setting.

## Compatibility and known limitations

- **Target platform:** Windows 11 x64 and Windows 10 x64 (22H2 recommended). Core-function smoke tests on one Windows 10 VM and one Windows 11 PC passed; broad device compatibility and ARM64 have not been validated.
- Audio output switching uses the Windows `IPolicyConfig` COM interface, which is undocumented and may change in future Windows releases.
- Screen capture invokes the Windows `Win + Shift + S` behavior. Elevated or protected windows may restrict simulated keyboard input.
- Audio sessions can appear or disappear as applications start, stop, or produce sound. The activity indicator is an estimate, not a calibrated loudness measurement.
- The selected Windows accent affects supported controls; SwiftTray does not switch to a separate light UI simply because Windows is in Light mode.
- No administrator rights, Windows service, or third-party audio utility are required for normal operation.
- Global shortcuts may be restricted when a higher-integrity program (for example an elevated Task Manager) is foreground.
- Exclusive fullscreen games may minimize when an interactive window or notification is shown; borderless mode behaves differently.
- If Discord microphone audio pauses when switching outputs with input set to Windows Default, select the named input device explicitly in Discord (verified on two devices; not a SwiftTray input-device change).

## Project and license

- [Changelog](CHANGELOG.md)
- [Windows release checklist](RELEASE.md)
- [MIT License](LICENSE)

SwiftTray is an independent, open-source Windows audio utility.
