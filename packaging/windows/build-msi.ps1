<#
.SYNOPSIS
    Builds the Slate-To-UCS MSI.

.DESCRIPTION
    Compiles packaging/windows/slate.wxs with candle and links it with light, using the
    WiX 3.14 toolset. Every input path is passed as a preprocessor variable, so the .wxs
    file itself holds no absolute path.

    The sidecar is optional. When -SidecarDir points at a directory it is harvested with
    `heat dir` into a component group; when it does not, the MSI is built without it and
    the smoke test still runs. That keeps packaging breakage visible before the sidecar
    exists, which is the point of building unsigned MSIs from phase 1.

.PARAMETER Version
    The product version. Defaults to the value in version.toml, which is the single
    source of truth, so a hand-typed version cannot drift from the binaries.

.PARAMETER OutputDir
    Where the .msi is written. Defaults to packaging/windows/out.

.EXAMPLE
    pwsh packaging/windows/build-msi.ps1 -SlateExe target/release/slate.exe `
        -ReaperDll apps/reaper/extension/build/Release/slate_reaper.dll
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
    [string]$WixBin = $env:WIX_BIN
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

function Find-WixBin([string]$Explicit) {
    if ($Explicit) {
        if (-not (Test-Path (Join-Path $Explicit "candle.exe"))) {
            throw "WIX_BIN is set to '$Explicit' but it holds no candle.exe"
        }
        return $Explicit
    }
    $candidates = @(
        (Join-Path $env:ProgramFiles "WiX Toolset v3.14\bin"),
        (Join-Path ${env:ProgramFiles(x86)} "WiX Toolset v3.14\bin"),
        (Join-Path $env:LOCALAPPDATA "wix314")
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path (Join-Path $candidate "candle.exe"))) { return $candidate }
    }
    throw "WiX 3.14 not found. Set WIX_BIN to a directory holding candle.exe and light.exe."
}

function Invoke-Tool([string]$Exe, [string[]]$Arguments) {
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$([System.IO.Path]::GetFileName($Exe)) failed with exit code $LASTEXITCODE"
    }
}

$root = Resolve-RepoRoot
Push-Location $root
try {
    if (-not $Version) { $Version = Get-ProductVersion $root }
    $wix = Find-WixBin $WixBin
    Write-Host "version $Version, WiX at $wix"

    foreach ($required in @($SlateExe, $ReaperDll, $IconPath, "LICENSE", "README.md")) {
        if (-not (Test-Path $required)) { throw "missing input: $required" }
    }

    $out = Join-Path $root $OutputDir
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $obj = Join-Path $out "obj"
    New-Item -ItemType Directory -Force -Path $obj | Out-Null

    $sources = @((Join-Path $root "packaging/windows/slate.wxs"))

    if ($SidecarDir) {
        $sidecar = Join-Path $root $SidecarDir
        if (-not (Test-Path $sidecar)) { throw "missing sidecar directory: $sidecar" }
        $harvest = Join-Path $obj "SidecarFiles.wxs"
        Invoke-Tool (Join-Path $wix "heat.exe") @(
            "dir", $sidecar,
            "-cg", "SidecarComponents",
            "-dr", "SidecarFolder",
            "-gg", "-g1", "-srd", "-sfrag", "-sreg", "-scom",
            "-var", "var.SidecarDir",
            "-out", $harvest
        )
        $sources += $harvest
    }

    $candleArgs = @(
        "-nologo",
        "-arch", "x64",
        "-ext", "WixUIExtension",
        "-dVersion=$Version",
        "-dSlateExe=$(Join-Path $root $SlateExe)",
        "-dReaperDll=$(Join-Path $root $ReaperDll)",
        "-dScriptsDir=$(Join-Path $root $ScriptsDir)",
        "-dIconPath=$(Join-Path $root $IconPath)",
        "-dLicencePath=$(Join-Path $root 'LICENSE')",
        "-dReadmePath=$(Join-Path $root 'README.md')",
        "-dLicenceRtf=$(Join-Path $root 'packaging/windows/licence.rtf')",
        "-out", "$obj\",
        "-I$(Join-Path $root 'packaging/windows')"
    )
    if ($SidecarDir) {
        $candleArgs += "-dSidecarDir=$(Join-Path $root $SidecarDir)"
    }
    $candleArgs += $sources
    Invoke-Tool (Join-Path $wix "candle.exe") $candleArgs

    $wixobjs = Get-ChildItem -Path $obj -Filter "*.wixobj" | ForEach-Object { $_.FullName }
    if (-not $wixobjs) { throw "candle produced no .wixobj" }

    $msi = Join-Path $out "Slate-To-UCS-$Version-x64.msi"
    Invoke-Tool (Join-Path $wix "light.exe") (@(
        "-nologo",
        "-ext", "WixUIExtension",
        "-cultures:en-us",
        "-b", $root,
        "-out", $msi
    ) + $wixobjs)

    if (-not (Test-Path $msi)) { throw "light reported success but $msi does not exist" }
    $size = (Get-Item $msi).Length
    Write-Host "built $msi ($size bytes)"
    Write-Output $msi
}
finally {
    Pop-Location
}
