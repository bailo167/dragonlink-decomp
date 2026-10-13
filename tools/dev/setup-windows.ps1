<#
.SYNOPSIS
    Report, and optionally install, the tooling this project uses on a Windows host.

.DESCRIPTION
    Without switches this only reports: one line per tool with the version found or MISSING.
    With -Install it installs what is missing through winget (and rustup for Rust targets).
    It never reboots, never changes power or service settings, and never touches the private
    source root. Installing Visual Studio Build Tools can ask for a reboot; if it does, the
    script says so and stops there. See docs/BUILDING.md ("Windows host").

    Tiers:
      build  Rust toolchain, MSVC build tools, Git, wasm target      (needed to build and run the gate)
      re     Java, Python, 7-Zip, ffmpeg, Android platform tools     (needed for source inventory work)
    Reverse-engineering applications (Ghidra, JADX, Apktool) are listed with where they are
    expected; they are unpacked by hand under the tools folder named in docs/BUILDING.md.

.PARAMETER Install
    Install missing tools of the selected tiers.

.PARAMETER Tier
    'build' (default) or 'all'.
#>
[CmdletBinding()]
param(
    [switch]$Install,
    [ValidateSet('build', 'all')][string]$Tier = 'build'
)

$ErrorActionPreference = 'Stop'
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) { $env:Path = "$cargoBin;$env:Path" }

function Get-ToolVersion {
    param([string]$Command, [string[]]$VersionArgs = @('--version'))
    $found = Get-Command $Command -ErrorAction SilentlyContinue
    if (-not $found) { return $null }
    try {
        $out = & cmd.exe /d /c "`"$($found.Source)`" $($VersionArgs -join ' ') 2>&1" | Select-Object -First 1
        return "$out".Trim()
    } catch { return 'present (version unknown)' }
}

function Test-MsvcTools {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { return $null }
    $path = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationVersion | Select-Object -First 1
    if ($path) { return "MSVC build tools $path" }
    return $null
}

$tools = @(
    @{ Name = 'git';     Tier = 'build'; Probe = { Get-ToolVersion 'git' };    Winget = 'Git.Git' }
    @{ Name = 'rustup';  Tier = 'build'; Probe = { Get-ToolVersion 'rustup' }; Winget = 'Rustlang.Rustup' }
    @{ Name = 'cargo';   Tier = 'build'; Probe = { Get-ToolVersion 'cargo' };  Winget = $null }
    @{ Name = 'msvc';    Tier = 'build'; Probe = { Test-MsvcTools };           Winget = 'Microsoft.VisualStudio.2022.BuildTools' }
    @{ Name = 'gh';      Tier = 'build'; Probe = { Get-ToolVersion 'gh' };     Winget = 'GitHub.cli' }
    @{ Name = 'java';    Tier = 'all';   Probe = { Get-ToolVersion 'java' @('-version') }; Winget = 'EclipseAdoptium.Temurin.21.JDK' }
    @{ Name = 'python';  Tier = 'all';   Probe = { Get-ToolVersion 'python' }; Winget = 'Python.Python.3.13' }
    @{ Name = '7z';      Tier = 'all';   Probe = { Get-ToolVersion '7z' @('i') }; Winget = '7zip.7zip' }
    @{ Name = 'ffmpeg';  Tier = 'all';   Probe = { Get-ToolVersion 'ffmpeg' @('-version') }; Winget = 'Gyan.FFmpeg' }
    @{ Name = 'adb';     Tier = 'all';   Probe = { Get-ToolVersion 'adb' };    Winget = 'Google.PlatformTools' }
)

$missing = @()
foreach ($t in $tools) {
    if ($Tier -eq 'build' -and $t.Tier -ne 'build') { continue }
    $version = & $t.Probe
    if ($version) {
        Write-Host ('setup: {0,-8} {1}' -f $t.Name, $version)
    } else {
        Write-Host ('setup: {0,-8} MISSING' -f $t.Name)
        $missing += $t
    }
}

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $targets = & rustup target list --installed
    $hasWasm = $targets -contains 'wasm32-unknown-unknown'
    Write-Host ('setup: {0,-8} {1}' -f 'wasm32', $(if ($hasWasm) { 'target installed' } else { 'MISSING' }))
    if (-not $hasWasm -and $Install) { & rustup target add wasm32-unknown-unknown }
}

if (-not $Install) {
    if ($missing.Count -gt 0) { Write-Host "setup: $($missing.Count) tool(s) missing; run again with -Install to install them." }
    exit 0
}

foreach ($t in $missing) {
    if (-not $t.Winget) { Write-Host "setup: $($t.Name) comes with another tool (rustup installs cargo); skipping."; continue }
    Write-Host "setup: installing $($t.Name) ($($t.Winget)) through winget"
    $extra = @()
    if ($t.Name -eq 'msvc') {
        $extra = @('--override', '--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
    }
    & winget install --id $t.Winget --exact --accept-package-agreements --accept-source-agreements --disable-interactivity @extra
    if ($LASTEXITCODE -eq 3010) {
        Write-Host "setup: $($t.Name) installed but Windows asks for a reboot. Not rebooting. Reboot when convenient, then run this script again."
        exit 3010
    }
}
Write-Host 'setup: done. Open a new terminal so PATH changes take effect, then run tools\dev\gate.ps1.'
