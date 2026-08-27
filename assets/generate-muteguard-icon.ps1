Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$assetRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$sourcePath = Join-Path $assetRoot "muteguard-source.svg"
$pngPath = Join-Path $assetRoot "muteguard.png"
$icoPath = Join-Path $assetRoot "muteguard.ico"
$edgePath = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

function Assert-PngSize {
    param(
        [string]$Path,
        [int]$ExpectedSize
    )

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 24) {
        throw "Rendered PNG is incomplete: $Path"
    }
    $width = [System.Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($bytes, 16))
    $height = [System.Net.IPAddress]::NetworkToHostOrder([BitConverter]::ToInt32($bytes, 20))
    if ($width -ne $ExpectedSize -or $height -ne $ExpectedSize) {
        throw "Rendered PNG has size ${width}x${height}; expected ${ExpectedSize}x${ExpectedSize}"
    }
}

if (-not (Test-Path -LiteralPath $edgePath -PathType Leaf)) {
    throw "Microsoft Edge was not found at $edgePath"
}
if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "Icon source was not found at $sourcePath"
}

$temporaryRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
$renderRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $temporaryRoot ("muteguard-icon-" + [guid]::NewGuid().ToString("N")))
)
if (-not $renderRoot.StartsWith($temporaryRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to use a render directory outside the system temporary directory: $renderRoot"
}
New-Item -ItemType Directory -Path $renderRoot | Out-Null

try {
    $sourceUri = ([System.Uri]$sourcePath).AbsoluteUri
    $sizes = @(16, 20, 24, 32, 40, 48, 64, 256)
    $rendered = @()

    foreach ($size in @($sizes + 1024)) {
        $profilePath = Join-Path $renderRoot ("edge-profile-{0}" -f $size)
        New-Item -ItemType Directory -Path $profilePath | Out-Null
        $outputPath = if ($size -eq 1024) {
            $pngPath
        } else {
            Join-Path $renderRoot ("muteguard-{0}.png" -f $size)
        }
        if (Test-Path -LiteralPath $outputPath -PathType Leaf) {
            Remove-Item -LiteralPath $outputPath -Force
        }
        $arguments = @(
            "--headless=new"
            "--disable-gpu"
            "--disable-background-networking"
            "--disable-sync"
            "--hide-scrollbars"
            "--default-background-color=00000000"
            "--no-first-run"
            "--no-default-browser-check"
            "--force-device-scale-factor=1"
            "--user-data-dir=$profilePath"
            "--window-size=$size,$size"
            "--screenshot=$outputPath"
            $sourceUri
        )
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            & $edgePath $arguments 2>$null | Out-Null
            $edgeExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        if ($edgeExitCode -ne 0 -or -not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
            throw "Edge failed to render the ${size}px icon"
        }
        Assert-PngSize -Path $outputPath -ExpectedSize $size
        if ($size -ne 1024) {
            $rendered += [pscustomobject]@{ Size = $size; Path = $outputPath }
        }
    }

    $streams = @()
    try {
        foreach ($image in $rendered) {
            $streams += [pscustomobject]@{
                Size = $image.Size
                Bytes = [System.IO.File]::ReadAllBytes($image.Path)
            }
        }

        $fileStream = [System.IO.File]::Open($icoPath, [System.IO.FileMode]::Create)
        $writer = [System.IO.BinaryWriter]::new($fileStream)
        try {
            $writer.Write([uint16]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]$streams.Count)

            $offset = 6 + (16 * $streams.Count)
            foreach ($image in $streams) {
                $dimension = if ($image.Size -eq 256) { 0 } else { $image.Size }
                $writer.Write([byte]$dimension)
                $writer.Write([byte]$dimension)
                $writer.Write([byte]0)
                $writer.Write([byte]0)
                $writer.Write([uint16]1)
                $writer.Write([uint16]32)
                $writer.Write([uint32]$image.Bytes.Length)
                $writer.Write([uint32]$offset)
                $offset += $image.Bytes.Length
            }

            foreach ($image in $streams) {
                $writer.Write([byte[]]$image.Bytes)
            }
        } finally {
            $writer.Dispose()
            $fileStream.Dispose()
        }
    } finally {
        $streams = @()
    }
} finally {
    if (Test-Path -LiteralPath $renderRoot) {
        Remove-Item -LiteralPath $renderRoot -Recurse -Force
    }
}
