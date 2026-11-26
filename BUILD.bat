@echo off
setlocal
cd /d "%~dp0"
echo Building SwiftTray release candidate (not publishing)...
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\Build-Release-Candidate.ps1"
if errorlevel 1 (
  echo BUILD FAILED. Review the error above.
  pause
  exit /b 1
)
echo Candidate EXE and checksum are under dist\Release-Candidate\.
pause
