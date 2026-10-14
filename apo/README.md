# MuteGuard APO experiment

This is a standalone Windows system-effect APO experiment on the
`feature/microphone-apo` branch. It is adapted from Equalizer APO's
public-domain `MinimalAPO` example (see `License.txt`). It uses its own
CLSID, `{A4EC941D-F1BA-4C19-B195-4092858D9D90}`.

The current processing function halves every float sample. This provides a
simple measurable check that the APO code runs. It is not a replacement for
the current RNNoise, gate, and compressor chain.

Build `MuteGuardAPO.vcxproj` as Release/x64 with Visual Studio Build Tools
and the Windows SDK. The project uses the VS 2026 `v145` toolset. From a
Developer Command Prompt in this directory:

```bat
msbuild MuteGuardAPO.vcxproj /p:Configuration=Release /p:Platform=x64
cl /nologo /EHsc /MD test.cpp /link /out:test.exe
test.exe
```

The test loads the DLL through its COM class factory and checks the processing
function on a small sample buffer. No registry keys or audio endpoints are
modified by building or running the test.

To try the attenuation on this machine, open **PowerShell as Administrator**
in this directory after building the DLL:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\live-test.ps1 -Action Status
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\live-test.ps1 -Action Install
```

The installer finds the single active endpoint named `USB Audio CODEC`, saves
its current stream-effect value in `live-test-state.json` and exports the whole
`FxProperties` key to `live-test-FxProperties.reg`. It copies the DLL to
`%ProgramData%\MuteGuard\APO-test`, registers it, and replaces only the stream
effect value. This temporarily replaces Equalizer APO on that microphone;
RNNoise, gate, and compressor will not run during the test. Close and reopen
the recording app, or restart Windows if its audio graph does not refresh.
If the existing `Microphone APO Check` scheduled task is enabled, the script
disables it for the trial and restores its previous state afterward so it
cannot launch Equalizer APO's Device Selector at the next login.
Record a short sample with automatic gain control disabled: this APO should
lower the amplitude by half (about 6 dB). Check `Status` again after reopening
the app to see whether another process changed the registry value.

Restore Equalizer APO using the same elevated PowerShell window:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\live-test.ps1 -Action Restore
```

Reopen the recording app or restart Windows after restoring. The script leaves
the `.reg` export as an extra backup. Keep the state JSON until restoration is
complete; do not move it to another directory. The installed DLL remains on
disk after restoration but is no longer assigned to the endpoint.

This is still only a live test of the APO loading path. The MuteGuard branch
includes a read-only Microphone effects tab for endpoint status; effect
configuration and the desired modules remain to be built. Unsigned APOs
also require the Windows audio protection setting to allow loading, so this
version is unsuitable for general distribution.
