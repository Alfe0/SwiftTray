# Read-only report. Run on your main Windows PC and inside the Windows 10 VM.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$ExePath)
$ErrorActionPreference='Stop'
$exe = (Resolve-Path -LiteralPath $ExePath).Path
$os = Get-CimInstance Win32_OperatingSystem
Write-Host '--- SwiftTray read-only compatibility preflight ---' -ForegroundColor Cyan
Write-Host ('Windows: ' + $os.Caption + ' / build ' + $os.BuildNumber + ' / ' + $os.OSArchitecture)
Write-Host ('PowerShell: ' + $PSVersionTable.PSVersion.ToString())
Write-Host ('EXE: ' + $exe)
Write-Host ('SHA256: ' + (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash)
Write-Host ('File size (bytes): ' + (Get-Item -LiteralPath $exe).Length)
try {
  $devices = @(Get-PnpDevice -Class AudioEndpoint -PresentOnly -ErrorAction Stop)
  Write-Host ('Audio endpoints visible to this OS: ' + $devices.Count)
  foreach ($device in $devices) { Write-Host ('  - '+$device.FriendlyName) }
} catch { Write-Host 'Audio endpoint list not available from Get-PnpDevice on this system.' }
Write-Host 'This script does NOT install, start, update, or change SwiftTray.' -ForegroundColor Green
