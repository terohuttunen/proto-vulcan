@echo off
REM Proto-Vulcan VSCode Extension Installer for Windows
REM This script installs the Proto-Vulcan syntax highlighting extension

echo Installing Proto-Vulcan VSCode Extension...

REM Get the VSCode extensions directory
set EXTENSIONS_DIR=%USERPROFILE%\.vscode\extensions

REM Create extensions directory if it doesn't exist
if not exist "%EXTENSIONS_DIR%" mkdir "%EXTENSIONS_DIR%"

REM Define the target directory
set TARGET_DIR=%EXTENSIONS_DIR%\proto-vulcan.proto-vulcan-0.1.0

REM Remove existing installation if it exists
if exist "%TARGET_DIR%" (
    echo Removing existing installation...
    rmdir /s /q "%TARGET_DIR%"
)

REM Create the target directory
mkdir "%TARGET_DIR%"

REM Copy extension files
echo Copying extension files...
copy package.json "%TARGET_DIR%\"
copy language-configuration.json "%TARGET_DIR%\"
copy README.md "%TARGET_DIR%\"
xcopy syntaxes "%TARGET_DIR%\syntaxes\" /E /I

echo Proto-Vulcan VSCode extension installed successfully!
echo Please restart VSCode to activate the extension.
echo.
echo Test the extension by opening any .pv file.
echo You can try the included example.pv file to see syntax highlighting in action.
pause 