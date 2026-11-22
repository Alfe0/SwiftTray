# SwiftTray MSIX clean build for Microsoft Store submission.
# Does NOT edit source files, install certificates, or publish anything.
# Run from packaging/msix or from repo root; Output folder is gitignored.
[CmdletBinding()]
param(
    [string]$RepoRoot = '',
    [string]$OutputDir = '',
    [switch]$SkipCargo
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([string]::IsNullOrWhiteSpace($RepoRoot)) { $RepoRoot = Join-Path $PSScriptRoot '..\..' }
if ([string]::IsNullOrWhiteSpace($OutputDir)) { $OutputDir = Join-Path $PSScriptRoot 'Output' }


function Find-SdkTool([string]$Tool) {
    $kitsBase = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (-not (Test-Path -LiteralPath $kitsBase)) { throw 'Windows SDK was not found. Install Windows 10/11 SDK.' }
    $candidates = Get-ChildItem -LiteralPath $kitsBase -Directory |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
        Sort-Object { [version]$_.Name } -Descending
    foreach ($folder in $candidates) {
        $path = Join-Path $folder.FullName ('x64\' + $Tool)
        if (Test-Path -LiteralPath $path) { return $path }
    }
    throw "Windows SDK tool '$Tool' not found. Install Windows SDK (x64 tooling)."
}
function Run-Checked([string]$File, [string[]]$ToolArgs) {
    & $File @ToolArgs
    if ($LASTEXITCODE -ne 0) { throw "$File failed with exit code $LASTEXITCODE" }
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$manifestSrc = Join-Path $PSScriptRoot 'AppxManifest.xml'
$sourceAssets = Join-Path $RepoRoot 'packaging\msix\Assets'
$tomlPath = Join-Path $RepoRoot 'Cargo.toml'
if (-not (Test-Path -LiteralPath $tomlPath)) { throw "Cargo.toml not found: $tomlPath" }
if (-not (Test-Path -LiteralPath $sourceAssets)) { throw "MSIX icon assets not found: $sourceAssets" }
if (-not (Test-Path -LiteralPath $manifestSrc)) { throw 'AppxManifest.xml missing from builder folder.' }

# Use Cargo.toml as the version source for repeatable future releases.
# Manifest version is generated as major.minor.patch.0.
$toml = Get-Content -LiteralPath $tomlPath -Raw
$found = [regex]::Match($toml, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"')
if (-not $found.Success) { throw 'Could not read Cargo package version.' }
$appVersion = $found.Groups[1].Value
$packageVersion = "$appVersion.0"
$lockPath = Join-Path $RepoRoot 'Cargo.lock'
if (-not (Test-Path -LiteralPath $lockPath)) { throw 'Cargo.lock missing.' }
$lockText = Get-Content -LiteralPath $lockPath -Raw
$lockEntry = [regex]::Match($lockText, '(?s)\[\[package\]\]\s*name\s*=\s*"swifttray"\s*version\s*=\s*"([^"\r\n]+)"')
if (-not $lockEntry.Success -or $lockEntry.Groups[1].Value -ne $appVersion) {
    throw 'Cargo.lock must contain the same SwiftTray version as Cargo.toml. Run cargo update --workspace.'
}

[xml]$manifest = Get-Content -LiteralPath $manifestSrc -Raw
$ns = New-Object System.Xml.XmlNamespaceManager($manifest.NameTable)
$ns.AddNamespace('m','http://schemas.microsoft.com/appx/manifest/foundation/windows10')
$identity = $manifest.SelectSingleNode('//m:Identity', $ns)
if ($identity.GetAttribute('Name') -ne 'MohammedAlfehaid.SwiftTray' -or
    $identity.GetAttribute('Publisher') -ne 'CN=AF51EB05-4F62-4CEA-A009-91A47BA6CFD5' -or
    $identity.GetAttribute('ProcessorArchitecture') -ne 'x64') { throw 'Store identity or version does not match approved plan.' }
$app = $manifest.SelectSingleNode('//m:Application', $ns)
if ($app.GetAttribute('Id') -ne 'SWIFTTRAY') { throw 'Application Id differs from original Store package.' }
$identity.SetAttribute('Version', $packageVersion)

$makeappx = Find-SdkTool 'makeappx.exe'
$makepri = Find-SdkTool 'makepri.exe'
if (-not (Test-Path -LiteralPath $OutputDir)) { New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null }
$stage = Join-Path $OutputDir 'stage'
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
$assetsDst = Join-Path $stage 'Assets'
New-Item -ItemType Directory -Force -Path $assetsDst | Out-Null
Copy-Item -Path (Join-Path $sourceAssets '*.png') -Destination $assetsDst -Force
foreach ($required in @('StoreLogo.png','Square44x44Logo.png','Square150x150Logo.png', 'Square44x44Logo.targetsize-16_altform-unplated.png')) {
    if (-not (Test-Path -LiteralPath (Join-Path $assetsDst $required))) { throw "Missing MSIX icon file: $required" }
}
$manifest.Save((Join-Path $stage 'AppxManifest.xml'))

# Local builds run tests and build the exe. CI has already done both, so it may use -SkipCargo.
if (-not $SkipCargo) {
    $cargo = Get-Command cargo.exe -ErrorAction SilentlyContinue
    if (-not $cargo) { throw 'Cargo not found. Install Rust for Windows and MSVC Build Tools.' }
    $oldFlags = $env:RUSTFLAGS
    $oldRepo = $env:SWIFTTRAY_GITHUB_REPO
    try {
        $env:RUSTFLAGS = '-C target-feature=+crt-static'
        # This ensures local Store builds have the same user-initiated GitHub update source
        # as official GitHub Action builds (which set it at the workflow level).
        $env:SWIFTTRAY_GITHUB_REPO = 'Alfe0/SwiftTray'
        Push-Location $RepoRoot
        try {
            Run-Checked $cargo.Source @('test','--release','--locked','--target','x86_64-pc-windows-msvc')
            Run-Checked $cargo.Source @('build','--release','--locked','--target','x86_64-pc-windows-msvc')
        } finally { Pop-Location }
    } finally {
        $env:RUSTFLAGS = $oldFlags
        $env:SWIFTTRAY_GITHUB_REPO = $oldRepo
    }
}
$compiledExe = Join-Path $RepoRoot 'target\x86_64-pc-windows-msvc\release\swifttray.exe'
if (-not (Test-Path -LiteralPath $compiledExe)) { throw "Built EXE not found: $compiledExe" }
$targetExeDir = Join-Path $stage 'VFS\ProgramFilesX64\SwiftTray'
New-Item -ItemType Directory -Force -Path $targetExeDir | Out-Null
Copy-Item -LiteralPath $compiledExe -Destination (Join-Path $targetExeDir 'SwiftTray.exe') -Force

$cfg = Join-Path $stage 'priconfig.xml'
Run-Checked $makepri @('createconfig','/cf',$cfg,'/dq','en-US','/o')
Run-Checked $makepri @('new','/pr',$stage,'/cf',$cfg,'/of',(Join-Path $stage 'resources.pri'),'/o')
Remove-Item -LiteralPath $cfg -Force
$msixPath = Join-Path $OutputDir ("MohammedAlfehaid.SwiftTray_{0}_x64.msix" -f $packageVersion)
if (Test-Path -LiteralPath $msixPath) { Remove-Item -LiteralPath $msixPath -Force }
Run-Checked $makeappx @('pack','/d',$stage,'/p',$msixPath,'/o')
Write-Host '---------------------------' -ForegroundColor Green
Write-Host 'MSIX created (UNSIGNED):' -ForegroundColor Green
Write-Host $msixPath
Get-FileHash -LiteralPath $msixPath -Algorithm SHA256 | Format-List
Write-Host 'For Microsoft Store submission only. Do not attempt to sideload this unsigned file directly.' -ForegroundColor Yellow
Write-Host 'Confirm in Partner Center only after certification tests and release review.' -ForegroundColor Yellow
