# Quick release

Prerequisiti: versione e `RELEASE_NOTES.md` aggiornati, artifact già generati
con `./build-docker.ps1`, GitHub CLI autenticata con `gh auth login --web`.

```powershell
$version = "1.5.2"

git add -A
git commit -m "Release MuteGuard $version"
git tag -a "v$version" -m "MuteGuard $version"
git push origin main --follow-tags

gh release create "v$version" `
  "dist/$version/muteguard-$version-windows-x64-setup.exe" `
  "dist/$version/muteguard-$version-windows-x64-setup.zip" `
  "dist/$version/muteguard-$version-windows-x64-portable.zip" `
  --title "MuteGuard $version" `
  --notes-file RELEASE_NOTES.md `
  --latest
```
