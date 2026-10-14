# MuteGuard microphone APO experiment

This is an unsigned Windows x64 system-effect APO on the
`feature/microphone-apo` branch. It replaces the stream effect associated with
one selected capture endpoint while the experiment is active. The original
registry value is backed up and can be restored from MuteGuard's Microphone
effects tab. The prototype is adapted from Equalizer APO's public-domain
MinimalAPO sample (`License.txt`) but does not run Equalizer APO.

The effect chain is RNNoise, gate, then compressor. RNNoise uses a fixed 480
sample frame and runs only when the audio graph uses 48 kHz. At any other rate
RNNoise is bypassed while gate and compressor remain available. RNNoise adds
about 20 ms of latency. The tab adjusts RNNoise strength, gate threshold/hold/
release, and compressor threshold/ratio/attack/release. Existing settings keep
their original values. The switches and values are stored in
`%ProgramData%\MuteGuard\APO-test\effects.ini` and read when the recording
app opens its audio graph again. The APO never reads files or allocates memory
from its real-time processing callback.

`rnnoise/` contains the BSD-licensed Xiph RNNoise sources needed by the APO.
The 5.5 MB `weights_blob.bin` was generated from the model in
`werman/noise-suppression-for-voice` commit
`9c4e5c28d8950e2cef837d8a0abd36c2fd9b5c2d`, under
`external/rnnoise`, using its `write_weights.c` with the output file opened in
binary mode (`wb`). SHA-256:
`35da861b090c25df53ba7f258b1ea55b1ea36d0c9063d9495bc145ada945c90e`.
The original source and model license is in `rnnoise/COPYING`.

Build `MuteGuardAPO.vcxproj` as Release/x64 with Visual Studio Build Tools and
the Windows SDK. Build the Rust app with `cargo build`. The tab can then run
`live-test.ps1` with UAC approval, without a console window. It requires
`apo/live-test.ps1`, `apo/x64/Release/MuteGuardAPO.dll`, and
`apo/rnnoise/weights_blob.bin` to remain beside the source checkout; a packaged
app must carry them in its `apo` subdirectory. Select the microphone and use
**Activate MuteGuard APO**. Reopen the recording app. Use **Restore previous
effect** to return to Equalizer APO. The saved state and registry export remain
under `%ProgramData%\MuteGuard\APO-test` until restoration; the `.reg` export
is retained afterward as an extra backup. The previous Equalizer APO guard
task is disabled for the test and re-enabled by restore if it was enabled
before activation. Activation and restoration restart Windows Audio Endpoint
Builder so Windows loads the new effect chain; other audio pauses briefly.

The same script can be run directly as Administrator with `-Action Status`,
`Install`, or `Restore` for recovery. It refuses to overwrite an unexpected
stream-effect value. Building and running `test.cpp` and `rnnoise/smoke.cpp`
does not change Windows audio settings.

This branch has not been validated with a real recording using the complete
chain. An unsigned APO also needs the Windows audio protection setting to
allow loading, so this experiment is not ready for a public release.
