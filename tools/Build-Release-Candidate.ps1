# One-command local Windows release candidate build. No publishing, tagging or signing.
# Requires Rust MSVC x64, VS C++ build tools and Windows SDK for -BuildMSIX.
[CmdletBinding()]
param([switch]$BuildMSIX)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
# Take the version from Cargo, then fail closed if README/lockfile/MSIX differ.
$toml = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw
$match = [regex]::Match($toml, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"')
if (-not $match.Success) { throw 'Cannot read Cargo package version' }
$version = $match.Groups[1].Value
& (Join-Path $PSScriptRoot 'Assert-Release-Version.ps1') -Tag ('v' + $version) -RepoRoot $root
if (-not (Get-Command cargo.exe -ErrorAction SilentlyContinue)) {
  throw 'Rust/cargo is missing. Install Rust MSVC toolchain and Visual Studio C++ Build Tools.'
}
$oldFlags=$env:RUSTFLAGS
$oldRepo=$env:SWIFTTRAY_GITHUB_REPO
$oldExperimental=$env:SWIFTTRAY_EXPERIMENTAL_BUILD
$oldTarget=$env:CARGO_TARGET_DIR
try {
  $env:RUSTFLAGS='-C target-feature=+crt-static'
  $env:SWIFTTRAY_GITHUB_REPO='Alfe0/SwiftTray'
  # Critical: never compile experimental no-self-install guard into the official EXE.
  $env:SWIFTTRAY_EXPERIMENTAL_BUILD=$null
  $env:CARGO_TARGET_DIR=$null
  Push-Location $root
  try {
    if (Get-Command python -ErrorAction SilentlyContinue) {
      & python tools/verify_icon_assets.py
      if ($LASTEXITCODE -ne 0) { throw 'Icon asset verification failed' }
    }
    & cargo test --locked --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Rust release tests failed; no release generated' }
    & cargo build --locked --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed; no release generated' }
    $exe=Join-Path $root 'target\x86_64-pc-windows-msvc\release\swifttray.exe'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Compiled EXE was not found' }
    $out=Join-Path $root 'dist\Release-Candidate'
    New-Item -ItemType Directory -Path $out -Force | Out-Null
    $dest=Join-Path $out 'SwiftTray.exe'
    Copy-Item -LiteralPath $exe -Destination $dest -Force
    $hash=(Get-FileHash -LiteralPath $dest -Algorithm SHA256).Hash.ToLowerInvariant()
    # Required format expected by the embedded updater script.
    $shaLine="$hash  SwiftTray.exe"+[Environment]::NewLine
    [IO.File]::WriteAllText((Join-Path $out 'SwiftTray.sha256'),$shaLine,[Text.Encoding]::ASCII)
    Write-Host "RELEASE CANDIDATE EXE: $dest" -ForegroundColor Green
    Write-Host "SHA256: $hash" -ForegroundColor Green
    if ($BuildMSIX) {
      & (Join-Path $root 'packaging\msix\Build-MSIX.ps1') -RepoRoot $root -SkipCargo
    }
    Write-Host 'NOT PUBLISHED. Test portable updater and Store MSIX before release.' -ForegroundColor Yellow
  } finally { Pop-Location }
} finally {
  $env:RUSTFLAGS=$oldFlags
  $env:SWIFTTRAY_GITHUB_REPO=$oldRepo
  $env:SWIFTTRAY_EXPERIMENTAL_BUILD=$oldExperimental
  $env:CARGO_TARGET_DIR=$oldTarget
}
