@echo off
setlocal
cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo.exe >nul 2>nul
if errorlevel 1 (
    echo.
    echo Rust is not installed yet.
    echo Please visit https://rustup.rs and install Rust for Windows.
    echo Also install Visual Studio Build Tools with Desktop development with C++.
    echo Then double-click this BUILD.bat again.
    echo.
    pause
    exit /b 1
)
echo.
echo Building SwiftTray v1.0.0 release candidate (portable Windows Release)...
echo This requires an internet connection the first time.
echo.
rem Embed the Microsoft C runtime into the executable for portability.
set "RUSTFLAGS=-C target-feature=+crt-static"
cargo build --release
if errorlevel 1 (
    echo.
    echo The build failed. Copy the red error text or send a screenshot.
    echo Do not try to run an old version until the build succeeds.
    pause
    exit /b 1
)
if not exist dist mkdir dist
copy /Y "target\release\swifttray.exe" "dist\SwiftTray.exe" >nul
if errorlevel 1 (
    echo Could not copy the executable.
    pause
    exit /b 1
)
echo.
echo SUCCESS!
echo Your program is here:
echo %~dp0dist\SwiftTray.exe
echo.
echo Double-click SwiftTray.exe and look for its icon in the system tray.
echo Accent mode: SwiftTray brand gradient or Windows system accent.
echo First-run shortcuts: Alt+M mixer, Alt+O output, Alt+S screen snip.
echo Existing custom shortcuts are preserved.
echo.
pause
