# MuteGuard verification report

Date: 2026-08-28

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
- The current 66-test Windows harness links and runs successfully. It includes
  native overlay, exact HEX color handling, application-accent normalization,
  compact scale, automatic palette-icon contrast, arbitrary keyboard-chord
  migration/matching, work-area margin, WAV
  validation/duration, diagnostics-report, update interval/version/URL safety,
  notification XML and complete Guide-section coverage tests.
- `dx build --desktop --release --target x86_64-pc-windows-gnu --frozen`:
  passed and assembled all 34 referenced Dioxus assets.
- `makensis` compiled `installer/muteguard-cross.nsi`: passed.
- Both PowerShell build scripts parse without errors.
- All 55 remaining SVG files parse as XML; the web manifest parses as JSON.
- `git diff --check`: passed (line-ending notices are informational).
- An anonymous read-only request to the configured public GitHub endpoint
  returned stable tag `v1.3.2` and the expected x64 setup asset name. No GitHub
  credential or token was supplied.

The release build also passed the following feature-specific checks:

- Default-capture changes reuse the existing Core Audio callback. The final
  event in a callback burst renews a 350 ms debounce timer before MuteGuard
  re-reads Windows state, avoiding comparisons against an endpoint assignment
  that has not settled yet. Communications, console and multimedia capture
  defaults are tracked independently, while the communications endpoint
  remains the operational mute target. Device state/add/remove callbacks use
  the same delayed reconciliation, so physical disconnects and ordinary
  default-device changes are both covered. When a reconnected endpoint is not
  immediately ready, callback binding retries once per second until successful
  and then stops. Expected disconnect transitions update tray/overlay state
  without producing a technical Core Audio error. Device changes use Windows
  app notifications under a dedicated application identity registered by the
  installer with the packaged MuteGuard PNG; clicking one opens Settings. The
  legacy tray notification remains only as a fallback if the modern channel
  cannot be opened.
- Custom feedback files are atomically replaced, limited to uncompressed
  16-bit PCM WAV, validated from RIFF chunks and internally consistent PCM
  metadata, and rejected above five seconds. Oversized files are rejected from
  metadata before being loaded by the runtime. Missing or invalid custom files
  fall back to the built-in tone. Each request uses an independent WinMM
  waveOut voice, so rapid mute/unmute cues overlap without cutting off or
  waiting for an earlier cue; simultaneous voices are bounded to 16. Settings
  previews are dispatched to the background process and use the same playback
  path as real mute changes. Backend rejection is reported and a rejected
  custom sound retries with the built-in tone.
- Update checks use WinHTTP against the fixed public GitHub Releases endpoint,
  require no account or token, use bounded request timeouts and reject responses
  above 1 MiB. Automatic checks run at most once per 24 hours and are
  coordinated across background and Settings processes with a named mutex.
  Strict numeric version comparison rejects malformed tags; only the exact x64
  setup asset and expected GitHub repository URLs can be opened. The result is
  cached separately from config.json, a notification is emitted only once per
  newer version, and installation requires an explicit user click.
- Diagnostics intentionally omits credentials and complete personal file
  paths. The copied report contains application, Windows, Core Audio, input,
  overlay and non-sensitive update status only.
- Portable and installer archives contain no Markdown files. The packaged
  executable retains `MuteGuard` file/product descriptions and version 1.4.0.
  Its PE machine field is `0x8664`, confirming an x64/AMD64 executable; the
  Diagnostics label presents this as `x64 (AMD64)` instead of Rust's `x86_64`.

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

Visible terminology is consistent across Settings, the Guide and microphone
notifications. The three color-source selectors share the single `System
color` label constant; `behavior`, `Built-in tone` and `default communications
microphone` are used consistently for their respective concepts. Unicode dash
punctuation is absent from the current tracked sources and documentation.

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
Every Settings section uses the same responsive masonry layout: one column in
the compact window and as many 320 px-or-wider columns as the available width
allows. General, Hotkeys, Overlay, Tray, Sound and Diagnostics now span the
whole content area instead of stopping at a separate maximum width. Card
heights are observed and reflowed independently, so a tall card no longer
creates an empty grid row below shorter neighbours. Main cards share one 20 px
padding token, and the Sound preview actions fill their common inner width.
Overlay columns use a 400 px responsive minimum so the section can form two
columns in the medium window. The position selector targets 376 px, shrinks only
when its card requires it and remains centered whenever the card is wider. Its
nine controls share aligned 84 x 32 px hit areas; inactive controls draw a small
centered point, while the selected anchor draws the miniature overlay preview.
The dark overlay background is `#131313`, and its displayed 100% scale now maps
to 85% of the previous geometry. MDI is the default microphone artwork.
Hotkey targets include the default communications microphone, every currently
active capture endpoint by its Windows friendly name, and `All microphones`.
A specific endpoint is persisted by its stable Windows device ID and is opened
directly when the hotkey fires; a saved disconnected endpoint remains visible
as unavailable instead of being silently replaced. The selector is searchable.
Configuration normalization now preserves those direct device IDs during both
Settings saves and subsequent reloads. A load-and-serialize regression test
covers the complete persistence path.
The `All microphones` target retains the distinct four-square group glyph.
Descriptive microphone cues in Hotkeys, Overlay visibility/content and
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
The `--exit-all` helper now waits for both the background and Settings windows
to close, retries delivery while an existing instance is still starting and
returns a nonzero process status on failure. The installer and uninstaller
check that status before replacing or deleting application files.

Settings now opens at 1200 x 740 logical pixels while retaining the existing
760 x 590 minimum size, so the responsive layouts can still be tested in their
compact form. A dedicated Guide entry is anchored at the bottom of the sidebar.
Its reference cards cover every primary Settings section and every exposed
control, including conditional options and hardware-reconnect behavior. Two
tests require every primary section to remain represented and every guide item
to have a unique topic location and non-empty explanation.
The Guide card grid alone uses one full-width column for long-form reading.
Its compact tablist selects one topic at a time, while each setting is presented
as an aligned label-description row that collapses cleanly on narrow windows.
All operational Settings sections retain the responsive masonry layout. The
Overlay Position picker remains 376 logical pixels wide and centered; at
narrower emergency widths, it contracts to the available content box instead
of overflowing the card's equal 20-pixel insets. Overlay masonry tracks reserve
the full 418 logical pixels required by that picker, both insets and borders.

## Package checks

- Portable directory: 39 files, including 34 hashed Dioxus assets, the root
  notification PNG and no Markdown documentation.
- Executable: references all 34 packaged asset names.
- ZIP: all 39 entries are byte-identical to the staging folder; none has a
  `.md` extension.
- The NSIS installer also excludes every `.md` from its recursive input and
  deletes root Markdown files left by an older installation before copying the
  new build.
- After confirming that both MuteGuard processes have stopped, the uninstaller
  removes `%LOCALAPPDATA%\muteguard` and `%APPDATA%\MuteGuard`. A locked file
  is retried and scheduled for deletion on reboot rather than leaving user
  data behind permanently.
- Windows resource: the purple slashed MDI microphone and version
  information are present. The source ICO contains eight validated PNG-backed
  sizes: 16, 20, 24, 32, 40, 48, 64 and 256 px.
- The 256 px frame is derived from the validated 1024 px canonical render,
  avoiding Edge's narrow headless-viewport offset. Generation rejects a
  missing or edge-clipped purple foreground. The frame extracted directly
  from the final installer has foreground bounds `56,52` through `199,203`.
- The icon glyph is pixel-bounds centered at `511.5 x 511.5` on the 1024 px
  source render, with symmetric padding (`224/224` horizontal and `208/208`
  vertical). All four corner pixels have alpha `0`; rounded-corner space is
  transparent rather than white.
- The Linux-hosted cross-build provides the MinGW `windres` and `ar` tools
  explicitly, so Dioxus completes its Windows resource prebuild without a
  warning. Inspection of the final Windows PE confirmed the `.rsrc` section,
  icon group and version resource.
- Version fields: product `MuteGuard`, file/product version `1.4.0`, original
  filename `muteguard.exe`. The application `FileDescription` is `MuteGuard`,
  so Task Manager uses the short product name; the installer description is
  `MuteGuard Setup`.
- PE format: 64-bit Windows GUI executable.
- PE hardening: `HIGH_ENTROPY_VA`, `DYNAMIC_BASE` and `NX_COMPAT` are enabled.
- No section is both writable and executable; release debug/symbol sections
  are stripped.

## Windows live regression

The user installed and exercised the final 1.1.6 release candidate that became
the 1.2.0 runtime baseline. The following checks passed on Windows before the
1.3.0 Guide and initial-window-size changes:

- the background instance and its `--settings` child both remained responsive;
- opening Settings produced a visible `MuteGuard Settings` window;
- every current Settings section rendered successfully;
- the tabs were cycled repeatedly without a panic, blank panel or unresponsive
  process;
- direct Logitech USB headset targeting persisted and controlled the intended
  endpoint;
- tray and overlay state followed real mute/unmute changes;
- custom/default sound feedback played completely and overlapping requests
  started immediately;
- disconnect/reconnect notifications used the MuteGuard PNG identity, avoided
  the transient technical error, and restored live tray updates;
- installer upgrades cleanly stopped the previous process.

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
| `dist/1.4.0/muteguard-1.4.0-windows-x64-portable.zip` | 8,505,820 bytes | `944534353E0C6107042B8D22598326B79666FBFB5CC41CB7C7EE09FF82FD5239` |
| `dist/1.4.0/muteguard-1.4.0-windows-x64-setup.zip` | 6,170,856 bytes | `345AAD512C540024E33A6AF985E7319AAB0DE66730132565C941BFE5853A536E` |
| Installer EXE (standalone and inside setup ZIP) | 6,212,397 bytes | `F2E3CC7484C1581FA40FDE10474C62D1F98E8D0BDD80B5616376959FAC8B9215` |
| Portable `muteguard.exe` | 20,042,240 bytes | `51FF659BCBFCD4415BC9B3C11E2CEFE72A989F9EC212D5A13DB3C1A508F55FB2` |
| Portable `WebView2Loader.dll` | 160,320 bytes | `8427B1FC58EC707813E5C0A51EB5D69397BB333250A7B891BE4D3B123F1E0F1C` |

## External/manual boundary

Trend Micro's reputation protection blocks new unsigned hashes launched from
PowerShell. The post-fix runtime test therefore used one manual Explorer launch
after the user's explicit approval; repeated automated launches were avoided.
Trend Micro removed earlier newly written setup executables after they had been
built and verified. At the time of this final verification, both the standalone
setup EXE and its byte-identical copy inside
`muteguard-1.4.0-windows-x64-setup.zip` are present; the ZIP remains the
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
