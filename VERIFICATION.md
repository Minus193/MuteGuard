# MuteGuard verification report

Date: 2026-08-27

## Result

The final Windows x64 source, Dioxus asset package, portable archive and NSIS
installer pass the static, build and package checks listed below. The newly
rebuilt unsigned executable was deliberately not launched automatically, to
avoid another set of Trend Micro prompts for each generated hash. No known
RustSec vulnerability remains.

The packaged executables are intentionally not code-signed. Trend Micro's
reputation-based "Newly Encountered Program" protection can consequently ask
for confirmation whenever a newly rebuilt hash is launched from PowerShell.
This is an external trust/signing limitation, not a malware detection.

## Build and code checks

All project compilation and packaging commands ran in
`muteguard-builder:rust-1.98-dx-0.7.6`. Network access was disabled for the
actual checks and release build. It was enabled separately only to download
the two corrected crate versions fixed in `Cargo.lock` and to refresh the
RustSec advisory database.

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --offline --locked --tests --target
  x86_64-pc-windows-gnu -- -D warnings`: passed.
- Focused structural Clippy checks for excessively long functions, redundant
  clones, verbose assignments, needless value passing and avoidable control
  flow: passed with zero warnings.
- `cargo test --offline --locked --no-run --target
  x86_64-pc-windows-gnu`: passed and linked the Windows test harness.
- The linked harness was executed on Windows before the latest UI/input patch:
  10 passed, 0 failed. The current 37-test harness, including native overlay,
  exact HEX color handling, application-accent normalization, compact scale,
  automatic palette-icon contrast, arbitrary keyboard-chord
  migration/matching and work-area margin tests, links successfully.
- `dx build --desktop --release --target x86_64-pc-windows-gnu --frozen`:
  passed and assembled all 30 referenced Dioxus assets.
- `makensis` compiled `installer/muteguard-cross.nsi`: passed.
- Both PowerShell build scripts parse without errors.
- All 53 remaining SVG files parse as XML; the web manifest parses as JSON.
- `git diff --check`: passed (line-ending notices are informational).

The runtime now changes its working directory to the executable directory
before Dioxus starts. This makes adjacent portable assets resolvable when the
application is launched from a startup entry, shortcut, PowerShell or another
foreign working directory.

Windows startup registration is now reconciled once when the background
process starts. The existing current-user Run value is read first; MuteGuard
writes only when the configured registration is missing or points to a stale
executable and deletes only when a disabled registration still exists. There
is no polling or recurring startup-registry work after initialization.

Settings typography now uses shared tokens for page titles, card titles,
descriptions, labels, input values and secondary details. In particular,
selected monitor text and monitor-menu rows have identical metrics. The
position card exposes a temporary, non-persisted Preview checkbox without
restoring overlay drag. While checked it keeps the overlay visible; unchecking
it or leaving the section restores the configured visibility. A renewable
background lease also clears an abandoned preview after an abnormal Settings
exit. Enabling `Show border` reveals the shared exact-color editor. The chosen
border color is normalized, persisted and used by the native antialiased
renderer.

Hotkeys now use a normalized multi-key representation. Single keys,
modifier-only bindings, A+B-style chords and Win combinations are accepted
without forcing Ctrl; legacy modifier-plus-key configuration is migrated
transparently. Capture completes on the first constituent release while
retaining the entire chord, and a completed capture cannot be overwritten by
subsequent releases. While recording, short-lived physical key-state polling
backs up the low-level hook so missed Windows/WebView delivery cannot leave the
card stuck. The Settings process installs its own input hooks only for that
recording interval, leaving the background mute hook authoritative when the
Settings window has focus. Overlay placement uses each selected monitor's
Windows work area and applies a 10 px inset, keeping every anchored overlay
outside the taskbar and away from available screen edges. Free dragging has
been removed; top-center is the new default anchor. Secondary Settings
descriptions use a compact 14 px type size.

Overlay icon selectors render each selected SVG instead of a generic
microphone glyph. Fixed selector popups close immediately when the Settings
content scrolls, so they cannot detach from their trigger. Overlay and tray
microphone icons now expose one editable mode labelled `Colored`; legacy
`Colored` configuration migrates to that mode with a normalized custom color.
The shared color control accepts direct `#RRGGBB` entry, inline visual tuning
and presets without saving incomplete input; `#7D42FB` is covered explicitly.
The native Windows dialog has since been replaced by an inline MuteGuard color
studio with HSV controls and RGB feedback. Slider movement updates local
preview state, while only committed, changed colors reach the atomic config
writer; replacement uses unique temporary files and bounded retries for
short-lived antivirus or reader locks. Its tuning rows keep the label, a
flexible full-width slider and the percentage in a stable left-to-right order;
the spectrum is borderless and keeps its marker inside the selectable area.
The spectrum itself accepts clicks and pointer dragging: horizontal position
controls saturation, vertical position controls brightness, preview remains
local during movement and the exact color is committed on release.
General, Overlay and Tray share eight immutable square presets (`#FFFFFF`,
`#BDC3C8`, `#222F3D`, `#7E40FD`, `#2980B9`, `#F39C19`, `#2ECC70`, `#E84B3C`)
plus a separate custom-color editor.
General can use either the Windows accent or a live custom application accent.
The dark overlay background is `#131313`, and its displayed 100% scale now maps
to 85% of the previous geometry. MDI is the default microphone artwork.
The `All microphones` hotkey target retains the distinct four-square group
glyph. Descriptive microphone cues in Hotkeys, Overlay visibility/content and
Tray status use Fluent, independently from the actual selectable overlay/tray
artwork. Tray configuration previews and the Overlay Content → Style cue always
use the unmuted glyph; only the actual Overlay icon selector uses barred glyphs
when visibility is `Visible when muted`, rather than following the live
microphone state. Color
swatches choose a light or dark palette glyph from their perceived brightness,
so the icon remains legible on both pale and dark colors. Square
checkboxes share one clearer selected/unchecked treatment, and selector popup
shadows use a compact two-layer falloff instead of the previous heavy halo.
Left-clicking the tray icon opens or focuses Settings. The right-click menu
offers a state-aware `Mute microphone` / `Unmute microphone` command, Settings
and Exit. When Core Audio is unavailable, the mute command is replaced by a
disabled `Microphone unavailable` item. The former Pause/Resume hotkeys state,
hook branches and tooltip suffix have been removed completely.

The nine overlay anchors now expose only their visible 4:3 buttons as pointer
targets; empty monitor cells are inert. The monitor surface uses the same
control background and corner hierarchy as adjacent fields. The custom wheel
handler was removed from shared range controls, so scrolling never steps a
slider value. The rounded native background and its optional border use an 8x
coverage mask instead of GDI's non-antialiased edge. Native overlay text and
fallback glyphs render into a 3x
non-ClearType bitmap and are downsampled into a grayscale alpha mask before
composition. Styled Segoe UI faces normalize to `Segoe UI`, leaving the
separate weight setting as the sole source of thickness.
The main ARGB surface is cleared to transparent before composition; the former
magenta GDI sentinel therefore cannot leak through antialiased corner pixels.

Configuration recovery also preserves an invalid legacy `SilenceV2` file,
uses a process-specific high-resolution backup name, and does not let a
Windows startup-registration error prevent `config.json` from being restored
to safe defaults.

The final maintainability pass separates native surface allocation, background
painting, icon composition and label composition. Settings rendering is split
by cohesive UI responsibility: Hotkey cards, Overlay behavior/content/icon/
text/background controls and Tray variant/microphone controls no longer live
inside monolithic section functions. Repeated preview and temporary-overlay
paths share one implementation, and mechanical clone/control-flow noise was
removed without changing saved configuration or runtime behavior.

The final defensive review additionally covers temporary and degraded runtime
states. Overlay Preview now remains available even while Core Audio has no
active microphone. A Settings window found successfully is treated as handled
even when Windows refuses foreground activation, avoiding a redundant helper
process; protocol/second-instance requests use the same focus-or-open path.
Generated hotkey IDs combine process, high-resolution time and an atomic
sequence and are tested across a 10,000-ID burst. Monitor selection callbacks
reject an empty vector without indexing it. When selected monitors are
disconnected, unavailable IDs stay in the saved configuration for a future
reconnect, while the native runtime resolves only currently available physical
monitors, deduplicates the primary display and creates exactly one primary
fallback instead of stacking multiple overlay windows. Windows display-change
messages rebuild that resolved set immediately, covering both disconnect and
reconnect without waiting for another microphone or configuration event.

## Package checks

- Portable directory: 36 files, including 30 hashed Dioxus assets.
- Executable: references all 30 packaged asset names.
- ZIP: all 36 entries are byte-identical to the staging folder.
- The portable archive retains `README.md` and third-party notices. The NSIS
  installer excludes every `.md` from its recursive input and deletes root
  Markdown files left by an older installation before copying the new build.
- Windows resource: the purple slashed MDI microphone and version
  information are present. The source ICO contains eight validated PNG-backed
  sizes: 16, 20, 24, 32, 40, 48, 64 and 256 px.
- The icon glyph is pixel-bounds centered at `511.5 x 511.5` on the 1024 px
  source render, with symmetric padding (`224/224` horizontal and `208/208`
  vertical). All four corner pixels have alpha `0`; rounded-corner space is
  transparent rather than white.
- The Linux-hosted cross-build logs a non-fatal "may not have an icon" warning;
  inspection of the final Windows PE confirmed eight `RT_ICON` entries, the
  group-icon entry, the `.rsrc` section and the version resource.
- Version fields: product `MuteGuard`, file/product version `0.1.0`, original
  filename `muteguard.exe`. The application `FileDescription` is `MuteGuard`,
  so Task Manager uses the short product name; the installer description is
  `MuteGuard Setup`.
- PE format: 64-bit Windows GUI executable.
- PE hardening: `HIGH_ENTROPY_VA`, `DYNAMIC_BASE` and `NX_COMPAT` are enabled.
- No section is both writable and executable; release debug/symbol sections
  are stripped.

## Windows live regression

An earlier post-fix release executable was launched manually once after Trend
Micro approved that build's hash. The following baseline checks passed on
Windows before the current UI/input changes:

- the background instance and its `--settings` child both remained responsive;
- opening Settings produced a visible `MuteGuard Settings` window;
- every then-current Settings section rendered successfully;
- the tabs were cycled repeatedly without a panic, blank panel or unresponsive
  process;
- the test instance accepted the internal clean-exit message and left zero
  MuteGuard processes running.

The reported Overlay-tab panic was caused by hook-using section functions
sharing the parent component's hook scope. Each settings section now renders
through its own Dioxus component, so switching tabs cannot change the parent's
hook order. The Settings window is also created visible instead of depending
on a later desktop visibility call.

## Dependency audit

RustSec loaded 1,226 current advisories and found zero vulnerability
advisories in the 575-package lockfile.

- `anyhow` was updated from 1.0.102 to patched 1.0.103.
- `memmap2` was updated from 0.9.10 to patched 0.9.11.
- The 16 remaining audit entries are informational warnings. GTK/glib and
  rand 0.7 entries are not in the Windows target graph. The remaining entries
  concern transitive crates marked unmaintained; their advisories provide no
  patched version and do not report a known vulnerability.

## Produced artifacts

| Artifact | Size | SHA-256 |
| --- | ---: | --- |
| `dist/0.1.0/muteguard-0.1.0-windows-x64-portable.zip` | 8,314,652 bytes | `FDE1099A39136C27DC656AAF0AC3DC6EDF61E29F1EADBE0BA6FAAC06381A2612` |
| `dist/0.1.0/muteguard-0.1.0-windows-x64-setup.zip` | 6,060,537 bytes | `C19F2603CE91EBDBF2DD4282B8E937AEF561104F47BDC6AA290A8847CD2DC4D1` |
| Installer EXE (standalone and inside setup ZIP) | 6,102,159 bytes | `D65A0F9C5F02B2CD3797A187A82CD187EA0189660BDFF3AF8FB70843AAC64BE4` |
| Portable `muteguard.exe` | 19,506,176 bytes | `EE32D070A53DAAD866D2150AC9CDA4AC8C2C5588CFE6E92D390B9BD943C21DD6` |
| Portable `WebView2Loader.dll` | 160,320 bytes | `8427B1FC58EC707813E5C0A51EB5D69397BB333250A7B891BE4D3B123F1E0F1C` |

## External/manual boundary

Trend Micro's reputation protection blocks new unsigned hashes launched from
PowerShell. The post-fix runtime test therefore used one manual Explorer launch
after the user's explicit approval; repeated automated launches were avoided.
Trend Micro removed earlier newly written setup executables after they had been
built and verified. At the time of this final verification, both the standalone
setup EXE and its byte-identical copy inside
`muteguard-0.1.0-windows-x64-setup.zip` are present; the ZIP remains the
reputation-resistant recovery copy if the standalone file is quarantined.

A fresh Microsoft Defender scan of this final post-fix build was attempted
with remediation disabled. Both the legacy launcher and the current platform
scanner returned `0x80004005`; the log states `Product/Feature disabled`,
consistent with Trend Micro being the active antivirus provider. This is not a
detection, but it also means no Defender result is claimed for the final hash.
An earlier pre-GUI-fix package scan completed with no threats found.

The following actions are deliberately not claimed as automated evidence:

- changing the real microphone mute state;
- installing the final rebuilt installer over the currently installed copy;
- measuring idle CPU/RAM over a long meeting;
- bypassing Trend Micro reputation protection;
- Authenticode/SmartScreen trust, which requires a trusted code-signing
  certificate.
