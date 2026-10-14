# MuteGuard 1.5.3

- Added a Microphone effects tab with adjustable RNNoise strength, noise gate,
  and compressor settings for the selected microphone.
- Added the x64 MuteGuard audio processing component and its model to the
  portable ZIP and installer. Activation is available from the new tab.
- Deactivating the component disconnects it from the microphone without
  reconnecting an older stream effect.
- Added one-command local source verification and automatic GitHub CI checks.
- Added repeatable release artifact validation for ZIP contents, PE metadata,
  versions, installer equality, sizes, and SHA-256 hashes.
- Pinned the Rust 1.98 toolchain and removed the updater test's manual
  future-version bump.

The microphone APO is experimental and unsigned. Windows may refuse to load it
on systems that require signed audio effects; the standard MuteGuard features
do not depend on it.
