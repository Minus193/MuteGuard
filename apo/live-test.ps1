param(
    [Parameter(Mandatory)]
    [ValidateSet('Install', 'Status', 'Restore')]
    [string]$Action,
    [string]$EndpointGuid = ''
)

$ErrorActionPreference = 'Stop'
$reportPath = Join-Path $PSScriptRoot 'last-action-error.txt'
trap {
    $_.Exception.Message | Set-Content -LiteralPath $reportPath -Encoding UTF8
    exit 1
}
Remove-Item -LiteralPath $reportPath -ErrorAction SilentlyContinue
$capturePath = 'SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Capture'
$propertyName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5'
$friendlyNameProperty = '{b3f8fa53-0004-438e-9003-51a46e139bfc},6'
$apoClsid = '{A4EC941D-F1BA-4C19-B195-4092858D9D90}'
$installDirectory = Join-Path $env:ProgramData 'MuteGuard\APO-test'
$statePath = Join-Path $installDirectory 'live-test-state.json'
$backupPath = Join-Path $installDirectory 'live-test-FxProperties.reg'
$installedDll = Join-Path $installDirectory 'MuteGuardAPO.dll'
$sourceDll = Join-Path $PSScriptRoot 'x64\Release\MuteGuardAPO.dll'
$sourceModel = Join-Path $PSScriptRoot 'rnnoise\weights_blob.bin'
$guardTaskName = 'Microphone APO Check'

function Invoke-ApoRegistration([string]$dll, [bool]$unregister) {
    $arguments = if ($unregister) { @('/u', '/s', "`"$dll`"") }
                 else { @('/s', "`"$dll`"") }
    $process = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\regsvr32.exe') `
        -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) {
        $operation = if ($unregister) { 'unregistration' } else { 'registration' }
        throw "APO $operation failed (code $($process.ExitCode))."
    }
}

function Test-ApoRegistered {
    (Test-Path "HKLM:\SOFTWARE\Classes\CLSID\$apoClsid") -or
    (Test-Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio\AudioProcessingObjects\$apoClsid")
}

function Open-Endpoint([string]$guid, [bool]$writable) {
    if ($guid -notmatch '^\{[0-9a-fA-F-]{36}\}$') {
        throw 'EndpointGuid must be a GUID in braces.'
    }
    $root = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $endpoint = $root.OpenSubKey("$capturePath\$guid", $writable)
    if ($null -eq $endpoint) { throw "Capture endpoint $guid was not found." }
    return $endpoint
}

function Find-Endpoint {
    if ($EndpointGuid) { return $EndpointGuid }
    $root = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $capture = $root.OpenSubKey($capturePath)
    $matches = @(
        foreach ($guid in $capture.GetSubKeyNames()) {
            $endpoint = $capture.OpenSubKey($guid)
            $properties = $endpoint.OpenSubKey('Properties')
            if ($endpoint.GetValue('DeviceState') -eq 1 -and
                $null -ne $properties -and
                ([string]$properties.GetValue($friendlyNameProperty)).Trim() -eq 'USB Audio CODEC') {
                $guid
            }
        }
    )
    if ($matches.Count -ne 1) {
        throw "Found $($matches.Count) active USB Audio CODEC endpoints; pass -EndpointGuid explicitly."
    }
    return $matches[0]
}

function Read-Effect([string]$guid, [bool]$writable) {
    $null = Open-Endpoint $guid $false
    $root = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $fxPath = "$capturePath\$guid\FxProperties"
    $fx = if ($writable) {
        $rights = [Security.AccessControl.RegistryRights]::QueryValues -bor
                  [Security.AccessControl.RegistryRights]::SetValue
        $root.OpenSubKey($fxPath,
            [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree, $rights)
    } else {
        $root.OpenSubKey($fxPath)
    }
    if ($null -eq $fx) { throw "FxProperties is unavailable for $guid." }
    if ($propertyName -notin $fx.GetValueNames()) {
        throw "No stream-effect value exists for $guid; refusing to add one."
    }
    return $fx
}

if ($Action -eq 'Status') {
    $guid = if (Test-Path $statePath) { (Get-Content $statePath -Raw | ConvertFrom-Json).EndpointGuid } else { Find-Endpoint }
    $fx = Read-Effect $guid $false
    Write-Host "Microphone: $guid"
    Write-Host "Stream effect: $($fx.GetValue($propertyName))"
    Write-Host "Test state saved: $(Test-Path $statePath)"
    exit 0
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Open PowerShell as Administrator for Install or Restore.'
}

if ($Action -eq 'Install') {
    if (Test-Path $statePath) { throw 'A live test is already recorded. Restore it first.' }
    if (-not (Test-Path $sourceDll)) { throw "Build the Release/x64 DLL first: $sourceDll" }
    if (-not (Test-Path $sourceModel)) { throw "RNNoise model is missing: $sourceModel" }
    if ((Get-FileHash -LiteralPath $sourceModel -Algorithm SHA256).Hash -ne
        '35DA861B090C25DF53BA7F258B1EA55B1EA36D0C9063D9495BC145ADA945C90E') {
        throw 'RNNoise model checksum mismatch.'
    }
    $guid = Find-Endpoint
    $fx = Read-Effect $guid $true
    $originalValue = [string]$fx.GetValue($propertyName)
    $originalKind = $fx.GetValueKind($propertyName)
    if ($originalKind -ne [Microsoft.Win32.RegistryValueKind]::String -or $originalValue -eq $apoClsid) {
        throw "Unexpected existing stream effect ($originalKind): $originalValue"
    }
    New-Item -ItemType Directory -Path $installDirectory -Force | Out-Null
    $nativePath = "HKLM\$capturePath\$guid\FxProperties"
    & reg.exe export $nativePath $backupPath /y | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not export the original FxProperties key.' }
    $guardTask = Get-ScheduledTask -TaskName $guardTaskName -ErrorAction SilentlyContinue
    $guardWasEnabled = $null -ne $guardTask -and $guardTask.State -ne 'Disabled'
    [pscustomobject]@{
        EndpointGuid = $guid
        OriginalEffect = $originalValue
        OriginalKind = $originalKind.ToString()
        InstalledDll = $installedDll
        GuardWasEnabled = $guardWasEnabled
    } | ConvertTo-Json | Set-Content -LiteralPath $statePath -Encoding UTF8

    try {
        if ($guardWasEnabled) {
            Disable-ScheduledTask -TaskName $guardTaskName | Out-Null
            if ($guardTask.State -eq 'Running') {
                Stop-ScheduledTask -TaskName $guardTaskName
            }
        }
        New-Item -ItemType Directory -Path (Split-Path $installedDll) -Force | Out-Null
        Copy-Item -LiteralPath $sourceDll -Destination $installedDll -Force
        Copy-Item -LiteralPath $sourceModel -Destination (Join-Path (Split-Path $installedDll) 'weights_blob.bin') -Force
        $effectsPath = Join-Path (Split-Path $installedDll) 'effects.ini'
        if (-not (Test-Path $effectsPath)) {
            "[Effects]`nNoise=1`nGate=1`nCompressor=1" |
                Set-Content -LiteralPath $effectsPath -Encoding ASCII
        }
        & icacls.exe $effectsPath /grant '*S-1-5-32-545:M' | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Could not allow the MuteGuard app to save effect settings.' }
        Invoke-ApoRegistration $installedDll $false
        $fx.SetValue($propertyName, $apoClsid, [Microsoft.Win32.RegistryValueKind]::String)
    } catch {
        $installError = $_
        try {
            if ($fx.GetValue($propertyName) -ne $originalValue) {
                $fx.SetValue($propertyName, $originalValue, $originalKind)
            }
        } finally {
            if (Test-ApoRegistered) { Invoke-ApoRegistration $installedDll $true }
            if ($guardWasEnabled) { Enable-ScheduledTask -TaskName $guardTaskName | Out-Null }
        }
        throw $installError
    }
    Write-Host "Installed on $guid. Original effect: $originalValue"
    Write-Host 'Close and reopen your microphone app; if the effect does not change, restart Windows.'
    Write-Host 'Run live-test.ps1 -Action Restore to return to the original effect.'
    exit 0
}

if (-not (Test-Path $statePath)) { throw 'No live-test state file exists to restore.' }
$state = Get-Content $statePath -Raw | ConvertFrom-Json
$fx = Read-Effect $state.EndpointGuid $true
$current = [string]$fx.GetValue($propertyName)
if ($current -ne $apoClsid -and $current -ne $state.OriginalEffect) {
    throw "The endpoint now uses $current; refusing to overwrite a later change."
}
$originalKind = [Microsoft.Win32.RegistryValueKind]([Enum]::Parse(
    [Microsoft.Win32.RegistryValueKind], [string]$state.OriginalKind))
if ($current -ne $state.OriginalEffect) {
    $fx.SetValue($propertyName, [string]$state.OriginalEffect, $originalKind)
}
if (Test-ApoRegistered) { Invoke-ApoRegistration $state.InstalledDll $true }
if ($state.GuardWasEnabled -and (Get-ScheduledTask -TaskName $guardTaskName -ErrorAction SilentlyContinue)) {
    Enable-ScheduledTask -TaskName $guardTaskName | Out-Null
}
Remove-Item -LiteralPath $statePath
Write-Host "Restored $($state.OriginalEffect) on $($state.EndpointGuid)."
Write-Host 'Close and reopen your microphone app; restart Windows if it still uses the previous chain.'
