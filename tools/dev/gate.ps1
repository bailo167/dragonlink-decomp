<#
.SYNOPSIS
    The Windows pre-merge gate. Nothing merges to main unless this passes on the Windows host.

.DESCRIPTION
    Runs every check in order, one line per step, with each step's full output kept in
    target\gate\<step>.log and a machine-readable summary in target\gate\summary.json.
    Every step runs even after a failure, so one run shows everything that is wrong.
    Exit code 0 only if every step that ran passed.

    Steps: fmt, clippy, test, hygiene, progress, app (debug build), smoke (start the app,
    run a fixed number of frames, exit), wasm (web target build), release (with -Release).

    Works in Windows PowerShell 5.1 and PowerShell 7. See docs/BUILDING.md.

.PARAMETER Release
    Also build the app in release mode.

.PARAMETER SkipWasm
    Skip the wasm32-unknown-unknown build.

.PARAMETER SkipSmoke
    Skip the smoke launch (for sessions without a desktop or GPU).

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\gate.ps1 -Release
#>
[CmdletBinding()]
param(
    [switch]$Release,
    [switch]$SkipWasm,
    [switch]$SkipSmoke
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $repo

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) {
    $env:Path = "$cargoBin;$env:Path"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host 'gate: cargo not found. Run tools\dev\setup-windows.ps1 first.'
    exit 2
}

$logDir = Join-Path $repo 'target\gate'
New-Item -ItemType Directory -Force $logDir | Out-Null
$results = New-Object System.Collections.Generic.List[object]

function Invoke-Step {
    param([string]$Name, [string]$CommandLine)
    $log = Join-Path $logDir "$Name.log"
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    # cmd does the redirection: PowerShell 5.1 turns a native program's stderr into error records.
    & cmd.exe /d /c "$CommandLine > `"$log`" 2>&1"
    $code = $LASTEXITCODE
    $watch.Stop()
    $seconds = [math]::Round($watch.Elapsed.TotalSeconds, 1)
    $status = if ($code -eq 0) { 'pass' } else { 'FAIL' }
    Write-Host ('gate: {0,-9} {1}  {2,7}s  {3}' -f $Name, $status, $seconds, $CommandLine)
    $results.Add([pscustomobject]@{ step = $Name; status = $status; exit_code = $code; seconds = $seconds; command = $CommandLine })
}

function Skip-Step {
    param([string]$Name, [string]$Reason)
    Write-Host ('gate: {0,-9} skip            {1}' -f $Name, $Reason)
    $results.Add([pscustomobject]@{ step = $Name; status = 'skip'; exit_code = $null; seconds = 0; command = $Reason })
}

Invoke-Step 'fmt'      'cargo fmt --all --check'
Invoke-Step 'clippy'   'cargo clippy --workspace --all-targets --locked -- -D warnings'
Invoke-Step 'test'     'cargo test --workspace --locked'
Invoke-Step 'hygiene'  'cargo run --locked -q -p repo-hygiene'
Invoke-Step 'progress' 'cargo run --locked -q -p progress-gen -- --check'

if (Test-Path (Join-Path $repo 'apps\dragonlink\Cargo.toml')) {
    Invoke-Step 'app' 'cargo build --locked -p dragonlink'
    if ($SkipSmoke) {
        Skip-Step 'smoke' '-SkipSmoke given'
    } else {
        Invoke-Step 'smoke' 'target\debug\dragonlink.exe --smoke-frames 180'
    }
    if ($SkipWasm) {
        Skip-Step 'wasm' '-SkipWasm given'
    } else {
        Invoke-Step 'wasm' 'cargo build --locked -p dragonlink --target wasm32-unknown-unknown'
    }
    if ($Release) {
        Invoke-Step 'release' 'cargo build --locked --release -p dragonlink'
    } else {
        Skip-Step 'release' 'pass -Release to build the release executable'
    }
} else {
    Skip-Step 'app' 'apps\dragonlink does not exist yet'
}

$failed = @($results | Where-Object { $_.status -eq 'FAIL' })
# Through cmd for the same reason as the steps; a repository without commits has no HEAD.
$commit = (& cmd.exe /d /c 'git rev-parse --short HEAD 2>nul')
if ($LASTEXITCODE -ne 0) { $commit = 'none' }
$dirty = [bool](& cmd.exe /d /c 'git status --porcelain 2>nul')
$summary = [pscustomobject]@{
    schema    = 'dragonlink-gate/1'
    passed    = ($failed.Count -eq 0)
    commit    = "$commit"
    dirty     = $dirty
    finished  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    os        = [System.Environment]::OSVersion.VersionString
    rustc     = "$(& rustc --version)"
    steps     = $results
}
$summary | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 (Join-Path $logDir 'summary.json')

if ($failed.Count -eq 0) {
    Write-Host "gate: PASSED ($($results.Count) steps; logs in target\gate)"
    exit 0
}
Write-Host "gate: FAILED: $(($failed | ForEach-Object { $_.step }) -join ', ') (see target\gate\<step>.log)"
exit 1
