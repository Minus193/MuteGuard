# MuteGuard APO experiment

This is a standalone Windows system-effect APO experiment on the
`feature/microphone-apo` branch. It is adapted from Equalizer APO's
public-domain `MinimalAPO` example (see `License.txt`). It uses its own
CLSID, `{A4EC941D-F1BA-4C19-B195-4092858D9D90}`.

The earlier 50% attenuation proof has been replaced with a noise gate and a
compressor. Their thresholds, timings, and ratio are fixed to approximate the
user's current VST settings. In MuteGuard's Microphone effects tab, select the
microphone while this APO is linked to it and switch the two modules on or off.
MuteGuard writes `%ProgramData%\MuteGuard\APO-test\effects.ini`; the APO reads
it when the audio graph opens. Close and reopen the recording app after a
change. An absent file disables both modules and leaves samples unchanged.
RNNoise is not yet integrated, so this is still not a full Equalizer APO
replacement.

Build `MuteGuardAPO.vcxproj` as Release/x64 with Visual Studio Build Tools
and the Windows SDK. The project uses the VS 2026 `v145` toolset. From a
Developer Command Prompt in this directory:

```bat
msbuild MuteGuardAPO.vcxproj /p:Configuration=Release /p:Platform=x64
cl /nologo /EHsc /MD test.cpp /link /out:test.exe
test.exe
```

The test loads the DLL through its COM class factory and checks bypass, gate,
and compressor processing. No registry keys or audio endpoints are
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
Check `Status` again after reopening the app to see whether another process
changed the registry value. Switch on gate and compressor in the MuteGuard tab
and make a short recording to compare against Equalizer APO. The tab only
shows controls when MuteGuard's APO is linked to the selected microphone.

Restore Equalizer APO using the same elevated PowerShell window:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\live-test.ps1 -Action Restore
```

Reopen the recording app or restart Windows after restoring. The script leaves
the `.reg` export as an extra backup. Keep the state JSON until restoration is
complete; do not move it to another directory. The installed DLL remains on
disk after restoration but is no longer assigned to the endpoint.

This remains an experimental audio path. The tab configures gate and compressor,
but RNNoise, GUI installation/restoration, and runtime audio validation are
still needed. Unsigned APOs
also require the Windows audio protection setting to allow loading, so this
version is unsuitable for general distribution.
