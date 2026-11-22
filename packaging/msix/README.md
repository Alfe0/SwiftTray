# SwiftTray — MSIX icon assets (source of truth: official SVG)

This folder contains **visual assets only** generated from `assets/SwiftTray-logo.svg`.
It is **not** a complete MSIX project: the submitted package's `AppxManifest.xml`,
original Store identity, signing settings, installer and `resources.pri` are not
included in the GitHub source ZIP. Do not invent or replace those values.

## What changed

- `assets/SwiftTray.ico` (resource ID **1**) now shows the same four-panel
  official logo at **every** resolution, including 16–40px, so Windows Explorer,
  desktop shortcuts and the executable no longer change to a tray design.
- `assets/SwiftTray-tray.ico` (resource ID **2**) is the **approved** green→teal→blue
  asymmetric rounded-square tile with a white speaker + two arcs; **no blue knob**.
  Dedicated 16px and 20–24px optical variants preserve legibility in the notification area.
  The tray image is deliberately not used for Start, Explorer, MSIX app identity or the taskbar.
- This `Assets` folder provides app-list/Start/Taskbar `Square44x44Logo`,
  `Square150x150Logo`, `StoreLogo`, scale variants and 16–256px `targetsize`
  variants (with `altform-unplated` counterparts).
- The About artwork still comes from the unmodified official SVG.

## Integrating into the existing MSIX packaging project

1. Keep the **existing** Partner Center Store package identity: `Identity Name`,
   `Publisher`, `Application Id`, package signing, and app capabilities.
2. In the *actual* MSIX project, find `AppxManifest.xml` / `Package.appxmanifest`
   and identify the resource paths under `Properties > Logo` and
   `Applications > Application > uap:VisualElements`.
3. Copy or rename the PNG assets into **that exact existing asset directory**.
   If it uses new names, update `Logo`, `Square44x44Logo` and
   `Square150x150Logo` paths to match. Do not assume these filenames already
   match the previously submitted MSIX.
4. Package the **newly rebuilt** executable so its main EXE icon uses resource
   ID 1 and the notification area uses resource ID 2.
5. Rebuild the package's resource index (`resources.pri`) if your packaging tool
   requires it; Microsoft's manual flow uses `makepri.exe` after asset changes.
6. Build and test in Windows 11 before changing the approved Store submission.
   Check the taskbar, Start/Search, desktop shortcut, Explorer, notification
   area and About on 100%, 125%, 150% and 200% DPI and both themes.
7. After successful testing, prepare a **new Store submission** with the
   correct package version and updated listing pictures where necessary. The
   approved-but-unpublished submission should not be canceled prematurely.

No changes to volume/audio handling, hotkeys, app configuration or startup
behavior are required for this icon correction.

Regenerate (designer machine only): `python tools/generate_icons.py`

Docs: https://learn.microsoft.com/windows/msix/desktop/desktop-to-uwp-manual-conversion
