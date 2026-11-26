# Changelog

## [1.0.2]

- Improve Windows 10 interface-icon compatibility while retaining the Windows 11 experience.
- Preserve stable audio device switching, mixer control, and shortcut behavior; improve output-notification visibility in borderless games.
- Add a user-initiated, GitHub release portable auto-updater with download verification, duplicate-update prevention, staged replacement, rollback/restart and cleanup.
- Direct Microsoft Store installs to Store updates; add StartupTask handling for packaged startup.
- Require version-matched GitHub tags and manually reviewed draft release publication; CI uses locked dependencies.
- Correct the mixer pagination label for one remaining application.

## [1.0.1] — Published 2026-11-22

- Refine the application/tray identity with multi-resolution Windows icons.
- Embed native EXE resources and create reproducible MSIX assets.
- Add Windows CI build artifacts and asset checks.

## [1.0.0] — First public release, 2026-11-20

- Native audio flyout and per-app volume mixer with mute and live activity indicators.
- Output device switching, configurable global shortcuts and Windows screen capture integration.
- Preferences, optional startup registration and system tray controls.

## Pre-1.0 history

Development versions 0.18.x established native icon quality, audio activity display and tray branding. Their historical test notes are preserved separately in the developer archive rather than in the published source release.
