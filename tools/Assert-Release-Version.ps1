# Fail closed on a malformed tag or mismatched app/MSIX/Cargo versions.
[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$Tag,
  [string]$RepoRoot = (Join-Path $PSScriptRoot '..')
)
$ErrorActionPreference = 'Stop'
if ($Tag -cnotmatch '^v([0-9]+)\.([0-9]+)\.([0-9]+)$') {
  throw "Stable release tags must look like v1.0.2 (got '$Tag')"
}
$expected = $Tag.Substring(1)
$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$toml = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw
$found = [regex]::Match($toml, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"')
if (-not $found.Success -or $found.Groups[1].Value -ne $expected) { throw 'Tag does not match Cargo.toml package version' }
$lock = Get-Content -LiteralPath (Join-Path $root 'Cargo.lock') -Raw
$found = [regex]::Match($lock, '(?ms)^\[\[package\]\]\s*name\s*=\s*"swifttray"\s*version\s*=\s*"([^"\r\n]+)"')
if (-not $found.Success -or $found.Groups[1].Value -ne $expected) { throw 'Tag does not match Cargo.lock swifttray package version' }
[xml]$xml = Get-Content -LiteralPath (Join-Path $root 'packaging\msix\AppxManifest.xml') -Raw
$ns = New-Object System.Xml.XmlNamespaceManager($xml.NameTable)
$ns.AddNamespace('p','http://schemas.microsoft.com/appx/manifest/foundation/windows10')
$identity = $xml.SelectSingleNode('/p:Package/p:Identity', $ns)
if (-not $identity -or $identity.GetAttribute('Version') -cne ($expected+'.0')) {
  throw 'Tag does not match MSIX Identity Version'
}
# Never change the certified identity: mismatches prevent Store updates.
if ($identity.GetAttribute('Name') -cne 'MohammedAlfehaid.SwiftTray' -or
    $identity.GetAttribute('Publisher') -cne 'CN=AF51EB05-4F62-4CEA-A009-91A47BA6CFD5') {
  throw 'Microsoft Store identity has changed'
}

# Keep the public source-version label synchronized as well.
$readme = Get-Content -LiteralPath (Join-Path $root 'README.md') -Raw
if ($readme -notmatch '(?m)\*\*Source version:\*\* `v([0-9]+\.[0-9]+\.[0-9]+)`' -or $Matches[1] -ne $expected) {
  throw 'README.md source version is not synchronized'
}

Write-Host "Version and Store identity verified for $Tag" -ForegroundColor Green
