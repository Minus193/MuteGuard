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

The DLL is intentionally not installed or assigned to an endpoint. Before
using it on a live microphone, the remaining work is to implement the desired
effects and configuration, build a safe endpoint installer with backup and
rollback, verify the Windows audio engine actually loads it, and add the
MuteGuard settings UI. Unsigned APOs also require the relevant Windows audio
protection setting to allow loading; that makes this unsuitable for general
distribution in its current form.
