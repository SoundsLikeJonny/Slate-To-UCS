<#
.SYNOPSIS
    Builds the Slate-To-UCS NSIS installer.

.DESCRIPTION
    Compiles packaging/windows/slate.nsi with makensis. Every input path is passed with
    /D, so the .nsi file holds no absolute path.

    The MSI is the primary artefact. This exists because an .exe installer is what many
    recordists expect to double-click, and because it can be smoke-tested without an MSI
    service round trip.

.PARAMETER Version
    The product version. Defaults to version.toml, the single source of truth.

.EXAMPLE
    pwsh packaging/windows/build-nsis.ps1
#>
[CmdletBinding()]
param(
    [string]$Version,
    [string]$SlateExe = "target/release/slate.exe",
    [string]$ReaperDll = "apps/reaper/extension/build/Release/slate_reaper.dll",
    [string]$ScriptsDir = "apps/reaper/scripts",
    [string]$SidecarDir,
    [string]$IconPath = "apps/desktop/src-tauri/icons/icon.ico",
    [string]$OutputDir = "packaging/windows/out",
    [string]$Makensis = $env:MAKENSIS
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Resolve-RepoRoot {
    $dir = $PSScriptRoot
    while ($dir -and -not (Test-Path (Join-Path $dir "version.toml"))) {
        $dir = Split-Path $dir -Parent
    }
    if (-not $dir) { throw "could not find version.toml above $PSScriptRoot" }
    return $dir
}

function Get-ProductVersion([string]$Root) {
    $line = Select-String -Path (Join-Path $Root "version.toml") -Pattern '^version\s*=\s*"([^"]+)"' |
        Select-Object -First 1
    if (-not $line) { throw "version.toml has no version line" }
    return $line.Matches[0].Groups[1].Value
}

function Find-Makensis([string]$Explicit) {
    if ($Explicit) {
        if (-not (Test-Path $Explicit)) { throw "MAKENSIS is set to '$Explicit' but it does not exist" }
        return $Explicit
    }
    $candidates = @(
        (Join-Path ${env:ProgramFiles(x86)} "NSIS\makensis.exe"),
        (Join-Path $env:ProgramFiles "NSIS\makensis.exe"),
        (Join-Path $env:LOCALAPPDATA "nsis\makensis.exe")
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) { return $candidate }
    }
    throw "makensis not found. Set MAKENSIS to the full path of makensis.exe."
}

$root = Resolve-RepoRoot
Push-Location $root
try {
    if (-not $Version) { $Version = Get-ProductVersion $root }
    $nsis = Find-Makensis $Makensis
    Write-Host "version $Version, makensis at $nsis"

    foreach ($required in @($SlateExe, $ReaperDll, $IconPath, "LICENSE", "README.md")) {
        if (-not (Test-Path $required)) { throw "missing input: $required" }
    }

    $out = Join-Path $root $OutputDir
    New-Item -ItemType Directory -Force -Path $out | Out-Null

    $args = @(
        "/V2",
        "/DSLATE_EXE=$(Join-Path $root $SlateExe)",
        "/DREAPER_DLL=$(Join-Path $root $ReaperDll)",
        "/DSCRIPTS_DIR=$(Join-Path $root $ScriptsDir)",
        "/DICON_PATH=$(Join-Path $root $IconPath)",
        "/DVERSION=$Version",
        "/DOUT_FILE=$(Join-Path $out "Slate-To-UCS-$Version-x64-setup.exe")"
    )
    if ($SidecarDir) {
        $sidecar = Join-Path $root $SidecarDir
        if (-not (Test-Path $sidecar)) { throw "missing sidecar directory: $sidecar" }
        $args += "/DSIDECAR_DIR=$sidecar"
    }
    $args += (Join-Path $root "packaging/windows/slate.nsi")

    & $nsis @args
    if ($LASTEXITCODE -ne 0) { throw "makensis failed with exit code $LASTEXITCODE" }

    $exe = Join-Path $out "Slate-To-UCS-$Version-x64-setup.exe"
    if (-not (Test-Path $exe)) { throw "makensis reported success but $exe does not exist" }
    Write-Host "built $exe ($((Get-Item $exe).Length) bytes)"
    Write-Output $exe
}
finally {
    Pop-Location
}
