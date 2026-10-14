param(
    [switch]$Offline,
    [switch]$SkipTests,
    [switch]$SkipGitDiffCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)

    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Invoke-NativeChecked {
    param(
        [string]$Description,
        [string]$Command,
        [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Get-ProjectFiles {
    param(
        [string]$RepoRoot,
        [string]$Pattern
    )

    $files = @(& git -C $RepoRoot ls-files --cached --others --exclude-standard -- $Pattern)
    if ($LASTEXITCODE -ne 0) {
        throw "Could not list project files matching $Pattern"
    }
    $files
}

function Get-PackageVersion {
    param([string]$RepoRoot)

    $cargoToml = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $RepoRoot "Cargo.toml")
    $package = [regex]::Match($cargoToml, '(?ms)^\[package\]\s*(?<body>.*?)(?:^\[|\z)')
    if (-not $package.Success) {
        throw "Cargo.toml does not contain a [package] section"
    }
    $version = [regex]::Match(
        $package.Groups["body"].Value,
        '(?m)^\s*version\s*=\s*"(?<value>[^"]+)"'
    )
    if (-not $version.Success) {
        throw "Cargo.toml does not contain a package version"
    }
    $version.Groups["value"].Value
}

function Assert-VersionConsistency {
    param(
        [string]$RepoRoot,
        [string]$Version
    )

    $checks = @(
        @{
            Path = "Cargo.lock"
            Pattern = "(?ms)^\[\[package\]\]\s*name\s*=\s*`"muteguard`"\s*version\s*=\s*`"$([regex]::Escape($Version))`""
        },
        @{
            Path = "installer/muteguard-cross.nsi"
            Pattern = "(?m)^\s*!define\s+VERSION\s+`"$([regex]::Escape($Version))`"\s*$"
        },
        @{
            Path = "MUTEGUARD_DETAILS.md"
            Pattern = "(?m)^\| Versione documentata \| $([regex]::Escape($Version)) \|\s*$"
        },
        @{
            Path = "RELEASE_NOTES.md"
            Pattern = "(?m)^# MuteGuard $([regex]::Escape($Version))\s*$"
        }
    )

    foreach ($check in $checks) {
        $path = Join-Path $RepoRoot $check.Path
        $content = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
        if ($content -notmatch $check.Pattern) {
            throw "$($check.Path) is not synchronized with package version $Version"
        }
    }
}

function Assert-PowerShellParses {
    param([string]$Path)

    $tokens = $null
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile(
        $Path,
        [ref]$tokens,
        [ref]$errors
    )
    if ($errors.Count -gt 0) {
        $detail = ($errors | ForEach-Object { $_.Message }) -join [Environment]::NewLine
        throw "PowerShell parser errors in ${Path}:$([Environment]::NewLine)$detail"
    }
}

function Assert-MarkdownLinks {
    param(
        [string]$RepoRoot,
        [string[]]$Files
    )

    $missing = [System.Collections.Generic.List[string]]::new()
    foreach ($relativePath in $Files) {
        $path = Join-Path $RepoRoot $relativePath
        $content = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
        foreach ($match in [regex]::Matches($content, '\[[^\]]+\]\((?<target>[^)]+)\)')) {
            $target = $match.Groups["target"].Value.Trim().Trim('<', '>')
            if ($target -match '^(https?://|mailto:|#)') {
                continue
            }
            $pathPart = ($target -split '#', 2)[0]
            if ([string]::IsNullOrWhiteSpace($pathPart)) {
                continue
            }
            $resolved = Join-Path (Split-Path -Parent $path) $pathPart
            if (-not (Test-Path -LiteralPath $resolved)) {
                $missing.Add("$relativePath -> $target")
            }
        }
    }

    if ($missing.Count -gt 0) {
        throw "Missing local Markdown targets:$([Environment]::NewLine)$($missing -join [Environment]::NewLine)"
    }
}

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$version = Get-PackageVersion -RepoRoot $repoRoot
$cargoCommon = @("--locked")
if ($Offline) {
    $cargoCommon += "--offline"
}

Write-Step "Checking version consistency for MuteGuard $version"
Assert-VersionConsistency -RepoRoot $repoRoot -Version $version

Write-Step "Checking PowerShell, SVG, JSON, and Markdown files"
foreach ($relativePath in Get-ProjectFiles -RepoRoot $repoRoot -Pattern "*.ps1") {
    Assert-PowerShellParses -Path (Join-Path $repoRoot $relativePath)
}
foreach ($relativePath in Get-ProjectFiles -RepoRoot $repoRoot -Pattern "*.svg") {
    [void][xml](Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot $relativePath))
}
foreach ($relativePath in Get-ProjectFiles -RepoRoot $repoRoot -Pattern "*.webmanifest") {
    [void](Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot $relativePath) | ConvertFrom-Json)
}
Assert-MarkdownLinks -RepoRoot $repoRoot -Files (Get-ProjectFiles -RepoRoot $repoRoot -Pattern "*.md")

Write-Step "Checking Rust formatting"
Invoke-NativeChecked -Description "Rust formatting" -Command "cargo" -Arguments @(
    "fmt", "--all", "--", "--check"
)

Write-Step "Running Clippy with warnings denied"
Invoke-NativeChecked -Description "Clippy" -Command "cargo" -Arguments @(
    "clippy"
) + $cargoCommon + @(
    "--tests", "--", "-D", "warnings"
)

Write-Step "Running focused structural Clippy checks"
Invoke-NativeChecked -Description "Structural Clippy" -Command "cargo" -Arguments @(
    "clippy"
) + $cargoCommon + @(
    "--tests", "--",
    "-D", "warnings",
    "-D", "clippy::too_many_lines",
    "-D", "clippy::redundant_clone",
    "-D", "clippy::assigning_clones",
    "-D", "clippy::needless_pass_by_value",
    "-D", "clippy::needless_continue"
)

if (-not $SkipTests) {
    Write-Step "Running the Rust test suite"
    Invoke-NativeChecked -Description "Rust tests" -Command "cargo" -Arguments @(
        "test"
    ) + $cargoCommon
}

if (-not $SkipGitDiffCheck) {
    Write-Step "Checking Git whitespace errors"
    Invoke-NativeChecked -Description "Working-tree diff check" -Command "git" -Arguments @(
        "-C", $repoRoot, "diff", "--check"
    )
    Invoke-NativeChecked -Description "Index diff check" -Command "git" -Arguments @(
        "-C", $repoRoot, "diff", "--cached", "--check"
    )
}

Write-Host ""
Write-Host "MuteGuard source verification passed." -ForegroundColor Green
