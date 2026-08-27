# Silence 2.3.3 vs MuteGuard 1.0.0

Date: 2026-08-26

## Comparison basis

This document compares the repository's original Git `HEAD` (`silence`
2.3.3) with the current MuteGuard 1.0.0 working tree. It describes the actual
local implementation, not only the two README files.

MuteGuard is a deliberately subtractive fork. It retains the Windows Core
Audio, Dioxus, global-input-hook, tray and native-overlay foundation, but
removes the broader audio-management and automation feature set in order to
serve one use case: reliably toggling microphone mute during meetings.

The tracked diff touches 83 files and currently contains about 2,704 inserted
lines and 22,001 removed lines. New untracked build, packaging and legal files
are not included in those Git diff totals.

## At a glance

| Area | Silence 2.3.3 | MuteGuard 1.0.0 |
| --- | --- | --- |
| Primary purpose | General microphone and audio-device control | Focused meeting microphone mute control |
| Mute actions | Toggle, force mute, force unmute, hold actions | Toggle only |
| Microphone targets | Specific device or all microphones | Default communications microphone, with console fallback, or all active microphones |
| Inputs | Keyboard, mouse and gamepad | Keyboard and mouse |
| Hotkeys | Multiple action types, device actions and Settings action | Multiple mute-toggle bindings with per-binding target and optional modifier ignoring |
| Device switching | Input and output switching | Removed |
| Sounds | Built-in themes and custom audio files | Removed |
| Automation | Inactivity auto-mute and optional activity unmute | Removed; optional one-time mute at startup remains |
| Overlay | Configurable visual feedback | Retained and narrowed to authoritative microphone state |
| Tray | Mute, devices and broader controls | Status-only icon; pause/resume hotkeys, Settings and Exit |
| Updater | In-app update flow | Removed |
| Settings import/export | Backup/restore and v1 import | Removed; one-time compatible SilenceV2 migration remains |
| Runtime model | Multiple monitoring and automation subsystems | Event-driven Core Audio notifications; no recurring device/session/process/config polling |
| Settings process | Dioxus/WebView settings | Retained as a separate process and explicitly released when closed |
| Windows builds | x64, x86 and arm64 | Verified Windows x64 package |
| Identity | `silence`, version 2.3.3 | `muteguard`, version 1.0.0 |

## Functionality retained

- Multiple global keyboard and mouse hotkeys.
- Per-binding choice between the default microphone and all microphones.
- Optional `Ignore modifiers` matching for an individual shortcut.
- Low-level Windows keyboard and mouse hooks.
- Windows Core Audio endpoint mute control and notifications.
- Tray-state feedback and a configurable tray icon.
- Native click-through overlay with visibility, monitor, position, scale,
  icon/dot/text, color, opacity, background and border options.
- Start with Windows.
- Optional mute-on-startup behavior.
- Dioxus desktop Settings UI using WebView2.
- Apache License 2.0 and explicit upstream attribution.

## Functionality removed

The following Silence subsystems and their UI/assets are absent from
MuteGuard:

- gamepad/controller input and all controller artwork;
- hold-to-mute, hold-to-unmute and hold-to-toggle actions;
- force-mute and force-unmute actions;
- device-specific hotkey targets;
- input and output device switching;
- per-process microphone-use detection;
- inactivity monitoring, auto-mute and activity-based unmute;
- built-in sound themes, custom sounds and audio preview/playback;
- updater, release notifications and update overlay;
- welcome/onboarding screen;
- Settings backup/export/import UI and v1 import flow;
- Mica-background toggle;
- recurring configuration, device, session, process, microphone-use and
  gamepad polling;
- upstream funding, download, issue, website and product branding.

The removed Rust dependencies include the gamepad, audio playback, file
dialog, HTTP/update, semantic-version and toast-notification stacks
(`gilrs`, `rodio`, `rfd`, `reqwest`, `semver`, `winrt-toast` and related
support crates).

## Runtime changes

### Authoritative mute state

Silence supported a wider collection of actions and monitoring paths.
MuteGuard makes Core Audio the single state authority:

```text
user action
  -> read current mute state
  -> set target mute state
  -> Core Audio endpoint notification
  -> read authoritative state again
  -> update tray and overlay
```

MuteGuard does not optimistically change its displayed state after a mute
request. Endpoint-change notifications rebind the volume callback when the
Windows default capture endpoint changes.

### Default microphone selection

MuteGuard prefers the Windows default **communications** capture endpoint,
which normally represents the meeting microphone. It falls back to the
console-default capture endpoint when required. A startup mute is retried for
a finite period while that endpoint becomes available.

### All-microphone action

When a binding targets all microphones, the default microphone determines the
direction of the toggle. Active capture endpoints are enumerated only for that
action. Partial endpoint failures are reported instead of being presented as
a fully successful toggle.

### Error handling and recovery

MuteGuard adds explicit handling for microphone, configuration,
process-launch, notification-registration, message-loop and panic failures.
Invalid configuration can be backed up with a process-specific timestamp and
replaced atomically with safe defaults. A Windows startup-registration error
does not prevent configuration recovery.

### Settings lifecycle

Settings remains a separate process, but its window is now created visible.
Every settings section has its own Dioxus component/hook scope, preventing the
hook-order panic that previously occurred when switching to Overlay. Overlay
placement is now anchor-only, and the required monitor multiselect creates one
synchronized native overlay per selected display.

## Configuration and migration

| Item | Silence | MuteGuard |
| --- | --- | --- |
| Current configuration | `%APPDATA%\SilenceV2\config.json` | `%APPDATA%\MuteGuard\config.json` |
| Startup registry value | Silence identity | `MuteGuard` |
| Process/window/mutex identity | Silence/SilenceV2 | MuteGuard-specific names |
| Runtime reload | Broader monitoring/config behavior | Explicit native notification after Settings saves |
| Legacy compatibility | Native Silence configuration | Reads compatible settings once from SilenceV2 |

Only compatible toggle hotkeys and settings are migrated. Gamepad bindings,
force actions, device-specific targets, sounds and other removed features are
discarded rather than silently emulated.

## User-interface changes

Silence exposed General, Devices, Hold to Mute, Hotkeys, Sounds, Overlay, Tray
Icon, Auto-Mute and About areas, plus onboarding and update surfaces.
MuteGuard contains four focused sections:

1. General
2. Hotkeys
3. Overlay
4. Tray

Branding, iconography, application copy, website files and installer metadata
have been changed to MuteGuard. The remaining controls were simplified around
the reduced data model rather than merely hidden.

## Build and distribution changes

- Package identity changed from `silence` 2.3.3 to non-publishable
  `muteguard` 1.0.0.
- Release symbols are stripped.
- A Windows resource build step embeds the MuteGuard icon and version fields
  into the final PE executable.
- The verified reproducible pipeline uses a pinned Docker builder containing
  Rust 1.98, Dioxus CLI 0.7.6, the Windows GNU target and NSIS.
- Project compilation and packaging run offline once the builder image and
  Cargo volumes have been prepared.
- The current verified output is Windows x64 only: a portable ZIP and a
  current-user NSIS installer.
- The installer uses `%LOCALAPPDATA%\Programs\MuteGuard`, writes an uninstaller
  entry and creates Desktop/Start-menu shortcuts.
- The installer detects WebView2 Runtime and can offer Microsoft's Evergreen
  bootstrapper. Only Settings requires WebView2; background hotkeys, tray and
  overlay do not.
- Portable packaging includes the exact 31 Dioxus assets referenced by the
  executable, `WebView2Loader.dll`, the license, README, third-party notices
  and application icon.

## Security and trust boundary

MuteGuard is not Authenticode-signed. Its PE includes version/icon resources
and standard hardening flags (`HIGH_ENTROPY_VA`, `DYNAMIC_BASE`, `NX_COMPAT`),
but those are not a substitute for a trusted publisher certificate.
Reputation-based products such as Trend Micro can therefore ask for approval
for every newly rebuilt hash even when no malware is detected.

## Practical consequence

Choose Silence when its automation, device switching, gamepad, sounds,
updater or broader action model is required. Choose MuteGuard when the desired
behavior is a smaller, more auditable, event-driven meeting utility whose
scope is limited to microphone mute, global hotkeys, tray feedback and the
native overlay.
