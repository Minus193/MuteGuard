# Quick release

Prerequisiti: versione e `RELEASE_NOTES.md` aggiornati, GitHub CLI autenticata
con `gh auth login --web`, e nessuna credenziale inserita in file o comandi.

```powershell
$cargoToml = Get-Content -Raw -Encoding UTF8 .\Cargo.toml
$version = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $version) { throw "Could not read Cargo.toml version" }

.\verify.cmd -Offline
.\build-docker.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\verify-release.ps1 -Version $version

git add -A
git commit -m "Release MuteGuard $version"
git tag -a "v$version" -m "MuteGuard $version"
$releaseRemote = git remote | Where-Object {
  (git remote get-url $_) -eq "https://github.com/Minus193/MuteGuard.git"
} | Select-Object -First 1
if (-not $releaseRemote) { throw "Canonical GitHub remote not found" }
git push $releaseRemote main --follow-tags

gh release create "v$version" `
  "dist/$version/muteguard-$version-windows-x64-setup.exe" `
  "dist/$version/muteguard-$version-windows-x64-setup.zip" `
  "dist/$version/muteguard-$version-windows-x64-portable.zip" `
  --title "MuteGuard $version" `
  --notes-file RELEASE_NOTES.md `
  --latest
```

After upload, verify the public tag, latest-release marker, artifact sizes, and
SHA-256 values anonymously. Remove temporary authentication after publishing.
