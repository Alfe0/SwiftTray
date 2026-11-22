# SwiftTray MSIX packaging (Microsoft Store)

The current Store package identity is preserved from the certified 1.0.0.0 package.
This folder now contains the **build script**, the **manifest template**, and the
existing checked-in brand assets in `Assets/` (kept unchanged).

## Build locally on Windows

Prerequisites: Rust MSVC toolchain, Windows 10/11 SDK (makepri and makeappx),
and the project source. From the repository root:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\msix\Build-MSIX.ps1 -RepoRoot .
```

The script runs Rust tests and compiles SwiftTray, then stages and creates an **unsigned** MSIX at:

`packaging/msix/Output/MohammedAlfehaid.SwiftTray_<version>.0_x64.msix`

`Cargo.toml` is the only source for release versioning (the package version is
`major.minor.patch.0`). Update `Cargo.toml` and `Cargo.lock` together.

## GitHub Actions

The Windows job builds and tests the portable EXE and then invokes the same
MSIX script with `-SkipCargo` (to reuse the newly built EXE). It uploads an
**unsigned MSIX artifact** for release review; it does not publish this file
as a GitHub Release asset and does not submit to Microsoft Store.

The CI pipeline embeds `SWIFTTRAY_GITHUB_REPO` for manual version checks, and
local builds now use `Alfe0/SwiftTray` too. GitHub version checks do **not**
replace Store-managed updates.

## Important identity / signing notes

- The package **Name, Publisher, Application Id, x64 architecture, full-trust
  entry point and executable path** match the original Store package.
- Do not commit `Output`, any `.msix` outputs, or development certificates.
- Do not distribute unsigned MSIX files directly to users; Microsoft Store
  signs packages it distributes.
- The sandbox self-signed test package and `.cer` are separate test-only files.
  Do not trust the test certificate on a production Windows installation.
- `Start with Windows` and update-in-place from the old MSIX still require
  separate tests. A clean Sandbox install is not proof of in-place update.
- Keep the current Partner Center submission unchanged until the next release
  procedure is agreed upon.
- Before uploading to the Store, review Windows App Certification Kit results.

## Previously existing assets

`packaging/msix/Assets/` remains the single source for MSIX icon images.
The `assets/SwiftTray.ico` and `assets/SwiftTray-tray.ico` resources are
separate, optimized variants of the approved unified brand mark.

Useful docs:
- https://learn.microsoft.com/windows/msix/package/create-app-package-with-makeappx-tool
- https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/upload-app-packages
