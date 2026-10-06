@echo off
rem Builds the release binary and packages it into dist\windows\ (needs the latest Rust stable).
cd /d "%~dp0\.."
cargo build --locked --release || exit /b 1
if exist dist\windows rmdir /s /q dist\windows
mkdir dist\windows
copy target\release\openxplane.exe dist\windows\ >nul
copy README.md dist\windows\ >nul
copy LICENSE dist\windows\ >nul
xcopy examples dist\windows\examples\ /E /I /Q >nul
echo openXplane built into dist\windows\
