# MuteGuard 0.1.0

First focused personal release.

- Renamed the application, package, executable, configuration directory,
  startup entry, installer, mutexes, windows, and visible UI to MuteGuard.
- Reduced the runtime to default/all-microphone mute toggles.
- Made Core Audio callbacks the authoritative path for tray and overlay state.
- Removed recurring runtime/configuration polling and all device, session,
  process, microphone-use, inactivity, gamepad, sound, updater, hold, volume,
  and device-switching subsystems.
- Kept multiple global keyboard/mouse hotkeys with per-binding target and
  modifier handling.
- Kept a configurable native click-through overlay and minimal tray menu.
- Kept Settings as a separate WebView process with explicit configuration
  notification to the background process.
- Give every Settings section its own Dioxus component scope so switching
  between hook-using tabs cannot panic, and create the Settings window visible.
- Replaced upstream branding with a dark MuteGuard application icon built from
  the purple slashed MDI microphone while retaining attribution and the
  Apache License 2.0.
- Prefer the Windows communications capture endpoint, retry startup mute until
  that endpoint is ready, and keep Core Audio work off low-level input hooks.
- Make configuration replacement atomic and surface save/startup-registration
  failures directly in Settings.
- Validate native overlay paint messages correctly.
- Prevent cancelled new hotkeys and duplicate bindings from double-toggling.
- Keep new hotkeys transient until a real shortcut is captured, and always end
  recording cleanly on cancel, delete, tab change or window close.
- Install the Settings process's low-level input hooks only while a shortcut is
  being recorded, so the background mute hook remains authoritative when the
  Settings window has focus.
- Treat an empty shortcut as inert, ignore clicks inside Settings while a
  shortcut is being recorded, and recover from low-level key-up events missed
  by the Windows hook.
- Make an "all microphones" target supersede duplicate default-microphone
  targets and report partial endpoint failures instead of presenting them as a
  successful toggle.
- Surface microphone, configuration, process, and notification errors; provide
  timestamped backup-and-reset recovery for an invalid configuration.
- Migrate the legacy single-monitor setting to a required multiselect, preserve
  unavailable displays explicitly, and render a synchronized native overlay on
  every selected monitor.
- Handle message-loop errors, open Settings from a primary tray click, and show
  fatal or panic errors in a native dialog.
- Bound cross-process window messages so a non-responsive window cannot block
  the installer or Settings indefinitely.
- Load monitor/font catalogs only when the Overlay tab needs them, persist
  continuous controls when editing finishes, and cache monitor, accent and SVG
  icon data used by the native overlay and tray previews.
- Add keyboard navigation and ARIA state to custom selectors and tray choices,
  and remove unused UI/CSS variants.
- Refactor the native renderer and the Hotkeys, Overlay and Tray settings into
  cohesive components and helpers; remove redundant clones and duplicated
  temporary-overlay paths, with focused structural Clippy checks kept clean.
- Show the actual microphone artwork in the icon selector, and close open
  selector popups when the Settings content scrolls so they cannot detach from
  their controls.
- Make range controls update their thumb and value continuously while saving
  only the committed value, and replace free-axis position sliders with an
  accessible nine-anchor monitor picker contained within its card, with 4:3
  position markers.
- Limit each overlay-position hit area to its visible 4:3 button, align the
  monitor surface with the other controls, and let the mouse wheel scroll the
  Settings page without changing any range value.
- Provide one editable `Colored` mode for overlay and tray microphone icons,
  align text measurement and rendering, and preserve literal label characters
  in the native overlay.
- Capture arbitrary simultaneous keyboard chords through the native settings
  hook, including single keys, modifier-only bindings, A+B and Win+Shift+A,
  without requiring Ctrl or relying on WebView keyboard delivery. Existing
  shortcuts migrate transparently to the multi-key representation.
- Finish shortcut recording when the first captured key or mouse button is
  released, while retaining every key that formed the chord.
- Supplement low-level hook events with short-lived physical key-state polling
  while the recorder is open, so Windows and WebView delivery cannot leave a
  shortcut card stuck in recording mode.
- Constrain anchored overlays to every selected monitor's Windows work area,
  outside the taskbar and with a 10 px edge margin. Remove free dragging and
  make top-center the default anchor.
- Set the overlay's dark background to `#131313`, retain adjustable background
  opacity, reduce its effective 100% geometry to 85% of the previous size,
  enlarge icon content by reducing its internal padding, and restyle the
  nine-position picker as a monitor surface.
- Replace read-only color displays with a shared color picker that combines a
  visual swatch, exact editable HEX input, validation and presets; values such
  as `#7D42FB` now round-trip exactly for overlay, tray and application accent.
- Replace the unstyleable Windows color dialog with an inline MuteGuard color
  studio providing a spectrum preview, Hue/Saturation/Brightness controls and
  RGB feedback. Preview changes remain local, identical commits are skipped,
  and atomic configuration replacement retries short-lived file locks.
- Keep Color Studio tuning rows in label-slider-value order, remove the
  spectrum frame, and provide one custom-color editor beside eight immutable
  shared presets: white, grey, slate, purple, blue, orange, green and red.
- Add a General setting for either the Windows application accent or a custom
  accent, applied live to controls, focus and tinted surfaces.
- Use MDI for the application icon and default selectable microphone artwork;
  use Fluent for descriptive microphone cues in Hotkeys, Overlay visibility,
  Overlay content style and Tray status; align monitor checkboxes on the left
  and standardize nested corner radii by visual hierarchy.
- Keep Tray configuration artwork and the Overlay Content → Style cue unmuted;
  show barred artwork in the actual Overlay icon selector only when visibility
  is explicitly set to `Visible when muted`.
- Restore the four-square group glyph for the `All microphones` hotkey target,
  unify square checkboxes with a clearer selected/unchecked treatment, and
  soften selector popup shadows.
- Automatically switch the palette glyph between light and dark foregrounds
  so it remains legible over every exact color swatch.
- Make the Color Studio spectrum respond to clicks and pointer dragging,
  preview saturation/brightness continuously and commit once on release.
- Render native overlay glyphs through a 3x monochrome mask downsampled to
  grayscale, avoiding bitmap ClearType artifacts, and collapse styled Segoe UI
  faces to the base family so the dedicated weight control remains authoritative.
- Supersample the native rounded overlay silhouette at 8x resolution, removing
  the jagged GDI corner edge while preserving independently rendered content.
- Clear the software overlay surface to transparent ARGB before composition so
  supersampled corners cannot expose the former magenta GDI sentinel color.
- Replace the native hotkey target dropdown with the shared custom selector.
- Keep left-click focused on opening Settings; restore a dynamic
  `Mute microphone` / `Unmute microphone` command to the right-click menu and
  remove the hotkey-pause state and all of its runtime branches.
- Remove the About settings section and its dedicated code, style and icon.
- Reset the central Settings scroll position whenever a navigation section is
  selected, so every section opens from its heading instead of inheriting the
  previous section's offset.
- Remove the remaining unreferenced upstream icons, store imagery, screenshots,
  fonts and obsolete website redirect while preserving every packaged asset.
- Package the actual Dioxus executable with all referenced assets, embedded
  Windows icon/version metadata, third-party notices, and the WebView2 loader.
- Expose the application as `MuteGuard` in Windows Task Manager and the
  installer as `MuteGuard Setup` instead of using their longer descriptions.
- Keep native and Docker portable packages aligned on README, license,
  third-party notices and application icon.
- Detect a missing WebView2 Runtime before opening Settings; the installer can
  fetch Microsoft's official Evergreen bootstrapper with user consent.
- Keep Markdown documentation in the source repository only. Portable and
  installer packages contain no `.md`, and upgrades remove Markdown files
  left in the application directory by earlier versions.
- Remove the remaining upstream funding, website, and web-manifest branding.
- Pin remediated transitive dependencies and ship an installer that uses a
  fixed application directory with non-recursive cleanup.
- Standardize Settings typography through shared title, card-title,
  description, label, input and detail tokens; selector rows and their current
  values now use the same input scale.
- Add a temporary Overlay Preview checkbox beside the nine-position picker
  without reintroducing free dragging. While selected it keeps the current
  overlay visible, and unselecting it restores the configured visibility.
- Keep Overlay Preview usable when no capture endpoint is currently available.
  A short renewable lease prevents the preview from remaining visible if the
  Settings process closes unexpectedly.
- Avoid redundant Settings helpers when Windows denies foreground activation,
  and route every second-instance/open request through the same focus-or-open
  path.
- Make hotkey IDs collision-resistant during burst creation and guard monitor
  multiselect callbacks against an empty value.
- Preserve unavailable monitor choices for reconnect while deduplicating the
  active physical displays, so disconnected selections produce one primary
  fallback instead of stacked overlay windows; display-change messages rebuild
  that active set immediately.
- Add an exact custom border color when `Show border` is enabled and render it
  in the native antialiased overlay.
- Reconcile `Start with Windows` once when the background process starts:
  read the current user Run entry and repair it only when missing or stale, so
  upgrades self-heal without recurring checks or unnecessary registry writes.
