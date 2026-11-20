# SwiftTray v1.0.0 — Release candidate (test before publishing)

## What changed from v0.18.8

- Render the About logo from a dedicated, transparent, 256px GDI+ PArgb
  bitmap generated from the original SwiftTray SVG. This avoids the old
  HICON-to-GDI+ conversion path responsible for noisy outlines.
- The bitmap is embedded inside the single EXE; no external PNG, SVG, DLL,
  service, WebView2 or installer is required at runtime.
- Keep the accepted system-tray icon and its native Windows `SwiftTray`
  hover tooltip; keep the existing device/audio/snip/shortcut behavior.
- Advance app metadata, version comparisons and GitHub release tag to 1.0.0.

## Mandatory acceptance before publishing

1. Run `BUILD.bat` on Windows 11; no Rust compiler or linker errors.
2. Run `cargo test --release` and confirm all unit tests pass.
3. Open About at 100%, 125% and 150% display scaling: smooth vector artwork,
   correct transparency and no black/cyan outlines on any part of the logo.
4. Hover the actual notification-area icon: it must show `SwiftTray` and remain
   crisp against a dark and light taskbar.
5. Verify per-app volume, mute states, audio activity dashes, app icon quality,
   device switching/disconnect/reconnect, and quick mixer behavior.
6. Verify screen snip when the mixer is open, then click outside to close.
7. Verify saved shortcuts after a restart and automatic recovery from conflicts.
8. Confirm launch-at-startup toggling and preserved aliases/appearance settings.
9. Try Windows Light mode and `Adjust for best performance`.
10. Upload source to GitHub, let the Windows CI job finish green, and test the
    downloaded unsigned `SwiftTray.exe` on a second Windows 11 x64 machine.
11. Create the `v1.0.0` release tag **only after** all checks pass. Verify
    `SwiftTray.exe` and `SwiftTray.sha256` appear as GitHub Release assets.
12. Configure an actual public GitHub repository for update checks; local builds
    without `SWIFTTRAY_GITHUB_REPO=owner/repo` intentionally say the source is
    not configured. Never use a placeholder repository for production.

## Known constraints to disclose

- Windows 11 x64 is the target. ARM64 and older Windows versions are not validated.
- Windows uses an undocumented default-device switching COM interface. This may
  change in future Windows builds.
- Windows input security may prevent snipping above elevated/protected windows.
- An unsigned EXE may trigger SmartScreen. Do not bypass platform safety prompts
  on behalf of users; code signing is recommended for distribution.
- This ZIP contains source, not a prebuilt or empirically verified EXE.

---

# SwiftTray v0.18.8 — New identity preview (not yet approved)

- Preview the user's supplied v3 logo and tray symbol (no artwork substitution).
- Bundle a curated multi-resolution ICO with an adapted small tray glyph for
  16, 20 and 24px; 32/40px use the supplied tray mark, 48px+ the full mark.
- Preserve alpha transparency; white in the original PNG is preview-only.
- Centralize brand colors in `src/brand.rs`, derived from the logo's SVG stops:
  green #20B27A, teal #149FA3, blue #1B78DB, deep blue #2756D6.
- Use the same smooth four-stop gradient on default-brand volume sliders and
  accent accents, while keeping system-accent mode and high-contrast support.
- Keep original application icons, mute red, native audio engine and hotkeys.

## Test before approving

1. Build with BUILD.bat on Windows 11; check for Rust compiler errors.
2. Verify 16px tray icon in both Light and Dark taskbar themes.
3. Compare 96px About logo to the supplied SVG, verify no white square.
4. Try default SwiftTray colors and Windows accent mode with the same audio.
5. Check icon/text contrast, notifications, shortcuts, About and tray menu.
6. Compare RAM/CPU to v0.18.6 after leaving the mixer open with active audio.

> Source and assets reviewed; Windows build/runtime test pending.

---

# SwiftTray v0.18.6 — Volume-aware peak visualizer

- Scale measured session activity by the selected per-app volume before drawing the dashed meter.
- Keep the meter at or before the volume thumb, including during rapid drags.
- Muted and zero-volume sessions do not display live activity.
- Keep the existing icon extraction, mixer layout, and shortcut engine unchanged.
- Add regression tests for Chrome at 3% versus 65%, clipping, and mute.
- Audio peak display is an estimate, not a calibrated measurement of speaker output.

# SwiftTray v0.18.5 — Native icon quality & live visualizer

- Resolve official Windows Shell application icons from the system extra-large
  icon image list; prefer 64px resource frames from Win32 executables.
- Preserve AppUserModelID-based identity and fall back to session/executable
  resources without hardcoded application-name/icon mappings.
- Render the original colored icons using high-quality GDI+ scaling; cache
  decoded images to avoid repeatedly decoding them during meter updates.
- Dim the *entire* icon to about 54% opacity when muted while keeping the red
  mute glyph and muted slider legible. No RGB recoloring of application logos.
- Replace the thin live-activity line with small, separate dashed bars based on
  real Windows Core Audio peak measurements. The height pattern is decorative,
  not a frequency spectrum; muted and silent sessions show no active bars.
- Retain existing shortcut, device-switch, snipping, tray-tooltip, settings,
  and update-checking behavior.

## Verify on Windows before publishing

1. Build with `BUILD.bat`; confirm no Rust compiler or linker errors.
2. Compare icons from Brave, Chrome, WhatsApp, Steam, Discord, games and PWAs
   with the originals in the taskbar/Start menu. Some apps may expose only
   low-resolution or fallback icons through Windows Shell.
3. Run audio through different applications; confirm the dashed meter changes
   with activity and does not shift the volume thumb.
4. Mute apps by clicking the icon and by dragging volume to 0. Confirm the icon
   becomes translucent, the badge remains red, and unmuting restores colors.
5. With multiple apps, monitor RAM/CPU for regressions and test at different DPI
   scaling values, especially 125% and 150%.

> Source ZIP verified structurally; Windows build/runtime verification pending.

---

# SwiftTray v0.18.4 — Tray tooltip hotfix

- Restore the normal Windows hover tooltip for the notification-area icon.
- Use `NIF_SHOWTIP | NIF_TIP` because `NOTIFYICON_VERSION_4` normally suppresses standard text tips.
- Display the short name `SwiftTray` on mouse hover, including after Explorer restarts.
- Add regression tests for the tooltip flags and UTF-16 text field.
- No changes to the audio engine, flyout visuals, capture shortcuts or user configuration.

> Source-only validation. Verify with an actual Windows 11 build before publishing.

---

# SwiftTray v0.18.4 — Tray tooltip hotfix and release candidate changes

- Refine About with a slightly larger original high-resolution logo,
  balanced typography and compact link-only update control.
- Refine tray context menu: consistent icons, calmer hover and separated Exit.
- Preserve asynchronous GitHub Releases checks using WinHTTP. Local builds
  show a neutral, concise setup status only on user request; network errors
  are short and never clip. No auto-download/install.
- Automatically retry failed global-hotkey registrations at low frequency
  while no shortcut-recording interaction is active.
- Preserve v0.17.4 mixer visuals and v0.18.0 audio/compatibility fixes.

> The source archive has **not** been compiled or executed on Windows here.
> Validate the build, UI and hotkey recovery on Windows before publishing.

---

# SwiftTray Windows release checklist

## Before publishing

1. Build and run all tests using the `windows-x64` GitHub Actions job.
2. On Windows 11, verify audio output switching with one device and multiple devices.
3. Verify USB or Bluetooth device removal and reconnection, including while hidden.
4. Test mixer sliders, application icons, live meters, and mute at zero.
5. Test the screen snip shortcut with the flyout both open and closed. Verify
   normal click-away behavior after a completed or cancelled capture.
6. Verify saved shortcuts, appearance, and startup preference after restarting.
7. Test at 100%, 125%, and 150% DPI and on a secondary monitor if available.
8. Confirm CPU/RAM use at idle, with the mixer open, and while playing audio.
9. Test the downloadable EXE on a clean Windows 11 x64 machine without Rust or
   Visual Studio Build Tools installed.
10. Confirm Windows SmartScreen / signing behavior. Sign releases when available.

## Publishing

1. Commit source to GitHub, then wait for CI to pass.
2. Create a version tag such as `v0.18.6` **only after testing**.
3. GitHub Actions publishes `SwiftTray.exe` and its SHA-256 checksum to Releases.
4. Test the actual downloaded asset, not only the local build result.

## Important limitations

- This project uses an undocumented Windows interface to set default output.
- `SendInput` cannot guarantee screen capture above elevated windows.
- The presence of a ZIP or a successful static source review is not evidence of
  a successful Windows build or cross-machine compatibility.

## v0.18.3 additional release checks

- On a fresh profile, verify `Alt + M`, `Alt + O`, and `Alt + S` register, or
  show an actionable conflict if another app owns a shortcut.
- Upgrade from v0.18.2 with custom Page Down/End/Page Up bindings and confirm
  all three are unchanged.
- Toggle Windows Light and Dark modes; verify SwiftTray retains its dark surface.
- Toggle Windows "Adjust for best performance" and make sure rendering remains
  readable (animations and backdrop effects must not be required).
- Try at least one Windows Contrast Theme; verify labels, controls, and the
  muted state are distinguishable. Do not declare this verified without testing.
- Exercise the redesigned shortcut rows using mouse and keyboard recording.

## v0.18.8 visual QA before beta

- Verify native 16px tray icon at 100% scaling; verify 20/24px frames on
  125%/150% taskbars where available; confirm Shell tooltip `SwiftTray`.
- Check About at 100%, 125%, 150% DPI: transparent, high-resolution full icon.
- Compare default brand gradient against Windows accent in mixer, notification,
  shortcut, appearance and tray-menu hover states. Third-party app icons remain native.
- Regression checks: mute state, activity visualizer, switch device, snip, hotkeys.
- The ZIP is source-only. A successfully built Windows EXE and a beta release
  should only be claimed after compilation and actual Windows testing.
