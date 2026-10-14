param(
    [string]$Version,
    [string]$DistRoot,
    [switch]$AllowOtherDistVersions
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)

    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Get-PackageVersion {
    param([string]$RepoRoot)

    $cargoToml = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $RepoRoot "Cargo.toml")
    $package = [regex]::Match($cargoToml, '(?ms)^\[package\]\s*(?<body>.*?)(?:^\[|\z)')
    $versionMatch = [regex]::Match(
        $package.Groups["body"].Value,
        '(?m)^\s*version\s*=\s*"(?<value>[^"]+)"'
    )
    if (-not $package.Success -or -not $versionMatch.Success) {
        throw "Could not read the package version from Cargo.toml"
    }
    $versionMatch.Groups["value"].Value
}

function Assert-PathInside {
    param(
        [string]$Root,
        [string]$Path
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside the expected root: $resolvedPath"
    }
}

function Get-StreamSha256 {
    param([System.IO.Stream]$Stream)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($Stream))).Replace("-", "")
    }
    finally {
        $sha.Dispose()
    }
}

function Get-FileSha256 {
    param([string]$Path)

    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash
}

function Get-RelativePathInside {
    param(
        [string]$Root,
        [string]$Path
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside the expected root: $resolvedPath"
    }
    $resolvedPath.Substring($resolvedRoot.Length)
}

function Get-PeMachine {
    param([string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    $reader = [System.IO.BinaryReader]::new($stream)
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            throw "$Path is not a valid DOS/PE executable"
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadUInt32()
        if ($peOffset -gt $stream.Length - 6) {
            throw "$Path contains an invalid PE header offset"
        }
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw "$Path does not contain a PE signature"
        }
        $reader.ReadUInt16()
    }
    finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Assert-FileVersion {
    param(
        [string]$Path,
        [string]$ExpectedVersion
    )

    $expected = [version]"$ExpectedVersion.0"
    $info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
    $actual = [version]::new(
        $info.FileMajorPart,
        $info.FileMinorPart,
        $info.FileBuildPart,
        $info.FilePrivatePart
    )
    if ($actual -ne $expected) {
        throw "$Path has file version $actual; expected $expected"
    }
    if ($info.ProductName -ne "MuteGuard") {
        throw "$Path has unexpected product name '$($info.ProductName)'"
    }
}

function Assert-PortableArchive {
    param(
        [string]$ArchivePath,
        [string]$PortableDir
    )

    $archive = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $entries = @($archive.Entries | Where-Object { -not $_.FullName.Replace('\', '/').EndsWith("/") })
        $entryNames = @($entries | ForEach-Object { $_.FullName.Replace('\', '/') })
        $markdown = @($entryNames | Where-Object { [System.IO.Path]::GetExtension($_) -ieq ".md" })
        if ($markdown.Count -gt 0) {
            throw "Portable archive contains Markdown files: $($markdown -join ', ')"
        }

        $required = @(
            "muteguard.exe",
            "WebView2Loader.dll",
            "LICENSE",
            "muteguard.ico",
            "muteguard.png",
            "apo/live-test.ps1",
            "apo/x64/Release/MuteGuardAPO.dll",
            "apo/rnnoise/weights_blob.bin"
        )
        foreach ($requiredName in $required) {
            if ($entryNames -notcontains $requiredName) {
                throw "Portable archive is missing $requiredName"
            }
        }

        $files = @(Get-ChildItem -LiteralPath $PortableDir -Recurse -File)
        $relativeFiles = @(
            $files | ForEach-Object {
                (Get-RelativePathInside -Root $PortableDir -Path $_.FullName).Replace('\', '/')
            }
        )
        $missingFromZip = @($relativeFiles | Where-Object { $entryNames -notcontains $_ })
        $extraInZip = @($entryNames | Where-Object { $relativeFiles -notcontains $_ })
        if ($missingFromZip.Count -gt 0 -or $extraInZip.Count -gt 0) {
            throw "Portable ZIP and staging folder differ. Missing: $($missingFromZip -join ', '). Extra: $($extraInZip -join ', ')"
        }

        foreach ($entry in $entries) {
            $relativePath = $entry.FullName.Replace('\', '/')
            $filePath = Join-Path $PortableDir ($relativePath.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
            $stream = $entry.Open()
            try {
                $entryHash = Get-StreamSha256 -Stream $stream
            }
            finally {
                $stream.Dispose()
            }
            $fileHash = Get-FileSha256 -Path $filePath
            if ($entryHash -ne $fileHash) {
                throw "Portable ZIP entry differs from staging file: $relativePath"
            }
        }
    }
    finally {
        $archive.Dispose()
    }
}

function Assert-SetupArchive {
    param(
        [string]$ArchivePath,
        [string]$InstallerPath,
        [string]$ExpectedEntryName
    )

    $archive = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $entries = @($archive.Entries | Where-Object { -not $_.FullName.Replace('\', '/').EndsWith("/") })
        if ($entries.Count -ne 1 -or $entries[0].FullName -ne $ExpectedEntryName) {
            throw "Setup ZIP must contain only $ExpectedEntryName"
        }
        if ([System.IO.Path]::GetExtension($entries[0].FullName) -ieq ".md") {
            throw "Setup ZIP contains a Markdown file"
        }
        $stream = $entries[0].Open()
        try {
            $archivedHash = Get-StreamSha256 -Stream $stream
        }
        finally {
            $stream.Dispose()
        }
        $standaloneHash = Get-FileSha256 -Path $InstallerPath
        if ($archivedHash -ne $standaloneHash -or $entries[0].Length -ne (Get-Item -LiteralPath $InstallerPath).Length) {
            throw "Standalone setup EXE is not byte-identical to the setup ZIP entry"
        }
    }
    finally {
        $archive.Dispose()
    }
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = Get-PackageVersion -RepoRoot $repoRoot
}
if ([string]::IsNullOrWhiteSpace($DistRoot)) {
    $DistRoot = Join-Path $repoRoot "dist"
}

$versionRoot = Join-Path $DistRoot $Version
$portableDir = Join-Path $versionRoot "muteguard-$Version-windows-x64-portable"
$portableZip = "$portableDir.zip"
$setupName = "muteguard-$Version-windows-x64-setup.exe"
$setupExe = Join-Path $versionRoot $setupName
$setupZip = Join-Path $versionRoot "muteguard-$Version-windows-x64-setup.zip"

Assert-PathInside -Root $DistRoot -Path $versionRoot
foreach ($path in @($portableDir, $portableZip, $setupExe, $setupZip)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required release artifact is missing: $path"
    }
}

if (-not $AllowOtherDistVersions) {
    $otherVersions = @(
        Get-ChildItem -LiteralPath $DistRoot -Directory |
            Where-Object { $_.Name -ne $Version }
    )
    if ($otherVersions.Count -gt 0) {
        throw "dist contains release directories other than ${Version}: $($otherVersions.Name -join ', ')"
    }
}

Add-Type -AssemblyName System.IO.Compression.FileSystem

Write-Step "Checking portable archive contents"
Assert-PortableArchive -ArchivePath $portableZip -PortableDir $portableDir

Write-Step "Checking setup archive equality"
Assert-SetupArchive -ArchivePath $setupZip -InstallerPath $setupExe -ExpectedEntryName $setupName

Write-Step "Checking PE architecture and version resources"
$portableExe = Join-Path $portableDir "muteguard.exe"
$portableMachine = Get-PeMachine -Path $portableExe
if ($portableMachine -ne 0x8664) {
    throw "$portableExe has PE machine 0x$($portableMachine.ToString('X4')); expected x64 0x8664"
}
$apoDll = Join-Path $portableDir "apo\x64\Release\MuteGuardAPO.dll"
$apoMachine = Get-PeMachine -Path $apoDll
if ($apoMachine -ne 0x8664) {
    throw "$apoDll has PE machine 0x$($apoMachine.ToString('X4')); expected x64 0x8664"
}
$setupMachine = Get-PeMachine -Path $setupExe
if ($setupMachine -notin @(0x014C, 0x8664)) {
    throw "$setupExe has unsupported PE machine 0x$($setupMachine.ToString('X4'))"
}
foreach ($path in @($portableExe, $setupExe)) {
    Assert-FileVersion -Path $path -ExpectedVersion $Version
}

Write-Step "Computing release hashes"
$artifacts = @(
    $portableZip,
    $setupZip,
    $setupExe,
    $portableExe,
    (Join-Path $portableDir "WebView2Loader.dll"),
    (Join-Path $portableDir "apo\x64\Release\MuteGuardAPO.dll"),
    (Join-Path $portableDir "apo\rnnoise\weights_blob.bin")
)
$manifest = @(
    foreach ($path in $artifacts) {
        $item = Get-Item -LiteralPath $path
        [pscustomobject]@{
            Artifact = Get-RelativePathInside -Root $versionRoot -Path $item.FullName
            Size = $item.Length
            SHA256 = Get-FileSha256 -Path $item.FullName
        }
    }
)
$manifest | Format-Table -AutoSize

Write-Host ""
Write-Host "MuteGuard $Version release artifact verification passed." -ForegroundColor Green
