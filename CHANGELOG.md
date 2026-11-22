# Changelog

## [Unreleased] — approved visual identity (pending Windows/MSIX release verification)

- Replace the previous four-panel symbol with the approved interlocking ribbon mark for Explorer, Start, taskbar, About, and Windows package image assets.
- Use the same mark for System Tray, with 8% optical enlargement inside the tray icon's transparent frame for better legibility at small sizes.
- Embed app ICO as resource ID 1 and optimized notification-area ICO as resource ID 2; the audio and input behavior are unchanged.
- Add asset checks to GitHub CI. This source update is not a new signed Store package and does not alter existing v1.0.0 releases.


## [1.0.0] — Release candidate (not yet published)

- Repair high-DPI About logo rendering with direct premultiplied-alpha bitmap
  from the user-provided vector artwork, embedded in the executable.
- Preserve the tuned notification-area icon and tooltip from v0.18.8.
- Preserve default-brand gradient, Windows accent selection, native application
  icon pipeline, live audio activity dashes, output switching and screen snip.
- Include explicit Windows acceptance tests and GitHub Release instructions.

## [0.18.8] — Brand polish preview

- Tuned tray icon frames, About high-resolution icon path and brand colors.

### Final UI polish (same v1.0.0)

- The tray context menu now starts with "Open mixer" instead of repeating the application logo and name. The tray icon/hover tooltip are unchanged.
- The About page uses a recognisable GitHub mark for "Open source"; its existing repository link is unchanged and the icon respects the active accent.
- No audio engine, capture, hotkey, startup, or update-checker behavior changes.
