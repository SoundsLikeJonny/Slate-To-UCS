<#
.SYNOPSIS
    Install, upgrade and uninstall the MSI on a clean machine and assert the result.

.DESCRIPTION
    Runs the three paths the release gate cares about, in order, against a real MSI:

      1. silent install, then assert the payload, the Add/Remove Programs entry and that
         the installed binary runs
      2. silent upgrade to a second, higher version, then assert the version moved and
         that there is still exactly one Add/Remove Programs entry
      3. silent uninstall, then assert the install directory and the registry keys are gone

    Every assertion names what it checked and the value it saw, so a failure in CI says
    which invariant broke rather than only that a step exited non-zero.

    This needs elevation: msiexec installs per-machine and the MSI is perMachine. It is
    written for the elevated windows-2022 runner and refuses to run unelevated rather than
    reporting a misleading pass.

.PARAMETER Msi
    The MSI to install.

.PARAMETER UpgradeMsi
    A second MSI with a higher version, used for the upgrade step. When omitted the
    upgrade step is skipped and reported as skipped.

.EXAMPLE
    pwsh packaging/windows/smoke-install.ps1 -Msi out/Slate-To-UCS-0.1.0-x64.msi `
        -UpgradeMsi out/Slate-To-UCS-0.1.1-x64.msi
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Msi,
    [string]$UpgradeMsi,
    [string]$InstallDir = (Join-Path $env:ProgramFiles "Slate-To-UCS"),
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$checks = 0
$failures = 0

function Ok([string]$Message) {
    $script:checks++
    Write-Host "  ok   $Message"
}

function Fail([string]$Message) {
    $script:checks++
    $script:failures++
    Write-Host "  FAIL $Message" -ForegroundColor Red
}

function Assert-True([bool]$Condition, [string]$Message) {
    if ($Condition) { Ok $Message } else { Fail $Message }
}

function Assert-File([string]$Path, [string]$Label) {
    Assert-True (Test-Path -LiteralPath $Path -PathType Leaf) "exists: $Label"
}

function Assert-Absent([string]$Path, [string]$Label) {
    Assert-True (-not (Test-Path -LiteralPath $Path)) "absent: $Label"
}

function Test-Elevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-Msi([string]$Path, [string]$Arguments) {
    $log = Join-Path $env:TEMP ("slate-msi-" + [guid]::NewGuid().ToString("N") + ".log")
    $process = Start-Process -FilePath "msiexec.exe" `
        -ArgumentList "/i `"$Path`" $Arguments /l*v `"$log`" /qn /norestart" `
        -Wait -PassThru -NoNewWindow
    if ($process.ExitCode -ne 0) {
        Write-Host "msiexec exited $($process.ExitCode); log follows" -ForegroundColor Red
        if (Test-Path $log) { Get-Content $log -Tail 60 | Write-Host }
        throw "msiexec failed with exit code $($process.ExitCode)"
    }
    return $log
}

function Invoke-Uninstall([string]$ProductCode) {
    $process = Start-Process -FilePath "msiexec.exe" `
        -ArgumentList "/x $ProductCode /qn /norestart" `
        -Wait -PassThru -NoNewWindow
    if ($process.ExitCode -ne 0) { throw "msiexec /x failed with exit code $($process.ExitCode)" }
}

function Get-ArpEntries {
    $roots = @(
        "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"
    )
    $found = @()
    foreach ($root in $roots) {
        if (-not (Test-Path $root)) { continue }
        foreach ($key in Get-ChildItem $root -ErrorAction SilentlyContinue) {
            $props = Get-ItemProperty $key.PSPath -ErrorAction SilentlyContinue
            if ($props -and $props.DisplayName -eq "Slate-To-UCS") {
                $found += [pscustomobject]@{
                    Key     = $key.PSChildName
                    Version = $props.DisplayVersion
                }
            }
        }
    }
    return $found
}

if (-not (Test-Elevated)) {
    throw "this script needs elevation: the MSI is perMachine and msiexec will fail without it"
}

$Msi = (Resolve-Path -LiteralPath $Msi).Path
if (-not (Test-Path -LiteralPath $Msi)) { throw "no such MSI: $Msi" }
if ($UpgradeMsi) { $UpgradeMsi = (Resolve-Path -LiteralPath $UpgradeMsi).Path }

Write-Host "install"
$log = Invoke-Msi $Msi "ALLUSERS=1"
Assert-File (Join-Path $InstallDir "slate.exe") "slate.exe"
Assert-File (Join-Path $InstallDir "slate_reaper.dll") "slate_reaper.dll"
Assert-File (Join-Path $InstallDir "LICENSE") "LICENSE"
Assert-File (Join-Path $InstallDir "README.md") "README.md"
Assert-File (Join-Path $InstallDir "Scripts\Slate-To-UCS\lib\slate_api.lua") "slate_api.lua"
Assert-File (Join-Path $InstallDir "Scripts\Slate-To-UCS\slate_tag_selected.lua") "slate_tag_selected.lua"

$version = (& (Join-Path $InstallDir "slate.exe") --version) 2>&1 | Out-String
Assert-True ($version -match '\d+\.\d+\.\d+') "slate --version prints a version: $($version.Trim())"

$entries = @(Get-ArpEntries)
Assert-True ($entries.Count -eq 1) "exactly one Add/Remove Programs entry, saw $($entries.Count)"
if ($entries.Count -eq 1) {
    Assert-True ($entries[0].Version -match '^\d+\.\d+\.\d+') "the entry has a version: $($entries[0].Version)"
}

if ($UpgradeMsi) {
    Write-Host "upgrade"
    $before = @(Get-ArpEntries)[0].Version
    $null = Invoke-Msi $UpgradeMsi "ALLUSERS=1"
    $after = @(Get-ArpEntries)
    Assert-True ($after.Count -eq 1) "still exactly one entry after the upgrade, saw $($after.Count)"
    if ($after.Count -eq 1) {
        Assert-True ($after[0].Version -ne $before) "the version moved from $before to $($after[0].Version)"
    }
    Assert-File (Join-Path $InstallDir "slate.exe") "slate.exe after the upgrade"
} else {
    Write-Host "  skip upgrade: no -UpgradeMsi supplied"
}

Write-Host "uninstall"
$productCode = @(Get-ArpEntries)[0].Key
if (-not $productCode) { throw "could not find the product code to uninstall" }
Invoke-Uninstall $productCode

Assert-Absent (Join-Path $InstallDir "slate.exe") "slate.exe after uninstall"
Assert-Absent (Join-Path $InstallDir "Scripts") "Scripts after uninstall"
Assert-Absent $InstallDir "the install directory after uninstall"
Assert-True (@(Get-ArpEntries).Count -eq 0) "no Add/Remove Programs entry after uninstall"
Assert-True (-not (Test-Path "HKLM:\Software\Slate-To-UCS")) "no HKLM\Software\Slate-To-UCS key after uninstall"

Write-Host ""
Write-Host "$checks checks, $failures failures"
if ($failures -ne 0) { exit 1 }
