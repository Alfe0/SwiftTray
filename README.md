# SwiftTray

A lightweight, native Windows 11 audio flyout built in Rust. Open the per-app
volume mixer, switch between enabled output devices, and trigger the Windows
screen-capture overlay without opening a large application window.

**Current status: v1.0.0 release candidate (source).** Build and validate on
Windows 11 before creating a public `v1.0.0` tag. This archive is not a
precompiled or signed Windows release.

## v1.0.0 release candidate

- Fix colored/pixelated outlines around the four-part logo in About by
  **bypassing Windows HICON conversion for that surface**. A direct 256px
  premultiplied-alpha bitmap is generated from the original vector artwork,
  embedded in the EXE and downscaled with GDI+. A normal native HICON remains
  as fallback if GDI+ cannot initialize.
- Keep the already-accepted 16/20/24px notification-area artwork, tray hover
  tooltip, default SwiftTray gradient, Windows Accent option and app icons.
- No behavior changes to the audio engine, screen snip or global hotkeys.
- The release pipeline runs Rust tests and produces one `SwiftTray.exe` plus an
  SHA-256 checksum; official CI builds embed their GitHub repository address
  for manual update checks.
- See `RELEASE.md` for acceptance tests. **Do not call this stable solely
  because the version field is 1.0.0.**

## Features

- Preview identity: user-provided SwiftTray v3 vector logo and a purpose-drawn
  compact tray variant. `SwiftTray.ico` includes independently rasterized
  transparent PNG frames for 16, 20, 24, 32, 40, 48, 64, 96, 128 and 256px.
  Smaller sizes use the simplified speaker+control-circle symbol; larger sizes
  use the complete four-panel mark. The About page uses the large mark.
- New default brand color tokens come from `assets/SwiftTray-logo.svg`, with
  four colors (#20B27A, #149FA3, #1B78DB, #2756D6) in the slider. Native
  application logos and the red mute color are deliberately unchanged.
- Existing users' preference for Windows accent and their other settings are
  preserved. Windows accent mode does not acquire the brand gradient.

- Per-app volume, mute, high-resolution icons sourced from the local Windows
  Shell/EXE resources, and a separate dashed live-audio activity visualizer scaled to the current volume.
  Muted app icons are softly translucent with the red mute indicator retained.
  The dashed meter is an activity estimate, not calibrated loudness; it stays
  within the volume thumb even during rapid slider changes.
- Toggle among user-selected output devices using a global shortcut.
- A screen-snip shortcut that invokes Windows `Win + Shift + S` behavior.
- Two accent modes: SwiftTray v3 green/teal/blue gradient or the Windows accent color.
- Tray icon, customizable shortcuts, device aliases, output notifications.
- Optional launch-at-login for the current Windows account (off until enabled).
- Native Win32/GDI+ interface; no Electron, WebView2 UI, or web runtime.
- Compact About page, coherent tray menu and manual Check for updates.
- Global shortcut registration automatically retries after temporary conflicts.
- First-run two-key defaults: `Alt + M` (mixer), `Alt + O` (switch),
  `Alt + S` (screen snip). Each can be remapped; saved keys are never replaced
  during upgrades. Some applications may use the same shortcuts, in which case
  SwiftTray reports a conflict and retries registration when available.
- Shortcuts settings use lightweight inline edit actions, not keycap-style boxes.
- Its opaque dark palette stays consistent in Windows Light or Dark mode and
  when Windows animations/transparency are disabled. Contrast Themes instead
  use Windows system colors for legibility.

## Download and run

Once a GitHub Release is published, download **`SwiftTray.exe`** from the
release's Assets section. Keep the EXE in a permanent folder and run it.
**Rust, Visual Studio, and `BUILD.bat` are not required for end users.**
Windows may show a SmartScreen warning for an unsigned new application; do not
interpret this as a guarantee of safety. Verify the source and release first.

## Build from source

Requires Rust (MSVC x64 toolchain) and Visual Studio C++ Build Tools / Windows SDK.
Double-click `BUILD.bat`, or run:

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo test --release
cargo build --release
```

`BUILD.bat` copies the resulting EXE to `dist\SwiftTray.exe`.

### Manual update checks

**Check for updates** uses the Windows WinHTTP library to query the latest
**published, non-prerelease** GitHub Release only when clicked. It does not
install or download files automatically. If a newer version is found, the app
links to its GitHub Release page for the user to review and download.

Official GitHub Actions builds automatically embed the repository address
(`owner/repo`) into the EXE using `SWIFTTRAY_GITHUB_REPO`. Local builds can do
the same by setting that environment variable before running `BUILD.bat`:

```cmd
set SWIFTTRAY_GITHUB_REPO=owner/repo
BUILD.bat
```

Replace `owner/repo` with the actual public GitHub repository. Builds without
this value show a subdued "Update source not configured" status **after the
user clicks** Check for updates; no error is shown by default. Do not configure an unrelated repository.

Windows builds automatically run in GitHub Actions on each push and pull request.
Pushing the version tag `v1.0.0` after validation publishes the executable
as a GitHub Release asset. No repository or release is published by this source ZIP.

## Configuration

User settings are stored in `%LOCALAPPDATA%\SwiftTray\settings.json`.
The app also retains `settings.json.bak` as a recovery copy of the preceding
valid configuration. Existing preferences are retained when upgrading.

The startup setting stores the absolute EXE path in the current user's Windows
Run key. If you move the executable, disable then re-enable **Launch at startup**.

## Compatibility and limitations

- Target: Windows 11 x64. ARM64 and earlier Windows releases are not yet validated.
- Windows Light/Dark mode does not switch SwiftTray to an untested light UI;
  the selected Windows accent affects controls only. Windows Contrast Themes
  take priority and use system UI colors. Check contrast, icon visibility, and
  focus behavior on target machines before a public release.
- "Adjust for best performance" is supported by an opaque GDI/GDI+ surface
  with basic Win32 fallback. Visual-effects settings should not remove text,
  icons, controls, or the window background.
- Audio output switching uses the Windows `IPolicyConfig` COM interface, which is
  **undocumented** and may change in future Windows releases.
- Screen snipping uses simulated `Win + Shift + S` input; elevated/protected
  windows can block this because of Windows input security restrictions.
- The flyout and live peak meters update when the flyout is visible. Inactive
  Windows audio sessions can disappear until the source produces sound again.
- No administrator rights, Windows service, or third-party system utility are required.

## License

[MIT](LICENSE). See [RELEASE.md](RELEASE.md) for the Windows release checklist.

## v1.0.0 release acceptance

Build `SwiftTray.exe` on Windows 11 and verify that the About logo is smooth at
100%, 125%, and 150% scaling, with no turquoise/black outlines or white tile.
Confirm the notification-area icon remains crisp and shows the `SwiftTray`
tooltip. Check mixer sliders, live meter, mute, original app icons, device
switching, screen snip, hotkey conflicts/recovery, startup option, dark/Light
mode, and manual update checking. No user configuration reset is required.
The static source review cannot substitute for this Windows runtime check.
