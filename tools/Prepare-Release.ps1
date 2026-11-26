# Controlled version stamping ONLY; never creates a tag, pushes, or publishes.
# Example: .\tools\Prepare-Release.ps1 -Version 1.0.2 -Apply
[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$Version,
  [switch]$Apply
)
$ErrorActionPreference='Stop'
if ($Version -cnotmatch '^[0-9]+\.[0-9]+\.[0-9]+$') { throw 'Expected a stable version such as 1.0.2' }
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$paths = @('Cargo.toml','Cargo.lock','packaging\msix\AppxManifest.xml','README.md')
foreach ($rel in $paths) {
  if (-not (Test-Path -LiteralPath (Join-Path $root $rel))) { throw "Missing $rel" }
}
$tomlPath=Join-Path $root 'Cargo.toml'
$lockPath=Join-Path $root 'Cargo.lock'
$manifestPath=Join-Path $root 'packaging\msix\AppxManifest.xml'
$readmePath=Join-Path $root 'README.md'
$toml=Get-Content -LiteralPath $tomlPath -Raw
$lock=Get-Content -LiteralPath $lockPath -Raw
$manifest=Get-Content -LiteralPath $manifestPath -Raw
$readme=Get-Content -LiteralPath $readmePath -Raw
$tomlPattern='(?m)^(version\s*=\s*")[0-9]+\.[0-9]+\.[0-9]+("\s*)$'
$lockPattern='(?ms)(^\[\[package\]\]\s*name\s*=\s*"swifttray"\s*version\s*=\s*")[0-9]+\.[0-9]+\.[0-9]+(")'
$readmePattern='(\*\*Source version:\*\* `v)[0-9]+\.[0-9]+\.[0-9]+(`)'
$manifestPattern='(<Identity\s+Name="MohammedAlfehaid.SwiftTray"\s+Publisher="CN=AF51EB05-4F62-4CEA-A009-91A47BA6CFD5"\s+Version=")[0-9]+\.[0-9]+\.[0-9]+\.0(")'
foreach ($entry in @(@($toml,$tomlPattern,'Cargo.toml'),@($lock,$lockPattern,'Cargo.lock'),@($manifest,$manifestPattern,'AppxManifest.xml'),@($readme,$readmePattern,'README.md'))) {
  if ([regex]::Matches($entry[0],$entry[1]).Count -ne 1) { throw ('Cannot safely identify exactly one version in '+$entry[2]) }
}
$replace = '${1}' + $Version + '${2}'
$newToml=[regex]::Replace($toml,$tomlPattern,$replace)
$newLock=[regex]::Replace($lock,$lockPattern,$replace)
$newManifest=[regex]::Replace($manifest,$manifestPattern,('${1}'+$Version+'.0${2}'))
$newReadme=[regex]::Replace($readme,$readmePattern,('${1}'+$Version+'${2}'))
Write-Host "Target version: $Version (Cargo, lockfile and MSIX)." -ForegroundColor Cyan
if (-not $Apply) {
  Write-Host 'PREVIEW ONLY. Re-run with -Apply to write; changelog still needs release-note review.' -ForegroundColor Yellow
  return
}
# Windows PowerShell 5.1 writes a BOM with -Encoding UTF8; Cargo files
# should remain UTF-8 without BOM for portable, repeatable builds.
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($tomlPath,$newToml,$utf8NoBom)
[System.IO.File]::WriteAllText($lockPath,$newLock,$utf8NoBom)
[System.IO.File]::WriteAllText($manifestPath,$newManifest,$utf8NoBom)
[System.IO.File]::WriteAllText($readmePath,$newReadme,$utf8NoBom)
& (Join-Path $PSScriptRoot 'Assert-Release-Version.ps1') -Tag ("v"+$Version) -RepoRoot $root
Write-Host 'Version files updated. Review docs, test and package before publishing.' -ForegroundColor Green
