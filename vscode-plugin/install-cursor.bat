@echo off
REM Proto-Vulcan Cursor IDE Extension Installer for Windows
REM This script installs the Proto-Vulcan syntax highlighting extension for Cursor IDE

echo Installing Proto-Vulcan Extension for Cursor IDE...

REM Get the Cursor extensions directory
set EXTENSIONS_DIR=%USERPROFILE%\.cursor\extensions

REM Create extensions directory if it doesn't exist
if not exist "%EXTENSIONS_DIR%" mkdir "%EXTENSIONS_DIR%"

REM Define the target directory
set TARGET_DIR=%EXTENSIONS_DIR%\proto-vulcan

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

echo Proto-Vulcan extension installed successfully for Cursor IDE!
echo Please restart Cursor to activate the extension.
echo.
echo Test the extension by opening any .pv file in Cursor.
echo The extension will provide syntax highlighting for Proto-Vulcan files.
pause 