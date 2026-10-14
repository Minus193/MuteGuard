param(
    [string]$BuilderImage = "muteguard-builder:rust-1.98-dx-0.7.6",
    [string]$CargoVolume = "muteguard-cargo",
    [string]$TargetVolume = "muteguard-target",
    [switch]$SkipChecks
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)

    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Invoke-Checked {
    param(
        [string]$Description,
        [scriptblock]$Command
    )

    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Assert-SafeBuildPath {
    param(
        [string]$RepoRoot,
        [string]$Path
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd('\') + '\'
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to modify a path outside the repository: $resolvedPath"
    }
}

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$cargoToml = Get-Content (Join-Path $repoRoot "Cargo.toml") -Raw
$versionMatch = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"(?<version>[^"]+)"')
if (-not $versionMatch.Success) {
    throw "Could not read the package version from Cargo.toml"
}

$version = $versionMatch.Groups["version"].Value
$versionRoot = Join-Path $repoRoot "dist\$version"
$portableDir = Join-Path $versionRoot "muteguard-$version-windows-x64-portable"
$zipPath = "$portableDir.zip"
$installerPath = Join-Path $versionRoot "muteguard-$version-windows-x64-setup.exe"
$installerZipPath = Join-Path $versionRoot "muteguard-$version-windows-x64-setup.zip"

Assert-SafeBuildPath -RepoRoot $repoRoot -Path $versionRoot
Assert-SafeBuildPath -RepoRoot $repoRoot -Path $portableDir
Assert-SafeBuildPath -RepoRoot $repoRoot -Path $installerZipPath

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw "Docker was not found in PATH"
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Visual Studio Build Tools are required to build the x64 microphone APO.'
}
$msbuild = & $vswhere -latest -products '*' -requires Microsoft.Component.MSBuild `
    -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
if (-not $msbuild) { throw 'MSBuild was not found by Visual Studio Installer.' }
$apoDll = Join-Path $repoRoot 'apo\x64\Release\MuteGuardAPO.dll'
$apoModel = Join-Path $repoRoot 'apo\rnnoise\weights_blob.bin'
if (-not (Test-Path -LiteralPath $apoModel)) { throw 'RNNoise model is missing.' }

Write-Step "Building the x64 microphone APO"
& $msbuild (Join-Path $repoRoot 'apo\MuteGuardAPO.vcxproj') `
    /p:Configuration=Release /p:Platform=x64 /m /nologo /v:minimal
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $apoDll)) {
    throw "Microphone APO build failed with exit code $LASTEXITCODE"
}

$repoMount = $repoRoot.Replace('\', '/')
$portableMount = $portableDir.Replace('\', '/')

if (-not $SkipChecks) {
    Write-Step "Running offline source and Windows x64 compile checks"
    Invoke-Checked -Description "Source verification" -Command {
        docker run --rm --network none `
            -e CARGO_HOME=/cargo `
            -e WINDRES=x86_64-w64-mingw32-windres `
            -e AR=x86_64-w64-mingw32-ar `
            -v "${repoMount}:/workspace" `
            -v "${CargoVolume}:/cargo" `
            -v "${TargetVolume}:/workspace/target" `
            $BuilderImage `
            sh -c 'cargo fmt --all -- --check && cargo clippy --offline --locked --tests --target x86_64-pc-windows-gnu -- -D warnings && cargo clippy --offline --locked --tests --target x86_64-pc-windows-gnu -- -D warnings -D clippy::too_many_lines -D clippy::redundant_clone -D clippy::assigning_clones -D clippy::needless_pass_by_value -D clippy::needless_continue && cargo test --offline --locked --no-run --target x86_64-pc-windows-gnu'
    }
}

Write-Step "Building the Dioxus Windows x64 application offline"
Invoke-Checked -Description "Dioxus build" -Command {
    docker run --rm --network none `
        -e CARGO_HOME=/cargo `
        -e WINDRES=x86_64-w64-mingw32-windres `
        -e AR=x86_64-w64-mingw32-ar `
        -v "${repoMount}:/workspace" `
        -v "${CargoVolume}:/cargo" `
        -v "${TargetVolume}:/workspace/target" `
        $BuilderImage `
        dx build --desktop --release --target x86_64-pc-windows-gnu --frozen
}

Write-Step "Assembling the portable package from the Dioxus output"
New-Item -ItemType Directory -Path $versionRoot -Force | Out-Null
if (Test-Path $portableDir) {
    Remove-Item -LiteralPath $portableDir -Recurse -Force
}
New-Item -ItemType Directory -Path $portableDir -Force | Out-Null

Invoke-Checked -Description "Portable application copy" -Command {
    docker run --rm --network none `
        -v "${TargetVolume}:/target:ro" `
        -v "${portableMount}:/out" `
        $BuilderImage `
        sh -c 'cp -a /target/dx/muteguard/release/linux/app/. /out/ && loader=$(find /target/x86_64-pc-windows-gnu/desktop-release/build -path "*/out/x64/WebView2Loader.dll" -type f -print -quit) && test -n "$loader" && cp "$loader" /out/WebView2Loader.dll'
}

$dioxusExecutable = Join-Path $portableDir "muteguard"
if (-not (Test-Path $dioxusExecutable)) {
    throw "The Dioxus executable was not found in the portable output"
}
Move-Item -LiteralPath $dioxusExecutable -Destination (Join-Path $portableDir "muteguard.exe")
Copy-Item -LiteralPath (Join-Path $repoRoot "LICENSE") -Destination $portableDir
Copy-Item -LiteralPath (Join-Path $repoRoot "assets\muteguard.ico") -Destination $portableDir
Copy-Item -LiteralPath (Join-Path $repoRoot "assets\muteguard.png") -Destination $portableDir
$apoBundle = Join-Path $portableDir 'apo'
New-Item -ItemType Directory -Path (Join-Path $apoBundle 'x64\Release'), (Join-Path $apoBundle 'rnnoise') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot 'apo\live-test.ps1') -Destination $apoBundle
Copy-Item -LiteralPath $apoDll -Destination (Join-Path $apoBundle 'x64\Release')
Copy-Item -LiteralPath $apoModel -Destination (Join-Path $apoBundle 'rnnoise')

if (Test-Path $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path (Join-Path $portableDir "*") -DestinationPath $zipPath -Force

Write-Step "Building the NSIS installer offline"
$installerContainerPath = "/workspace/dist/$version/muteguard-$version-windows-x64-setup.exe"
$installerZipContainerPath = "/workspace/dist/$version/muteguard-$version-windows-x64-setup.zip"
$portableContainerPath = "/workspace/dist/$version/muteguard-$version-windows-x64-portable"
$temporaryInstallerPath = "/tmp/muteguard-$version-windows-x64-setup.exe"
$installerBuildCommand = "set -eu; " +
    "makensis -DAPP_DIR=$portableContainerPath -DOUTPUT_FILE=$temporaryInstallerPath " +
    "-DAPP_ICON=/workspace/assets/muteguard.ico -DVERSION=$version " +
    "/workspace/installer/muteguard-cross.nsi; " +
    "rm -f $installerZipContainerPath; " +
    "zip -j -9 $installerZipContainerPath $temporaryInstallerPath >/dev/null; " +
    "cp $temporaryInstallerPath $installerContainerPath"
Invoke-Checked -Description "NSIS installer build" -Command {
    docker run --rm --network none `
        -v "${repoMount}:/workspace" `
        $BuilderImage `
        sh -c $installerBuildCommand
}

if (-not (Test-Path $installerZipPath)) {
    throw "The archived NSIS installer was not produced: $installerZipPath"
}
if (-not (Test-Path $installerPath)) {
    Write-Warning "The setup EXE was removed after the build; use the verified copy in $installerZipPath"
}

Write-Step "Verifying release artifacts"
& (Join-Path $repoRoot "scripts\verify-release.ps1") `
    -Version $version `
    -AllowOtherDistVersions

Write-Host ""
Write-Host "Build outputs are ready in $versionRoot" -ForegroundColor Green
