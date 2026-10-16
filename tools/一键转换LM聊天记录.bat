@echo off
chcp 65001 >nul
title LM Studio Chat Converter
setlocal

set "HERE=%~dp0"
set "PS1=%HERE%lm-import.ps1"

if not exist "%PS1%" (
  echo [x] lm-import.ps1 not found.
  echo     Keep this .bat in the same folder as lm-import.ps1.
  goto :end
)

rem drag a folder onto this .bat to use it as the input path
if "%~1"=="" (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%PS1%"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%PS1%" "%~1" "%~2"
)

:end
echo.
pause
