# Embedded in the official portable EXE. Do not run manually.
# Only GitHub's published stable EXE and checksum are downloaded; no remote scripts run.
[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$TargetExe,
  [Parameter(Mandatory=$true)][int]$AppPid,
  [Parameter(Mandatory=$true)][long]$AppWindow,
  [Parameter(Mandatory=$true)][string]$Tag,
  [Parameter(Mandatory=$true)][string]$ErrorFile
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue' # Required for reasonable WinPS 5.1 downloads.
$mutex = $null
$ownsMutex = $false
$cache = $null
$staged = $null
$backup = $null
$targetDir = $null
$backupReady = $false
$shutdownRequested = $false
$newProcess = $null
$failure = $null
try {
  # Cross-process protection: a second updater must not touch the same EXE.
  $mutex = [System.Threading.Mutex]::new($false, 'Local\SwiftTrayVerifiedGithubUpdater')
  try { $ownsMutex = $mutex.WaitOne(0) }
  catch [System.Threading.AbandonedMutexException] { $ownsMutex = $true }
  if (-not $ownsMutex) { throw 'Another SwiftTray update is already in progress' }

  if ($Tag -cnotmatch '^v?[0-9]+\.[0-9]+\.[0-9]+$') { throw 'Unexpected release tag' }
  $TargetExe = [IO.Path]::GetFullPath($TargetExe)
  if (-not (Test-Path -LiteralPath $TargetExe -PathType Leaf)) { throw 'Installed executable missing' }
  $targetDir = Split-Path -Parent $TargetExe
  if (-not (Test-Path -LiteralPath $targetDir -PathType Container)) { throw 'Installed folder is missing' }
  # TLS 1.2 remains available even on older Windows 10 PowerShell 5.1 setups.
  [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
  $cache = Join-Path $env:LOCALAPPDATA ('SwiftTray\updates\' + [guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $cache -Force | Out-Null
  $assetRoot = 'https://github.com/Alfe0/SwiftTray/releases/download/' + $Tag + '/'
  $download = Join-Path $cache 'SwiftTray.exe'
  $checksum = Join-Path $cache 'SwiftTray.sha256'
  Invoke-WebRequest -Uri ($assetRoot+'SwiftTray.exe') -UseBasicParsing -OutFile $download -TimeoutSec 60 -MaximumRedirection 5
  Invoke-WebRequest -Uri ($assetRoot+'SwiftTray.sha256') -UseBasicParsing -OutFile $checksum -TimeoutSec 30 -MaximumRedirection 5
  $shaText = Get-Content -LiteralPath $checksum -Raw
  if ($shaText -notmatch '(?im)^([0-9a-f]{64})\s+\*?SwiftTray\.exe\s*$') { throw 'The release checksum format is invalid' }
  $expected = $Matches[1]
  $actual = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash
  if ($actual -ine $expected) { throw 'SHA256 mismatch: update cancelled' }
  if ((Get-Item -LiteralPath $download).Length -lt 4096) { throw 'The downloaded executable is incomplete' }

  # Stage on the target volume before terminating SwiftTray.
  $staged = Join-Path $targetDir ('SwiftTray-update-' + [guid]::NewGuid().ToString('N') + '.exe')
  Copy-Item -LiteralPath $download -Destination $staged
  if ((Get-FileHash -LiteralPath $staged -Algorithm SHA256).Hash -ine $expected) { throw 'Staged update verification failed' }

  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SwiftTrayUpdater {
  [DllImport("user32.dll", SetLastError=true)] public static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);
}
'@
  if (-not [SwiftTrayUpdater]::PostMessage([IntPtr]$AppWindow, [uint32](0x8000 + 42), [IntPtr]::Zero, [IntPtr]::Zero)) {
    throw 'Could not ask SwiftTray to close safely'
  }
  $shutdownRequested = $true
  for ($i=0; $i -lt 80; $i++) {
    if (-not (Get-Process -Id $AppPid -ErrorAction SilentlyContinue)) { break }
    Start-Sleep -Milliseconds 250
  }
  if (Get-Process -Id $AppPid -ErrorAction SilentlyContinue) { throw 'SwiftTray did not close; no update was installed' }

  $backup = Join-Path $targetDir ('SwiftTray-backup-' + [guid]::NewGuid().ToString('N') + '.exe')
  Move-Item -LiteralPath $TargetExe -Destination $backup
  $backupReady = $true
  Move-Item -LiteralPath $staged -Destination $TargetExe
  $newProcess = Start-Process -FilePath $TargetExe -PassThru
  # If the new EXE terminates immediately, restore the known-good one.
  if ($newProcess.WaitForExit(4000)) { throw 'New version exited immediately; rolling back' }

  # Retain the latest rollback EXE; remove only updater-named older backups.
  Get-ChildItem -LiteralPath $targetDir -File -Filter 'SwiftTray-backup-*.exe' -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -ne $backup -and $_.Name -cmatch '^SwiftTray-backup-[0-9a-f]{32}\.exe$' } |
    Remove-Item -Force -ErrorAction SilentlyContinue
} catch {
  $failure = $_.Exception.Message
  if ($backupReady) {
    try {
      if ($newProcess -and -not $newProcess.HasExited) {
        throw 'The new version is still running; automatic rollback is unsafe'
      }
      if (Test-Path -LiteralPath $TargetExe) { Remove-Item -LiteralPath $TargetExe -Force }
      Move-Item -LiteralPath $backup -Destination $TargetExe
      $backupReady = $false
    } catch {
      $failure += '; rollback needs manual attention: ' + $_.Exception.Message
    }
  }
  # Reopen the old EXE if the updater already closed it (including when
  # moving the old EXE to backup failed). Do not launch duplicate processes.
  if ($shutdownRequested -and (Test-Path -LiteralPath $TargetExe) -and
      -not (Get-Process -Id $AppPid -ErrorAction SilentlyContinue) -and
      (-not $newProcess -or $newProcess.HasExited)) {
    try { Start-Process -FilePath $TargetExe | Out-Null }
    catch { $failure += '; restart failed: ' + $_.Exception.Message }
  }
  $message = 'SwiftTray update failed: ' + $failure
  try {
    $errorDir = Split-Path -Parent $ErrorFile
    if (-not (Test-Path -LiteralPath $errorDir)) { New-Item -ItemType Directory -Path $errorDir -Force | Out-Null }
    $message | Set-Content -LiteralPath $ErrorFile -Encoding UTF8
  } catch { }
  try {
    $root = Join-Path $env:LOCALAPPDATA 'SwiftTray'
    New-Item -ItemType Directory -Path $root -Force | Out-Null
    $message | Set-Content -LiteralPath (Join-Path $root 'update-error.txt') -Encoding UTF8
  } catch { }
  exit 1
} finally {
  if ($staged -and (Test-Path -LiteralPath $staged)) {
    Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
  }
  if ($cache -and (Test-Path -LiteralPath $cache)) {
    Remove-Item -LiteralPath $cache -Recurse -Force -ErrorAction SilentlyContinue
  }
  if ($ownsMutex -and $mutex) { try { $mutex.ReleaseMutex() } catch { } }
  if ($mutex) { $mutex.Dispose() }
  # The Rust parent also removes this specific script after the child exits.
  # Self-removal helps if the original SwiftTray process has already quit.
  if ($PSCommandPath -and $PSCommandPath -match '[\\/]apply-[0-9]+-[0-9]+\.ps1$') {
    Remove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue
  }
}
