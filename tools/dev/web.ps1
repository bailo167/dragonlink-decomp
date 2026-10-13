<#
.SYNOPSIS
    Build the web bundle of the application, and optionally check or serve it.

.DESCRIPTION
    Builds apps/dragonlink for wasm32-unknown-unknown with the profile in
    web\cargo-web.toml, generates the JavaScript glue with wasm-bindgen, and puts
    the page beside it in web\dist\. That folder is build output and is never
    committed. The bundle is the placeholder shell: generated shapes and the
    synthetic development fixture, with no original assets in it.

    -Check then loads the bundle in a headless Chromium-family browser and
    passes only if the application starts, draws its placeholder banner, and a
    press of Space changes the picture (tools\dev\web.mjs; results in
    target\web\). -Serve serves the bundle on localhost until Ctrl+C.

    Needs, beside the Rust toolchain: the wasm-bindgen command of exactly the
    version of the wasm-bindgen crate in Cargo.lock, and for -Check and -Serve
    Node.js 22 or later. See docs/BUILDING.md.

    Works in Windows PowerShell 5.1 and PowerShell 7.

.PARAMETER Check
    Check the bundle in a headless browser after building it.

.PARAMETER Serve
    Serve the bundle on http://127.0.0.1:<Port>/ after building it.

.PARAMETER Port
    Port for -Serve. Default 8080.

.PARAMETER NoBuild
    Use the bundle already in web\dist\.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\dev\web.ps1 -Check
#>
[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$Serve,
    [int]$Port = 8080,
    [switch]$NoBuild
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $repo

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) {
    $env:Path = "$cargoBin;$env:Path"
}

function Invoke-Native {
    param([string]$What, [string]$CommandLine)
    Write-Host "web: $What"
    # cmd runs it: PowerShell 5.1 turns a native program's stderr into error records.
    & cmd.exe /d /c $CommandLine
    if ($LASTEXITCODE -ne 0) {
        Write-Host "web: FAILED ($What, exit code $LASTEXITCODE)"
        exit 1
    }
}

function Require-Command {
    param([string]$Name, [string]$Hint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Host "web: $Name not found. $Hint"
        exit 2
    }
}

$dist = Join-Path $repo 'web\dist'

if (-not $NoBuild) {
    Require-Command 'cargo' 'Run tools\dev\setup-windows.ps1 first.'

    # The generated glue and the compiled module must come from the same
    # wasm-bindgen version, or the page fails when it loads.
    $lock = Get-Content (Join-Path $repo 'Cargo.lock') -Raw
    if ($lock -notmatch '(?m)^name = "wasm-bindgen"\r?\nversion = "([^"]+)"') {
        Write-Host 'web: Cargo.lock does not name a wasm-bindgen version.'
        exit 2
    }
    $wanted = $Matches[1]
    Require-Command 'wasm-bindgen' "Install it with: cargo install wasm-bindgen-cli --version $wanted --locked"
    $have = ((& cmd.exe /d /c 'wasm-bindgen --version') -replace '^wasm-bindgen\s+', '').Trim()
    if ($have -ne $wanted) {
        Write-Host "web: wasm-bindgen is $have; Cargo.lock needs $wanted."
        Write-Host "web: install it with: cargo install wasm-bindgen-cli --version $wanted --locked"
        exit 2
    }

    Invoke-Native 'building the application for wasm32-unknown-unknown' `
        'cargo build --locked -p dragonlink --target wasm32-unknown-unknown --config web/cargo-web.toml --profile web'

    if (Test-Path $dist) {
        Remove-Item -Recurse -Force $dist
    }
    New-Item -ItemType Directory -Force $dist | Out-Null
    Invoke-Native "generating the JavaScript glue (wasm-bindgen $wanted)" `
        'wasm-bindgen --target web --no-typescript --out-dir web/dist --out-name dragonlink target/wasm32-unknown-unknown/web/dragonlink.wasm'
    Copy-Item (Join-Path $repo 'web\index.html') (Join-Path $dist 'index.html') -Force

    $megabytes = [math]::Round((Get-Item (Join-Path $dist 'dragonlink_bg.wasm')).Length / 1MB, 1)
    Write-Host "web: bundle in web\dist ($megabytes MB of WebAssembly)"
}

if ($Check -or $Serve) {
    Require-Command 'node' 'Install Node.js 22 or later (docs/BUILDING.md).'
}
if ($Check) {
    Invoke-Native 'checking the bundle in a headless browser' 'node tools/dev/web.mjs check --dir web/dist --out target/web'
}
if ($Serve) {
    Invoke-Native "serving the bundle" "node tools/dev/web.mjs serve --dir web/dist --port $Port"
}
exit 0
