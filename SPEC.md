# SwiftTray Product Specification — v0.14.0

## Purpose

Native Rust/Win32 Windows 11 audio flyout with an output switcher, per-application volume mixer, and a customizable shortcut for the Windows screen capture overlay. UI and source are English-only. Prioritize usability, responsiveness, and accurate Windows visuals over an arbitrary RAM or executable-size limit.

## Main flyout

- One unified, bottom-right flyout layout; no Compact layout switch and no internal scroll bar.
- Active output icon, leading accent notch, saved device alias/name, switch button and settings gear.
- A slim mixer with genuine application icons, no permanent app names, per-app mute state, and accent-colored level sliders.
- Show app name in a native, text-measured tooltip below its row without covering the slider. Do not recolor or degrade app icons when muted.
- Master volume icon changes by effective level: zero, 1–32, 33–65 and 66–100 percent. Mute has an explicit mute icon.
- Slider interaction states: normal, pointer hover and pressed/drag; display a centered numeric value bubble only during dragging, including both endpoints (0 and 100). Clicking the track jumps to that point, dragging and wheel input change the volume.
- The appearance of sliders, icon sizes, tooltip placement and pointer interactions should continue to be calibrated against real Windows 11 reference screenshots.

## Settings

- In-flyout pages: audio device selection and alias/icon editor, shortcuts, notification settings, **Appearance** and About.
- Allow the user to select endpoints included in the output-switch cycle.
- Preserve device aliases locally and support a picker of system Fluent output symbols.
- Spatial sound is not part of SwiftTray settings. Do not open Windows Sound settings from Audio devices.
- Notification positions: top right, top left and above flyout. User-selectable symbols and accent styling.
- Tray menu has Open mixer, Settings, About, Exit.
- Settings home includes a Launch at startup toggle and an About page.
- Use the uploaded icon-only logo, since the uploaded lockups spell a different product name. Embed the multiresolution ICO in the EXE and tray.
- Offer an explicit opt-in on first launch. Store consent state in settings and use HKCU Run for startup; never silently enable it. Read the registry to present the effective current state. Update the entry when the user enables it from a new executable path.
- About shows the current version, product name and technical summary.
- Keep the app icon unchanged when muted; use a separate clearer mute marker and a refined, non-obscuring app-name tooltip. During slider dragging, reduce the inner thumb dot; at zero master volume show mute glyph.

## Shortcut behavior

- Global shortcuts for Open SwiftTray, Switch output and Screen snip.
- Recording a new shortcut: select an action, press a key or combination; Esc cancels without changing the binding.
- Do not replace a working binding if the replacement is unavailable; display an inline explanation.
- Plain F12, intentionally selected by the user, uses a scoped Windows low-level keyboard hook because RegisterHotKey reserves it. This captures only F12 while active; it is unhooked when removed or on exit. It can affect applications expecting F12.
- Other keys use `RegisterHotKey`; Windows-reserved combinations may still be unavailable.
- Screen snip triggers the Windows system rectangle capture overlay, not the full Snipping Tool application.

## Technical constraints

- Rust, Core Audio, Win32, optional GDI+ rendering. No embedded WebView2 or Electron.
- Handle light/dark system colors, system accent, multiple displays and DPI.
- Keep audio session enumeration robust across playback endpoints and avoid duplicate rows when sessions can be identified. Render the Windows System sounds session using a dedicated Fluent audio glyph rather than a generic application or monitor icon.
- Persist settings under LocalAppData, maintain old configuration compatibility.
- Automatic startup is per-user, does not require elevation, and must not run unless the user explicitly opts in.

## Verification requirements

- Windows x64 release build, resource icon embed, first-run opt-in, registry toggle on/off, About via Settings and tray, then app hotkeys, F12-only behavior, keyboard-hook teardown, snip overlay, audio engine, slider responsiveness, tooltips, tray menu, notification placement, DPI and memory use.
- The source archive is not sufficient to claim Windows build or runtime validation.

## Branding and onboarding (v0.11.0)

- Keep the exact four-panel SwiftTray mark. Use mint and teal for the three square tiles, with a fixed blue circular power tile. Never recolor the brand icon based on Windows accent color.
- Embed a multi-resolution app icon (including 16px and 256px) in the Windows executable. Keep the editable SVG source.
- On first launch, offer an optional startup choice. Do not register startup without consent; keep the per-user Settings toggle functional.
- The Settings landing page groups audio, shortcuts, and notifications as Quick settings; startup and About belong to General.
- About must show the app icon, name, current Cargo package version, purpose, native technology and MIT License.
- The **SwiftTray gradient** is the default accent for slider fills, controls and notifications. **Windows accent** is an optional, saved user choice. The brand logo and application icons always keep their native colors.

## UI cleanup (v0.11.0)

- Keep the System sounds audio session adjustable and recognizable.
- Removed the Spatial sound action and shortened Audio devices accordingly.
- Position the volume value popup directly above the thumb without horizontal clamping artifacts.
- Place the app-name tooltip between adjacent mixer rows with DPI-aware text measurement.


## v0.11.1
- Process description: SwiftTray
- Screen clipping: Windows Win+Shift+S keyboard sequence delivered through SendInput; release-event driven
- Reduced key release polling delay


## v0.13.0 App identity and performance

- Query the Windows AppModel identity of each audio process, then resolve it through Windows Shell's AppsFolder.
- Prefer the application's real installed identity and icon over a shared web runtime executable.
- Fall back to the native executable's version metadata and file icon for traditional desktop programs.
- Cache process identities and icon handles, and group audio sessions by installed app identity.
- No hardcoded mapping for WhatsApp, games, or package names.
- Icons remain their native colors; they never follow the Windows accent palette.
- Continue to test on Windows 11 because source-only builds cannot verify package-specific icon behavior.


## v0.13.0 changes
- Per-session live peak metering inside the existing volume track (only while flyout is visible).
- Dragging a volume to zero explicitly mutes that stream. Dragging above zero unmutes it.
- Screen snipping now triggers the Windows Win+Shift+S overlay directly instead of ShellExecute, which could launch Snipping Tool home. Elevated windows may block simulated input.
- Native app icons and the compact flyout dimensions are preserved.

## v0.14.0 Appearance and live meter

- `use_windows_accent=false` is the default for new or older configuration files. Appearance lets the user choose SwiftTray brand colors or the system accent; user preference persists across launches.
- Use the same in-flyout page width and preserve settings and existing Core Audio controls. Do not recolor the native icons of applications or the SwiftTray multi-part logo.
- The current live activity meter is a thin, genuine Windows audio peak meter. Do not clamp it to the volume slider progress or multiply the raw peak by the slider a second time; report it as stream activity, which may include system-provided gain. Update it only while the mixer is visible.
- The About screen uses a 96px embedded Windows icon resource, not the small system-tray bitmap, with its own dark mint-to-teal brand hero.
- Test mode persistence, restart, dark/light UI, accent changes, multi-DPI rendering, and audio activity latency on a real Windows machine.


## v0.15.0 visual-meter pass

- The live activity meter is embedded as a 2-DIP luminous inset inside the slider track, with a small endpoint cap. It is not clipped to the user-selected volume.
- Brand colors use a fixed mint/jade/teal track gradient and corresponding volume-thumb color; Windows accent mode remains available.
- Audio meter sampling is active only while the mixer is visible, at 40 ms intervals.
- Native application icon colors and all audio interactions remain unchanged.

## v0.16.0 capture and flyout lifecycle refactor (historical)

- Screen snip no longer calls `hide()` or waits for a keyboard-release timer.
- v0.16.0 tried ShellExecute of the Snipping Tool capture protocol. On the test PC it launched the app home window; this is deprecated as an implementation path in v0.16.1.
- Keep the flyout visible when snipping is launched from the visible flyout. Temporarily suspend click-away deactivation during capture and restore it using Windows events (foreground transitions, successful clipboard capture); do not add arbitrary hide delays or a pin-on-top feature.
- When capture starts while SwiftTray is hidden, do not show the flyout merely for snipping.
- Separate global hotkey registration and the opt-in plain-F12 hook from rendering; preserve saved mappings and Windows-reserved-key limitations.
- Restore the native-app mute indicator's v0.14.0 geometry without recoloring application artwork.
- Retain audio engine, AppUserModelID identity resolution, session metering and saved settings unchanged.
- Windows capture protocol support and event ordering vary across Snipping Tool builds; validate overlay opening, capture cancellation, foreground loss, and clipboard behavior on real Windows 11 before release.


## v0.16.1 screenshot regression fix

- Invoke the working Windows Win+Shift+S overlay shortcut through SendInput; do not use ShellExecute with Snipping Tool URI for unpackaged SwiftTray.
- Use a short-lived WH_KEYBOARD_LL hook to observe release of the configured capture shortcut. There is no fixed timer or arbitrary hide delay.
- Do not hide the flyout before starting a capture. Keep normal click-away behavior outside an active capture session.
- When capture completes or cancels, let foreground/clipboard events restore normal window behavior.
- Test real behavior on Windows 11, including PageUp and modifier hotkeys, elevated applications, and error handling.


## v0.17.1 compatibility correction

- Always render the correct theme-colored GDI+ background, independent of DWM material acceptance.
- Disable full-window transient Desktop Acrylic on this classic Win32/GDI+ surface: the v0.17.0 rendering path could produce an uncontrolled gray client area.
- Use a slight uniform-alpha dark window tint only where Win32 layered-window composition is available. This is not blurred Acrylic; if unsupported, display a fully opaque dark flyout.
- Restore the clean v0.16-style Settings rows and preserve the separate live peak meter, native icons, and existing audio/capture behavior.
- Verify appearance at different DPI scales and Windows light/dark themes before a public release.

## v0.17.0 visual refinement

- Keep the existing Core Audio and screen snip functionality unchanged.
- Draw an independent slim live-audio activity lane directly below the corresponding volume slider. Never place the live peak inside the slider track or let it alter the thumb position.
- Keep the SwiftTray brand gradient and Windows accent selection. Preserve original application icons and the v0.14 muted-app indicator.
- Original v0.17.0 intent: request native Desktop Acrylic. This was replaced by the compatibility correction in v0.17.1 because the actual visual result could become gray.
- Render the Settings rows with consistent subtle panels and hover feedback, without creating new settings or changing existing preferences.
- Treat this material update as unverified until built and visually tested on Windows 11.

## v0.17.2 visual stability decision

- Keep the entire GDI-rendered flyout opaque, including icons and text: do not enable `WS_EX_LAYERED` global opacity.
- Prefer a subtle native gradient surface to a bright or uncontrolled Windows backdrop.
- True acrylic requires an alpha-aware compositor and is reserved for a future renderer migration; no blur claim is made for this release.
- Meter is a 2-DIP activity-only line below the slider, not a full-width duplicate rail.


## v0.17.3 focused visual corrections

- Live activity meter moves 2 DIPs down from its v0.17.2 position without changing its value source or the main slider geometry.
- Brand gradient is one fixed-position three-stop GDI+ brush clipped to a single rounded filled track, removing a mid-track seam.
- Windows accent mode stays a uniform solid color; no changes to program state, feature behavior, or flyout dimensions.

## v0.18.3: shortcut UX and theme resilience

- Fresh installs default to `Alt + M`, `Alt + O`, and `Alt + S`
  (exactly two keys each); settings.json always takes precedence.
- Shortcuts rows use an inline underlined editing affordance, not separate
  keycap blocks. Recording still accepts one-key and multi-key custom binds.
- Normal Windows Light/Dark mode does not change the native flyout dark surface.
  Only the optional Windows accent color follows system personalization.
- Windows Contrast Themes override the brand palette and use system colors.
- Backdrops, transparency, and desktop animation effects are not prerequisites
  for rendering the opaque native flyout. A GDI fallback remains available.


## v0.18.4 tray tooltip correction

- With `NOTIFYICON_VERSION_4`, register the icon using `NIF_TIP | NIF_SHOWTIP`.
- The standard Windows notification-area tooltip must display `SwiftTray` on hover.
- Reapplying the icon after `TaskbarCreated` must retain this flag and the title.
- This is a Windows Shell tooltip, not a custom UI popup or timer.

## v0.18.8 identity candidate

The supplied v3 SVG is the sole source of product-brand colors and the large
logo. Source SVGs are included for review. Short forms are used for the tray.
App icons continue to originate from each user's installed Windows apps.

## v1.0.0 release-candidate brand render path

The large About glyph uses an embedded 256x256 premultiplied BGRA bitmap, built
from `assets/SwiftTray-logo.svg` by `tools/generate_icons.py`, and passed directly
to GDI+ with PixelFormat32bppPARGB. Its pixel Vec stays alive until the GDI+
image is disposed. This avoids HICON alpha/AND-mask conversion artifacts.
The tray image remains a native multi-resolution icon with its native Shell
tooltip. The application gradient and Windows accent option are unchanged.
Publication requires Windows build and acceptance tests in RELEASE.md.
